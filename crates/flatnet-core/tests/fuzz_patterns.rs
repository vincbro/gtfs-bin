//! Property tests that compile random feeds and check what the compiler promises of patterns: a
//! trip reads back the times and rules its feed gave it, and the trips of a pattern leave in order.

mod common;

use std::{fmt::Write as _, fs, path::Path};

use flatnet_compiler::Compiler;
use flatnet_core::{consumer::Consumer, models::Slice};
use proptest::{collection, prelude::*};

/// A trip that calls at the first `stops` of four stops, so many trips share their stops.
#[derive(Debug, Clone)]
struct Run {
    stops: usize,
    /// Seconds after midnight that it leaves the first stop.
    start: u32,
    /// For each stop: the seconds it takes to get there, and the seconds it waits.
    legs: Vec<(u32, u32)>,
    /// Signed for another destination at the first stop, which makes it another pattern.
    north: bool,
    /// Whether boarding is possible at the first stop.
    boards: bool,
}

fn run() -> impl Strategy<Value = Run> {
    (
        2_usize..=4,
        0_u32..6_000,
        collection::vec((30_u32..900, 0_u32..120), 4),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(|(stops, start, legs, north, boards)| Run {
            stops,
            start,
            legs,
            north,
            boards,
        })
}

/// The arrival and departure at each stop of the run, in seconds.
fn times(run: &Run) -> Vec<(u32, u32)> {
    let mut clock = run.start;
    run.legs[..run.stops]
        .iter()
        .enumerate()
        .map(|(position, &(ride, wait))| {
            if position > 0 {
                clock += ride;
            }
            let arrival = clock;
            clock += wait;
            (arrival, clock)
        })
        .collect()
}

fn clock(seconds: u32) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3_600,
        seconds % 3_600 / 60,
        seconds % 60
    )
}

fn write_feed(directory: &Path, runs: &[Run]) {
    let mut stop_times =
        "trip_id,arrival_time,departure_time,stop_id,stop_sequence,stop_headsign,pickup_type\n"
            .to_owned();
    let mut trips = "route_id,service_id,trip_id\n".to_owned();
    for (number, run) in runs.iter().enumerate() {
        writeln!(trips, "R,D,T{number}").expect("writing to a string works");
        for (position, (arrival, departure)) in times(run).into_iter().enumerate() {
            let headsign = if run.north && position == 0 {
                "North"
            } else {
                ""
            };
            let pickup = u8::from(position == 0 && !run.boards);
            writeln!(
                stop_times,
                "T{number},{},{},S{position},{},{headsign},{pickup}",
                clock(arrival),
                clock(departure),
                position + 1,
            )
            .expect("writing to a string works");
        }
    }
    let files = [
        ("agency.txt", "agency_id,agency_name,agency_url,agency_timezone\nA,A,https://example.com,Europe/Stockholm\n".to_owned()),
        ("stops.txt", "stop_id,stop_name,stop_lat,stop_lon\nS0,S0,59.0,18.0\nS1,S1,59.01,18.0\nS2,S2,59.02,18.0\nS3,S3,59.03,18.0\n".to_owned()),
        ("routes.txt", "route_id,route_short_name,route_type\nR,R,3\n".to_owned()),
        ("calendar.txt", "service_id,monday,tuesday,wednesday,thursday,friday,saturday,sunday,start_date,end_date\nD,1,1,1,1,1,1,1,20260101,20261231\n".to_owned()),
        ("trips.txt", trips),
        ("stop_times.txt", stop_times),
    ];
    for (name, contents) in files {
        fs::write(directory.join(name), contents).expect("a feed file can be written");
    }
}

#[allow(
    unsafe_code,
    reason = "mapping a file is unsafe in memmap2, and this file is not modified"
)]
fn check(runs: &[Run]) -> Result<(), TestCaseError> {
    let fail = |what: &str| TestCaseError::fail(what.to_owned());
    let directory = tempfile::tempdir().map_err(|_| fail("a temp directory can be created"))?;
    write_feed(directory.path(), runs);
    let bytes = Compiler::new()
        .compile_gtfs(directory.path())
        .map_err(|error| TestCaseError::fail(format!("the feed compiles: {error}")))?;
    let (_file, mmap) = common::map(&bytes).ok_or_else(|| fail("the file maps"))?;
    let consumer = Consumer::new(&mmap).map_err(|error| {
        TestCaseError::fail(format!(
            "the compiler wrote a file it can't read: {error:?}"
        ))
    })?;

    for (number, run) in runs.iter().enumerate() {
        let trip = consumer
            .trip_by_id(&format!("T{number}"))
            .ok_or_else(|| fail("every trip is kept"))?;
        let expected = times(run);
        let timings = consumer.timings_by_trip(trip.index);
        prop_assert_eq!(timings.len(), expected.len(), "a time for every stop");
        for (timing, (arrival, departure)) in timings.iter().zip(&expected) {
            prop_assert_eq!(
                timing.arrival.as_option().map(|time| time.0),
                Some(*arrival)
            );
            prop_assert_eq!(
                timing.departure.as_option().map(|time| time.0),
                Some(*departure)
            );
        }

        let pattern = consumer.trip_pattern_by_trip(trip.index);
        let stops = consumer.stop_sequence_by_trip_pattern(pattern.index);
        let rules = consumer.pattern_stops_by_trip_pattern(pattern.index);
        prop_assert_eq!(stops.len(), run.stops, "the pattern has the trip's stops");
        prop_assert_eq!(
            rules[0].pickup.is_regular(),
            run.boards,
            "the pattern keeps the boarding rule"
        );
        prop_assert_eq!(
            rules[0].headsign.is_some(),
            run.north,
            "the pattern keeps the headsign"
        );
    }

    for pattern in consumer.trip_patterns() {
        let leaving: Vec<u32> = consumer
            .trips_in_trip_pattern(pattern.index)
            .iter()
            .map(|&trip| {
                consumer.timings_by_trip(trip)[0]
                    .departure
                    .as_option()
                    .map_or(0, |time| time.0)
            })
            .collect();
        prop_assert!(
            leaving.windows(2).all(|pair| pair[0] <= pair[1]),
            "the trips of a pattern are in the order they leave"
        );
        prop_assert_eq!(
            pattern.trips.range().len(),
            leaving.len(),
            "a pattern's trips are all there"
        );
    }
    prop_assert!(
        consumer.trip_patterns().len() <= runs.len(),
        "no more patterns than trips"
    );
    Ok(())
}

proptest! {
    #[test]
    fn trips_read_back_their_times_and_patterns_keep_the_order(
        runs in collection::vec(run(), 1..14),
    ) {
        check(&runs)?;
    }
}
