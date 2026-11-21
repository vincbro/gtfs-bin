//! Property tests that compare the places and footpaths the compiler writes with a scan of every
//! pair of stops.

mod common;

use std::collections::BTreeMap;

use flatnet_compiler::{Compiler, Network, StopRecord};
use flatnet_core::{
    consumer::Consumer,
    models::{Coordinate, Distance, LocationType, Settings, StopIndex},
};
use proptest::{collection, option, prelude::*};

const LOCATION_TYPES: [LocationType; 5] = [
    LocationType::STOP_POINT,
    LocationType::STOP_AREA,
    LocationType::STATION_ENTRANCE,
    LocationType::GENERIC_NODE,
    LocationType::BOARDING_AREA,
];

fn stops() -> impl Strategy<Value = Vec<StopRecord>> {
    // A box of about two kilometres, so reaches from 50 m to 50 km all find some and miss some. It
    // sits in Stockholm, across the 180th meridian, on the equator or by the north pole.
    let origin = prop_oneof![
        Just((59.0_f64, 18.0_f64)),
        Just((0.0, 179.99)),
        Just((-16.0, -179.995)),
        Just((89.975, 0.0)),
    ];
    let offset = option::weighted(
        0.9,
        (
            prop_oneof![0.0_f64..0.02, 0.0_f64..0.0005],
            prop_oneof![0.0_f64..0.03, 0.0_f64..0.0005],
        ),
    );
    (
        origin,
        collection::vec(
            (option::of(0_usize..10), offset, 0..LOCATION_TYPES.len()),
            1..10,
        ),
    )
        .prop_map(|((latitude, longitude), stops)| {
            let count = stops.len();
            stops
                .into_iter()
                .enumerate()
                .map(|(index, (parent, offset, location))| StopRecord {
                    id: format!("s{index}"),
                    code: None,
                    name: None,
                    description: None,
                    latitude: offset.map(|(north, _)| latitude + north),
                    // Past 180 degrees the position continues from -180.
                    longitude: offset
                        .map(|(_, east)| (longitude + east + 180.0).rem_euclid(360.0) - 180.0),
                    location_type: LOCATION_TYPES[location],
                    parent: parent.filter(|&parent| parent < count),
                })
                .collect()
        })
}

fn can_board(location: LocationType) -> bool {
    ![
        LocationType::STOP_AREA,
        LocationType::STATION_ENTRANCE,
        LocationType::GENERIC_NODE,
    ]
    .contains(&location)
}

proptest! {

    #[test]
    fn footpaths_are_the_nearest_places_in_reach(
        stops in stops(),
        reach in prop_oneof![Just(0_u32), Just(50), Just(400), 0_u32..3000, Just(50_000)],
        per_place in prop_oneof![Just(0_u32), Just(1), Just(2), Just(5), Just(u32::MAX)],
    ) {
        let settings = Settings { footpath_max_meters: reach, footpath_max_per_place: per_place };
        let bytes = Compiler::new()
            .settings(settings)
            .compile(Network { stops, ..Network::default() })
            .expect("a network of stops compiles");
        let (_file, mmap) = common::map(&bytes).expect("the file maps");
        let consumer = Consumer::new(&mmap).expect("the compiler wrote a valid file");

        // The root of each stop, going up at most eight parents.
        let root = |start: StopIndex| {
            let mut root = start;
            for _ in 0..8 {
                match consumer.stop(root).parent_index.as_option() {
                    Some(parent) => root = parent,
                    None => break,
                }
            }
            root
        };
        let mut members: BTreeMap<StopIndex, Vec<StopIndex>> = BTreeMap::new();
        for stop in consumer.stops() {
            if can_board(stop.location_type) {
                members.entry(root(stop.index)).or_default().push(stop.index);
            }
        }

        for stop in consumer.stops() {
            prop_assert_eq!(consumer.place_of(stop.index), root(stop.index));
            let place = stop.index;
            let own = members.get(&place).cloned().unwrap_or_default();
            prop_assert_eq!(consumer.stops_by_place(place), own.as_slice(), "the stops of place {:?}", place);

            let mut expected: Vec<(StopIndex, f32)> = Vec::new();
            for (&other, others) in &members {
                if other == place || own.is_empty() {
                    continue;
                }
                let nearest = own
                    .iter()
                    .flat_map(|&from| others.iter().map(move |&to| (from, to)))
                    .filter_map(|(from, to)| {
                        let from = consumer.stop(from).coordinate.as_option()?;
                        let to = consumer.stop(to).coordinate.as_option()?;
                        let meters = Coordinate::distance_meters(from, to);
                        (meters <= f64::from(reach)).then(|| Distance::from_meters(meters).0)
                    })
                    .min_by(f32::total_cmp);
                if let Some(meters) = nearest {
                    expected.push((other, meters));
                }
            }
            expected.sort_by(|first, second| first.1.total_cmp(&second.1).then(first.0.cmp(&second.0)));
            expected.truncate(usize::try_from(per_place).unwrap_or(usize::MAX));

            let got: Vec<(StopIndex, f32)> = consumer
                .footpaths_by_place(place)
                .iter()
                .map(|footpath| (footpath.to_place_index, footpath.meters.0))
                .collect();
            prop_assert_eq!(got, expected, "the walks from place {:?} with reach {}", place, reach);
        }
    }
}
