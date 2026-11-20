//! Finding a route's trips by the time they start, which realtime matching uses.

mod common;

use flatnet_core::{
    consumer::Consumer,
    models::{RouteIndex, Sentinel, Time, TripIndex},
};

fn open(bytes: &[u8], body: impl FnOnce(&Consumer<'_>)) {
    let (_file, mmap) = common::map(bytes).expect("a compiled file maps");
    body(&Consumer::new(&mmap).expect("the compiler wrote a valid file"));
}

#[test]
fn a_trip_starts_when_it_leaves_its_first_stop() {
    open(&common::compile_fixture(), |consumer| {
        let trip = consumer.trip_by_id("trip4").expect("the trip exists");

        assert_eq!(
            consumer.trip_start(trip.index),
            Some(Time::from_hms("09:00:00").expect("a time")),
            "trip4 leaves at nine"
        );
    });
}

/// Checks the trips found by each start of the route against a scan, and returns how many starts.
fn check_route(consumer: &Consumer<'_>, route: RouteIndex) -> usize {
    let starts: Vec<(TripIndex, Time)> = consumer
        .trips_by_route(route)
        .iter()
        .filter_map(|&trip| Some((trip, consumer.trip_start(trip)?)))
        .collect();
    for &(trip, start) in &starts {
        let found = consumer.trips_by_route_start(route, start);
        let wanted: Vec<TripIndex> = starts
            .iter()
            .filter(|&&(_, other)| other == start)
            .map(|&(other_trip, _)| other_trip)
            .collect();

        assert_eq!(found, wanted, "the same trips as a scan, in index order");
        assert!(found.contains(&trip), "the trip is found by its own start");
    }
    starts.len()
}

#[test]
fn the_trips_found_by_start_are_exactly_those_of_the_route_that_start_then() {
    open(&common::compile_fixture(), |consumer| {
        let checked: usize = (0..consumer.routes().len())
            .map(|route| check_route(consumer, RouteIndex::from(route)))
            .sum();

        assert!(checked > 0, "the fixture has trips with a start");
    });
}

#[test]
fn a_route_lists_its_trips_from_the_earliest_start() {
    open(&common::compile_fixture(), |consumer| {
        for route in 0..consumer.routes().len() {
            let starts: Vec<Option<Time>> = consumer
                .trips_by_route(RouteIndex::from(route))
                .iter()
                .map(|&trip| consumer.trip_start(trip))
                .collect();

            assert!(
                starts.windows(2).all(|pair| pair[0] <= pair[1]),
                "route {route} is out of order: {starts:?}"
            );
        }
    });
}

#[test]
fn a_start_nobody_has_finds_nothing() {
    open(&common::compile_fixture(), |consumer| {
        let route = RouteIndex::from(0_usize);

        assert!(
            consumer
                .trips_by_route_start(route, Time::from_hms("03:33:33").expect("a time"))
                .is_empty(),
            "no trip starts then"
        );
        assert!(
            consumer.trips_by_route_start(route, Time::NONE).is_empty(),
            "no start is not a start"
        );
    });
}

#[test]
fn starts_that_do_not_match_the_trips_are_rejected() {
    let bytes = common::damage_section(
        common::compile_fixture(),
        |header| header.route_trip_starts,
        |starts: &mut [Time]| starts.iter_mut().for_each(|start| start.0 += 1),
    );

    assert!(
        common::is_rejected(&bytes),
        "each start is checked against its trip"
    );
}

#[test]
fn a_route_out_of_order_is_rejected() {
    let bytes = common::damage_section(
        common::compile_fixture(),
        |header| header.route_to_trips,
        |trips: &mut [TripIndex]| trips.reverse(),
    );

    assert!(
        common::is_rejected(&bytes),
        "the trips are checked against the starts"
    );
}
