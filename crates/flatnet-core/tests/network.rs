//! Compiling a network that no GTFS feed was read for, and what the compiler refuses in one.

use flatnet_compiler::{
    Call, Compiler, Error, Network, RouteRecord, ServiceRecord, StopRecord, TripRecord, Weekly,
};
use flatnet_core::{
    consumer::Consumer,
    models::{BoardingRule, Date, Direction, LocationType, Optional, RouteType, Sentinel, Time},
};
use memmap2::Mmap;

mod common;

fn stop(id: &str, latitude: f64) -> StopRecord {
    StopRecord {
        id: id.to_owned(),
        code: None,
        name: Some(format!("Stop {id}")),
        description: None,
        latitude: Some(latitude),
        longitude: Some(18.0),
        location_type: LocationType::STOP_POINT,
        parent: None,
    }
}

fn call(trip: usize, stop: usize, sequence: u32, seconds: u32) -> Call {
    Call {
        trip,
        stop,
        sequence,
        arrival: Optional::new(Time(seconds)),
        departure: Optional::new(Time(seconds)),
        headsign: None,
        distance: Optional::default(),
        pickup: BoardingRule::REGULAR,
        drop_off: BoardingRule::REGULAR,
    }
}

/// Two stops, one route that runs every day of 2026, and one trip between the stops.
fn network() -> Network {
    Network {
        stops: vec![stop("A", 59.0), stop("B", 59.01)],
        routes: vec![RouteRecord {
            id: "R".to_owned(),
            short_name: Some("1".to_owned()),
            long_name: None,
            description: None,
            route_type: RouteType::BUS,
        }],
        services: vec![ServiceRecord {
            id: "D".to_owned(),
            weekly: Some(Weekly {
                days: [true; 7],
                start: Date::parse_gtfs("20260101").expect("a date"),
                end: Date::parse_gtfs("20261231").expect("a date"),
            }),
            exceptions: Vec::new(),
        }],
        trips: vec![TripRecord {
            id: "T".to_owned(),
            route: 0,
            service: Some(0),
            headsign: None,
            short_name: None,
            line: None,
            block: None,
            direction: Direction::NONE,
        }],
        // Out of order on purpose: the compiler orders them.
        calls: vec![call(0, 1, 2, 28_800 + 600), call(0, 0, 1, 28_800)],
        ..Network::default()
    }
}

#[allow(
    unsafe_code,
    reason = "mapping a file is unsafe in memmap2, and this file is not modified"
)]
fn with_consumer(bytes: &[u8], check: impl FnOnce(&Consumer<'_>)) {
    let (_file, mmap): (_, Mmap) = common::map(bytes).expect("the file maps");
    check(&Consumer::new(&mmap).expect("the file is valid"));
}

#[test]
fn a_network_built_by_hand_compiles_and_reads_back() {
    let bytes = Compiler::new()
        .compile(network())
        .expect("the network is valid");

    with_consumer(&bytes, |consumer| {
        let trip = consumer.trip_by_id("T").expect("the trip is there");
        let stops =
            consumer.stop_sequence_by_trip_pattern(consumer.trip_pattern_by_trip(trip.index).index);
        let timings = consumer.timings_by_trip(trip.index);

        assert_eq!(stops.len(), 2, "two stops in the pattern");
        assert_eq!(
            consumer.stop(stops[0]).index,
            consumer.stop_by_id("A").expect("stop A").index
        );
        assert_eq!(
            timings[0].departure.as_option().map(|time| time.0),
            Some(28_800),
            "the call with the lower sequence comes first"
        );
        assert_eq!(
            timings[1].arrival.as_option().map(|time| time.0),
            Some(29_400),
            "ten minutes later"
        );
    });
}

#[test]
fn the_same_network_gives_the_same_bytes() {
    let compiler = Compiler::new();

    assert_eq!(
        compiler.compile(network()).expect("the network is valid"),
        compiler.compile(network()).expect("the network is valid"),
        "a network compiles to one file"
    );
}

#[test]
fn two_stops_with_one_id_are_refused() {
    let mut network = network();
    network.stops.push(stop("A", 59.02));

    let result = Compiler::new().compile(network);

    assert!(
        matches!(result, Err(Error::DuplicateId { table: "stops", .. })),
        "{result:?}"
    );
}

#[test]
fn a_reference_that_points_at_nothing_is_refused() {
    let mut network = network();
    network.calls.push(call(0, 7, 3, 30_000));

    let result = Compiler::new().compile(network);

    assert!(
        matches!(
            result,
            Err(Error::InvalidReference {
                table: "calls",
                field: "stop"
            })
        ),
        "{result:?}"
    );
}

#[test]
fn a_service_with_no_days_is_refused() {
    let mut network = network();
    network.services.push(ServiceRecord {
        id: "EMPTY".to_owned(),
        weekly: None,
        exceptions: Vec::new(),
    });

    let result = Compiler::new().compile(network);

    assert!(
        matches!(result, Err(Error::EmptyService { .. })),
        "{result:?}"
    );
}

#[test]
fn a_service_that_runs_for_centuries_is_refused() {
    let mut network = network();
    network.services[0].exceptions.push((Date(2_000_000), true));

    let result = Compiler::new().compile(network);

    assert!(
        matches!(result, Err(Error::ServiceRangeTooLong { .. })),
        "{result:?}"
    );
}

#[test]
fn a_date_that_is_not_a_real_day_is_refused() {
    for bad in [Date(u32::MAX), Date(3_000_000)] {
        let mut network = network();
        network.services[0].exceptions.push((bad, true));

        let result = Compiler::new().compile(network);

        assert!(
            matches!(result, Err(Error::InvalidDate { .. })),
            "{bad:?} is past the year 9999 or the sentinel: {result:?}"
        );
    }
}

#[test]
fn a_position_off_the_globe_is_refused() {
    for (latitude, longitude) in [(1000.0, 18.0), (59.0, 181.0), (f64::NAN, 18.0)] {
        let mut network = network();
        network.stops[0].latitude = Some(latitude);
        network.stops[0].longitude = Some(longitude);

        let result = Compiler::new().compile(network);

        assert!(
            matches!(result, Err(Error::InvalidPosition { .. })),
            "({latitude}, {longitude}) is not on the globe: {result:?}"
        );
    }
}
