//! Property tests that compare the consumer's queries with a plain scan of the tables.

mod common;

use common::arbitrary;
use flatnet_compiler::{Compiler, Network, StopRecord};
use flatnet_core::{
    consumer::Consumer,
    models::{Coordinate, LocationType, Parts, Sentinel, StopIndex, Time},
};
use proptest::{collection, option, prelude::*};

fn world_stops() -> impl Strategy<Value = Network> {
    let latitude = prop_oneof![
        -89.99_f64..89.99,
        85.0_f64..89.99,
        -89.99_f64..-85.0,
        -1.0_f64..1.0,
        59.0_f64..59.1
    ];
    let longitude = prop_oneof![
        -179.99_f64..179.99,
        179.0_f64..180.0,
        -180.0_f64..-179.0,
        18.0_f64..18.1
    ];
    collection::vec((latitude, longitude, any::<bool>()), 1..60).prop_map(|stops| Network {
        stops: stops
            .into_iter()
            .enumerate()
            .map(|(index, (latitude, longitude, positioned))| StopRecord {
                id: format!("s{index}"),
                code: None,
                name: None,
                description: None,
                latitude: positioned.then_some(latitude),
                longitude: positioned.then_some(longitude),
                location_type: LocationType::STOP_POINT,
                parent: None,
            })
            .collect(),
        ..Network::default()
    })
}

fn radius() -> impl Strategy<Value = f64> {
    prop_oneof![
        0.0_f64..1e3,
        0.0_f64..1e5,
        0.0_f64..2e6,
        0.0_f64..2.1e7,
        Just(1e12),
        Just(f64::MAX),
    ]
}

proptest! {

    #[test]
    fn near_stops_are_the_stops_a_scan_finds_anywhere_on_the_globe(
        network in world_stops(),
        latitude in prop_oneof![-90.0_f64..=90.0, 85.0_f64..=90.0, -90.0_f64..-85.0],
        longitude in prop_oneof![-180.0_f64..=180.0, 179.0_f64..=180.0, -180.0_f64..-179.0],
        radius in radius(),
        at_a_stop in option::of(any::<usize>()),
    ) {
        let bytes = Compiler::new().compile(network).expect("stops compile");
        let (_file, mmap) = common::map(&bytes).expect("the file maps");
        let consumer = Consumer::new(&mmap).expect("the compiler wrote a valid file");
        let (latitude, longitude) = at_a_stop
            .and_then(|pick| {
                let positioned: Vec<_> = consumer.stops().iter().filter_map(|stop| stop.coordinate.as_option()).collect();
                (!positioned.is_empty()).then(|| positioned[pick % positioned.len()].to_f64_pair())
            })
            .unwrap_or((latitude, longitude));

        let mut found = Vec::new();
        consumer.stops_near(latitude, longitude, radius, &mut found);

        let here = Coordinate::new(latitude, longitude);
        let mut expected: Vec<(StopIndex, f32)> = consumer
            .stops()
            .iter()
            .filter_map(|stop| {
                let meters = here.distance_meters(stop.coordinate.as_option()?);
                #[allow(clippy::cast_possible_truncation, reason = "the file keeps meters in an f32")]
                (meters <= radius).then_some((stop.index, meters as f32))
            })
            .collect();
        expected.sort_by(|first, second| first.1.total_cmp(&second.1).then(first.0.cmp(&second.0)));
        prop_assert_eq!(found, expected, "stops within {} m of ({}, {})", radius, latitude, longitude);
    }

    #[test]
    fn stops_in_a_box_are_the_stops_a_scan_finds_anywhere_on_the_globe(
        network in world_stops(),
        south in -90.0_f64..=90.0,
        height in prop_oneof![0.0_f64..1.0, 0.0_f64..180.0],
        west in -180.0_f64..=180.0,
        width in prop_oneof![0.0_f64..1.0, 0.0_f64..360.0],
    ) {
        let bytes = Compiler::new().compile(network).expect("stops compile");
        let (_file, mmap) = common::map(&bytes).expect("the file maps");
        let consumer = Consumer::new(&mmap).expect("the compiler wrote a valid file");
        let (north, east) = (south + height, west + width);

        let mut found = Vec::new();
        consumer.stops_in_box(south, west, north, east, &mut found);

        let expected: Vec<StopIndex> = consumer
            .stops()
            .iter()
            .filter(|stop| {
                stop.coordinate.as_option().is_some_and(|position| {
                    let (latitude, longitude) = position.to_f64_pair();
                    (south..=north).contains(&latitude) && (west..=east).contains(&longitude)
                })
            })
            .map(|stop| stop.index)
            .collect();
        prop_assert_eq!(found, expected);
    }
}

/// How well a name matches a query, by the ranks `search_by_name` documents.
fn rank(name: &str, query: &str) -> Option<u8> {
    let (name, query) = (name.to_lowercase(), query.trim().to_lowercase());
    if name == query {
        Some(0)
    } else if name.starts_with(&query) {
        Some(1)
    } else if name.split(' ').any(|word| word.starts_with(&query)) {
        Some(2)
    } else if name.contains(&query) {
        Some(3)
    } else {
        None
    }
}

proptest! {

    #[test]
    fn every_lookup_agrees_with_a_scan(
        network in arbitrary::network(),
        query in prop_oneof![Just("n".to_owned()), Just("N1".to_owned()), Just("S".to_owned()), "[a-z0-9 ]{0,3}"],
        starts in collection::vec(any::<u32>(), 0..4),
    ) {
        let Ok(bytes) = Compiler::new().parts(Parts::ALL).compile(network) else {
            return Ok(());
        };
        let (_file, mmap) = common::map(&bytes).expect("the file maps");
        let consumer = Consumer::new(&mmap).expect("the compiler wrote a valid file");

        // Ids find themselves, and an id that isn't there finds nothing.
        for stop in consumer.stops() {
            let found = consumer.stop_by_id(consumer.stop_id(stop.id));
            prop_assert_eq!(found.map(|stop| stop.index), Some(stop.index));
        }
        for trip in consumer.trips() {
            let found = consumer.trip_by_id(consumer.trip_id(trip.id));
            prop_assert_eq!(found.map(|trip| trip.index), Some(trip.index));
        }
        for route in consumer.routes() {
            let found = consumer.route_by_id(consumer.route_id(route.id));
            prop_assert_eq!(found.map(|route| route.index), Some(route.index));
        }
        for service in consumer.services() {
            let found = consumer.service_by_id(consumer.service_id(service.id));
            prop_assert_eq!(found.map(|service| service.index), Some(service.index));
        }
        prop_assert!(consumer.stop_by_id("nothing like this").is_none());
        prop_assert!(consumer.trip_by_id("").is_none());

        // The trips of a route that start at a time are the ones a linear scan finds, by index.
        let present: Vec<Time> = consumer.trips().iter().filter_map(|trip| consumer.trip_start(trip.index)).collect();
        for route in consumer.routes() {
            let route_index = route.index;
            let mut times: Vec<Time> = present.iter().copied().chain(starts.iter().map(|&seconds| Time(seconds))).collect();
            times.extend([Time(0), Time::NONE]);
            for start in times {
                let expected: Vec<_> = consumer
                    .trips()
                    .iter()
                    .filter(|trip| trip.route_index == route_index && consumer.trip_start(trip.index) == Some(start))
                    .map(|trip| trip.index)
                    .collect();
                prop_assert_eq!(consumer.trips_by_route_start(route_index, start), expected.as_slice(), "route {:?} start {:?}", route_index, start);
            }
        }

        // The trips at a stop are the trips whose pattern calls there, each once.
        for stop in consumer.stops() {
            let mut got: Vec<_> = consumer.iter_trips_by_stop(stop.index).map(|trip| trip.index).collect();
            got.sort_unstable();
            let mut expected: Vec<_> = consumer
                .trips()
                .iter()
                .filter(|trip| consumer.stop_sequence_by_trip_pattern(consumer.trip_pattern_by_trip(trip.index).index).contains(&stop.index))
                .map(|trip| trip.index)
                .collect();
            expected.sort_unstable();
            prop_assert_eq!(got, expected, "the trips at stop {:?}", stop.index);
        }

        // Names are found in the order the ranks give.
        let mut found = Vec::new();
        consumer.search_by_name(&query, &mut found);
        let mut expected: Vec<_> = (0..consumer.search_stops().len())
            .filter_map(|position| {
                let search = &consumer.search_stops()[position];
                let found = rank(consumer.string(search.name), &query)?;
                (!query.trim().is_empty()).then_some((found, search.index))
            })
            .collect();
        expected.sort_unstable();
        let expected: Vec<_> = expected.into_iter().map(|(_, index)| index).collect();
        prop_assert_eq!(found, expected, "names matching {:?}", query);
    }
}
