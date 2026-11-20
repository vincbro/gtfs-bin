//! The v5 additions: boarding rules, direction, blocks, trip transfers, checksums, the stop grid,
//! name search, lenient mode and feed metadata.

mod common;

use std::{fs, path::Path};

use flatnet_compiler::{
    Compiler, Error,
    gtfs::{self, ReadReport},
};
use flatnet_core::{
    consumer::{self, Consumer},
    models::{BoardingRule, Coordinate, Date, Direction, SearchIndex, Sentinel, StopIndex},
    rt::Realtime,
};
use proptest::prelude::*;

const STOPS: &str =
    "stop_id,stop_name,stop_lat,stop_lon\nA,Alpha,59.30,18.00\nB,Beta,59.31,18.01\n";
const ROUTES: &str = "route_id,route_short_name,route_type\nR,1,3\n";
const CALENDAR: &str = "service_id,monday,tuesday,wednesday,thursday,friday,saturday,sunday,start_date,end_date\n\
    S,1,1,1,1,1,0,0,20240101,20240131\n";

#[allow(clippy::unwrap_in_result, reason = "a test helper that fails the test")]
fn compile_with(files: &[(&str, &str)]) -> Result<(Vec<u8>, ReadReport), Error> {
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    write(directory.path(), files);
    let (network, report) = gtfs::Reader::new(directory.path()).read_with_report()?;
    Ok((Compiler::new().compile(network)?, report))
}

fn write(directory: &Path, files: &[(&str, &str)]) {
    for (name, content) in files {
        fs::write(directory.join(name), content).expect("the temp directory is writable");
    }
}

fn feed<'a>(trips: &'a str, stop_times: &'a str) -> Vec<(&'a str, &'a str)> {
    vec![
        ("stops.txt", STOPS),
        ("routes.txt", ROUTES),
        ("trips.txt", trips),
        ("stop_times.txt", stop_times),
        ("calendar.txt", CALENDAR),
    ]
}

fn with_consumer(bytes: &[u8], body: impl FnOnce(&Consumer<'_>)) {
    let (_file, mmap) = common::map(bytes).expect("a compiled file maps");
    body(&Consumer::new(&mmap).expect("the compiler wrote a valid file"));
}

#[test]
fn boarding_rules_direction_and_block_are_kept() {
    let trips = "route_id,service_id,trip_id,direction_id,block_id\nR,S,T,1,B7\nR,S,U,,\n";
    let stop_times = "trip_id,stop_id,stop_sequence,arrival_time,departure_time,pickup_type,drop_off_type\n\
        T,A,1,08:00:00,08:00:00,0,1\nT,B,2,08:10:00,08:10:00,3,0\nU,A,1,09:00:00,09:00:00,,\n";

    let (bytes, _) = compile_with(&feed(trips, stop_times)).expect("a valid feed");

    with_consumer(&bytes, |consumer| {
        let trip = consumer.trip_by_id("T").expect("trip T");
        let unset = consumer.trip_by_id("U").expect("trip U");
        assert_eq!(
            trip.direction,
            Direction::INBOUND,
            "direction_id 1 is inbound"
        );
        assert_eq!(unset.direction, Direction::NONE, "no direction is NONE");
        assert_eq!(
            trip.block.as_option().map(|block| consumer.string(block)),
            Some("B7"),
            "the block id is kept"
        );

        let stops =
            consumer.pattern_stops_by_trip_pattern(consumer.trip_pattern_by_trip(trip.index).index);
        assert_eq!(
            stops[0].drop_off,
            BoardingRule::NOT_AVAILABLE,
            "no alighting at the first stop"
        );
        assert_eq!(
            stops[1].pickup,
            BoardingRule::ARRANGE_WITH_DRIVER,
            "boarding is arranged with the driver"
        );
        assert!(
            consumer
                .pattern_stops_by_trip_pattern(consumer.trip_pattern_by_trip(unset.index).index)[0]
                .pickup
                .is_regular(),
            "an empty code is the default"
        );
    });
}

#[test]
fn times_between_timed_stops_are_filled_in() {
    let trips = "route_id,service_id,trip_id\nR,S,T\n";
    let stop_times = "trip_id,stop_id,stop_sequence,arrival_time,departure_time\n\
        T,A,1,08:00:00,08:00:00\nT,B,2,,\nT,A,3,,\nT,B,4,08:30:00,08:30:00\n";

    let (bytes, _) = compile_with(&feed(trips, stop_times)).expect("a valid feed");

    with_consumer(&bytes, |consumer| {
        let trip = consumer.trip_by_id("T").expect("trip T");
        let times: Vec<String> = consumer
            .timings_by_trip(trip.index)
            .iter()
            .map(|timing| timing.arrival.as_option().expect("a time").to_string())
            .collect();
        assert_eq!(
            times,
            ["08:00:00", "08:10:00", "08:20:00", "08:30:00"],
            "even thirds"
        );
    });
}

#[test]
fn a_transfer_between_trips_is_kept_and_one_naming_an_unknown_trip_is_dropped() {
    let trips = "route_id,service_id,trip_id\nR,S,T\nR,S,U\n";
    let stop_times = "trip_id,stop_id,stop_sequence,arrival_time,departure_time\n\
        T,A,1,08:00:00,08:00:00\nU,B,1,09:00:00,09:00:00\n";
    let transfers = "from_stop_id,to_stop_id,from_trip_id,to_trip_id,transfer_type\n\
        A,B,T,U,4\nA,B,T,GHOST,4\nA,B,,,0\n";
    let mut files = feed(trips, stop_times);
    files.push(("transfers.txt", transfers));

    let (bytes, _) = compile_with(&files).expect("a valid feed");

    with_consumer(&bytes, |consumer| {
        assert_eq!(
            consumer.transfers().len(),
            2,
            "the ghost transfer is dropped"
        );
        let in_seat = consumer
            .transfers()
            .iter()
            .find(|transfer| transfer.from_trip_index.is_some())
            .expect("the trip transfer is kept");
        assert_eq!(in_seat.transfer_type, 4, "in-seat");
        assert_eq!(
            in_seat.to_trip_index.as_option(),
            consumer.trip_by_id("U").map(|trip| trip.index),
            "it ends on trip U"
        );
    });
}

#[test]
fn feed_metadata_is_stored() {
    let bytes = common::compile_fixture();

    with_consumer(&bytes, |consumer| {
        assert_eq!(consumer.timezone(), Some("Europe/Paris"), "from agency.txt");
        assert_eq!(
            consumer.feed_start_date(),
            Date::parse_gtfs("20180709"),
            "from feed_info.txt"
        );
        assert_eq!(
            consumer.feed_end_date(),
            Date::parse_gtfs("20180927"),
            "from feed_info.txt"
        );
        let (first, last) = consumer.service_range().expect("the fixture has services");
        assert_eq!(
            first,
            Date::parse_gtfs("20170101").expect("a date"),
            "service1 starts first"
        );
        assert_eq!(
            last,
            Date::parse_gtfs("20170115").expect("a date"),
            "service1 ends last"
        );
    });
}

#[test]
fn a_feed_without_agency_or_feed_info_has_no_metadata() {
    let trips = "route_id,service_id,trip_id\nR,S,T\n";
    let stop_times = "trip_id,stop_id,stop_sequence\nT,A,1\n";

    let (bytes, _) = compile_with(&feed(trips, stop_times)).expect("a valid feed");

    with_consumer(&bytes, |consumer| {
        assert_eq!(consumer.timezone(), None, "no agency, no zone");
        assert_eq!(consumer.feed_start_date(), None, "no feed_info, no start");
    });
}

#[test]
fn the_checksum_catches_a_changed_byte_and_the_length_a_truncated_file() {
    let bytes = common::compile_fixture();
    {
        let (_file, mmap) = common::map(&bytes).expect("the fixture maps");
        let consumer = Consumer::new(&mmap).expect("the fixture is valid");
        assert!(
            consumer.verify_checksum().is_ok(),
            "an untouched file matches"
        );
    }

    // A change to a coordinate keeps every index valid, so only the checksum can see it.
    let mut changed = bytes.clone();
    let last = changed.len() - 1;
    changed[last] ^= 1;
    let (_file, mmap) = common::map(&changed).expect("the file maps");
    let consumer = Consumer::new_unvalidated(&mmap).expect("the header is intact");
    assert!(
        matches!(
            consumer.verify_checksum(),
            Err(consumer::Error::ChecksumMismatch)
        ),
        "the last byte changed"
    );

    let (_file, mmap) = common::map(&bytes[..bytes.len() - 8]).expect("the file maps");
    assert!(
        matches!(
            Consumer::new(&mmap),
            Err(consumer::Error::LengthMismatch { .. })
        ),
        "a file that lost its end is noticed by its length"
    );
}

#[test]
fn an_unvalidated_open_skips_the_table_checks_but_not_the_header_checks() {
    let mut bytes = common::compile_fixture();
    bytes[0] = b'X';
    let (_file, mmap) = common::map(&bytes).expect("the file maps");

    assert!(
        matches!(
            Consumer::new_unvalidated(&mmap),
            Err(consumer::Error::InvalidMagic)
        ),
        "the header is always checked"
    );
}

#[test]
fn the_state_can_be_shared_between_threads() {
    fn assert_shareable<T: Send + Sync>() {}

    assert_shareable::<Consumer<'_>>();
    assert_shareable::<Realtime>();
}

#[cfg(unix)]
#[test]
fn the_operating_system_can_be_told_how_the_file_is_read() {
    let bytes = common::compile_fixture();
    let (_file, mmap) = common::map(&bytes).expect("the fixture maps");

    assert!(
        consumer::advise(&mmap, consumer::Advice::WillNeed).is_ok(),
        "read ahead is allowed"
    );
    assert!(
        consumer::advise(&mmap, consumer::Advice::Random).is_ok(),
        "so is random access"
    );
}

#[test]
fn lenient_mode_skips_bad_rows_and_says_which() {
    let stops = "stop_id,stop_name,stop_lat,stop_lon\nA,Alpha,59.30,18.00\nB,Beta,north,18.01\nC,Gamma,59.32,18.02\n";
    let trips = "route_id,service_id,trip_id\nR,S,T\n";
    let stop_times =
        "trip_id,stop_id,stop_sequence,arrival_time\nT,A,1,08:00:00\nT,C,2,soon\nT,C,3,08:10:00\n";
    let mut files = feed(trips, stop_times);
    files[0] = ("stops.txt", stops);

    let strict = compile_with(&files);
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    write(directory.path(), &files);
    let (network, report) = gtfs::Reader::new(directory.path())
        .lenient()
        .read_with_report()
        .expect("lenient mode gets through");
    let bytes = Compiler::new()
        .compile(network)
        .expect("what was read compiles");

    assert!(strict.is_err(), "strict mode refuses the feed");
    assert_eq!(report.skipped_total, 2, "one stop and one stop time");
    assert!(
        report
            .skipped_rows
            .iter()
            .any(|row| row.file == "stops.txt" && row.message.contains("north")),
        "the stop row is named: {:?}",
        report.skipped_rows
    );
    with_consumer(&bytes, |consumer| {
        assert_eq!(consumer.stops().len(), 2, "stop B is left out");
        assert_eq!(
            consumer.timings().len(),
            2,
            "the stop time with a bad time is left out"
        );
    });
}

#[test]
fn lenient_mode_still_fails_for_a_missing_column() {
    let stops = "stop_name,stop_lat\nAlpha,59.30\n";
    let trips = "route_id,service_id,trip_id\nR,S,T\n";
    let mut files = feed(trips, "trip_id,stop_id,stop_sequence\nT,A,1\n");
    files[0] = ("stops.txt", stops);
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    write(directory.path(), &files);

    let result = gtfs::Reader::new(directory.path()).lenient().read();

    assert!(
        matches!(result, Err(Error::MissingColumn { .. })),
        "the file can't be read at all"
    );
}

#[test]
fn lenient_mode_drops_a_reversed_service() {
    let calendar = "service_id,monday,tuesday,wednesday,thursday,friday,saturday,sunday,start_date,end_date\n\
        S,1,1,1,1,1,0,0,20240101,20240131\nBAD,1,1,1,1,1,0,0,20240131,20240101\n";
    let trips = "route_id,service_id,trip_id\nR,S,T\nR,BAD,U\n";
    let mut files = feed(trips, "trip_id,stop_id,stop_sequence\nT,A,1\nU,A,1\n");
    files[4] = ("calendar.txt", calendar);
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    write(directory.path(), &files);

    let (network, report) = gtfs::Reader::new(directory.path())
        .lenient()
        .read_with_report()
        .expect("lenient mode gets through");
    let bytes = Compiler::new()
        .compile(network)
        .expect("what was read compiles");

    assert_eq!(report.skipped_total, 1, "the reversed service");
    with_consumer(&bytes, |consumer| {
        assert_eq!(consumer.services().len(), 1, "only the good service stays");
        let trip = consumer.trip_by_id("U").expect("the trip stays");
        assert!(
            trip.service_index.is_none(),
            "its service is unknown, so it never runs"
        );
    });
}

#[test]
fn nearby_stops_are_found_nearest_first() {
    with_consumer(&common::compile_fixture(), |consumer| {
        let mut found = Vec::new();

        // stop6 "Central Station" is at 48.8566, 2.3522, and stop7 is 73 meters away from it.
        consumer.stops_near(48.8566, 2.3522, 200.0, &mut found);

        let names: Vec<&str> = found
            .iter()
            .map(|&(stop, _)| consumer.stop_id(consumer.stop(stop).id))
            .collect();
        assert_eq!(
            names,
            ["stop6", "stop7"],
            "the station first, then the square"
        );
        assert!(found[0].1 < 1.0, "the first is right here");
        assert!(
            (70.0..77.0).contains(&found[1].1),
            "the second is about 73 m away: {}",
            found[1].1
        );
    });
}

#[test]
fn nothing_is_found_for_a_bad_search() {
    with_consumer(&common::compile_fixture(), |consumer| {
        let mut found = vec![(StopIndex(0), 0.0)];

        consumer.stops_near(f64::NAN, 2.0, 100.0, &mut found);
        assert!(
            found.is_empty(),
            "NaN is not a place, and the old result is cleared"
        );

        consumer.stops_near(48.8, 2.3, f64::INFINITY, &mut found);
        assert!(found.is_empty(), "an infinite radius is refused");

        consumer.stops_near(0.0, 0.0, 1000.0, &mut found);
        assert!(found.is_empty(), "nothing is near the middle of the ocean");
    });
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    /// The grid must give exactly what looking at every stop gives.
    #[test]
    fn the_grid_agrees_with_a_scan_of_every_stop(
        latitude in 48.78_f64..48.88,
        longitude in 2.33_f64..2.47,
        radius in 0.0_f64..20_000.0,
    ) {
        let bytes = common::compile_fixture();
        let (_file, mmap) = common::map(&bytes).expect("the fixture maps");
        let consumer = Consumer::new(&mmap).expect("the fixture is valid");

        let mut found = Vec::new();
        consumer.stops_near(latitude, longitude, radius, &mut found);

        let here = Coordinate::new(latitude, longitude);
        let mut expected: Vec<_> = consumer
            .stops()
            .iter()
            .filter_map(|stop| {
                let distance = here.distance_meters(stop.coordinate.as_option()?);
                (distance <= radius).then_some(stop.index)
            })
            .collect();
        expected.sort_unstable();
        let mut got: Vec<_> = found.iter().map(|&(stop, _)| stop).collect();
        got.sort_unstable();
        prop_assert_eq!(got, expected);
        prop_assert!(found.windows(2).all(|pair| pair[0].1 <= pair[1].1), "nearest first");
    }
}

#[test]
fn names_are_found_ignoring_case_and_accents() {
    let stops = "stop_id,stop_name,stop_lat,stop_lon\nA,Örebro Central,59.3,18.0\nB,Central Station,59.4,18.1\nC,Västerås,59.5,18.2\nD,Södra Central,59.6,18.3\n";
    let trips = "route_id,service_id,trip_id\nR,S,T\n";
    let mut files = feed(trips, "trip_id,stop_id,stop_sequence\nT,A,1\n");
    files[0] = ("stops.txt", stops);
    let (bytes, _) = compile_with(&files).expect("a valid feed");

    with_consumer(&bytes, |consumer| {
        let mut found = Vec::new();
        let names = |found: &[SearchIndex]| -> Vec<String> {
            found
                .iter()
                .map(|&index| consumer.string(consumer.search(index).name).to_owned())
                .collect()
        };

        consumer.search_by_name("orebro", &mut found);
        assert_eq!(names(&found), ["Örebro Central"], "ö matches o");

        consumer.search_by_name("CENTRAL", &mut found);
        assert_eq!(
            names(&found),
            ["Central Station", "Örebro Central", "Södra Central"],
            "a name that starts with it comes before names with a word that does"
        );

        consumer.search_by_name("vasteras", &mut found);
        assert_eq!(names(&found), ["Västerås"], "ä and å match a");

        consumer.search_by_name("  ", &mut found);
        assert!(
            found.is_empty(),
            "an empty query finds nothing, and clears the old result"
        );
    });
}
