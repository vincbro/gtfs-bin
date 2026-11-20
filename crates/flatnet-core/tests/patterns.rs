//! How the compiler groups trips into patterns and lays their times out, and what the consumer
//! refuses when that layout is damaged.

use std::{fs, path::Path};

use flatnet_compiler::Compiler;
use flatnet_core::{
    consumer::Consumer,
    models::{Header, PatternStop, Slice, Timing, Trip},
};
use memmap2::Mmap;

mod common;

/// Trips over the stops A and B, listed out of order. `EARLY` and `LATE` are never overtaken, and
/// `FAST` leaves after `EARLY` but reaches B before it. `NORTH` is signed for another destination,
/// and `VIA` calls at another stop.
fn write_feed(directory: &Path) {
    let files = [
        (
            "agency.txt",
            "agency_id,agency_name,agency_url,agency_timezone\nA,A,https://example.com,Europe/Stockholm\n",
        ),
        (
            "stops.txt",
            "stop_id,stop_name,stop_lat,stop_lon\nA,A,59.0,18.0\nB,B,59.01,18.0\nC,C,59.02,18.0\n",
        ),
        (
            "routes.txt",
            "route_id,route_short_name,route_type\nR,R,3\n",
        ),
        (
            "calendar.txt",
            "service_id,monday,tuesday,wednesday,thursday,friday,saturday,sunday,start_date,end_date\nD,1,1,1,1,1,1,1,20260101,20261231\n",
        ),
        (
            "trips.txt",
            "route_id,service_id,trip_id\nR,D,LATE\nR,D,EARLY\nR,D,FAST\nR,D,NORTH\nR,D,VIA\n",
        ),
        (
            "stop_times.txt",
            "trip_id,arrival_time,departure_time,stop_id,stop_sequence,stop_headsign\n\
             LATE,09:00:00,09:00:00,A,1,\n\
             LATE,09:10:00,09:10:00,B,2,\n\
             EARLY,08:00:00,08:00:00,A,1,\n\
             EARLY,08:10:00,08:10:00,B,2,\n\
             FAST,08:05:00,08:05:00,A,1,\n\
             FAST,08:08:00,08:08:00,B,2,\n\
             NORTH,10:00:00,10:00:00,A,1,North\n\
             NORTH,10:10:00,10:10:00,B,2,\n\
             VIA,07:00:00,07:00:00,A,1,\n\
             VIA,07:10:00,07:10:00,C,2,\n",
        ),
    ];
    for (name, contents) in files {
        fs::write(directory.join(name), contents).expect("a feed file can be written");
    }
}

fn compile() -> Vec<u8> {
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    write_feed(directory.path());
    Compiler::new()
        .compile_gtfs(directory.path())
        .expect("the feed compiles")
}

#[allow(
    unsafe_code,
    reason = "mapping a file is unsafe in memmap2, and this file is not modified"
)]
fn with_consumer(check: impl FnOnce(&Consumer<'_>)) {
    let bytes = compile();
    let (_file, mmap): (_, Mmap) = common::map(&bytes).expect("the file maps");
    check(&Consumer::new(&mmap).expect("the file is valid"));
}

#[test]
fn trips_that_agree_share_a_pattern_in_the_order_they_leave() {
    with_consumer(|consumer| {
        let early = consumer.trip_by_id("EARLY").expect("the trip exists");
        let late = consumer.trip_by_id("LATE").expect("the trip exists");

        let pattern = consumer.trip_pattern_by_trip(early.index);

        assert_eq!(
            consumer.trip_pattern_by_trip(late.index).index,
            pattern.index,
            "the same stops, the same rules and nothing overtaken"
        );
        assert_eq!(
            consumer.trips_in_trip_pattern(pattern.index),
            [early.index, late.index],
            "EARLY leaves first, though LATE is first in the feed"
        );
    });
}

#[test]
fn a_trip_that_overtakes_another_gets_its_own_pattern() {
    with_consumer(|consumer| {
        let early = consumer.trip_by_id("EARLY").expect("the trip exists");
        let fast = consumer.trip_by_id("FAST").expect("the trip exists");

        assert_ne!(
            consumer.trip_pattern_by_trip(fast.index).index,
            consumer.trip_pattern_by_trip(early.index).index,
            "FAST reaches B before EARLY does"
        );
    });
}

#[test]
fn another_headsign_or_other_stops_make_another_pattern() {
    with_consumer(|consumer| {
        let early = consumer.trip_by_id("EARLY").expect("the trip exists");
        let north = consumer.trip_by_id("NORTH").expect("the trip exists");
        let via = consumer.trip_by_id("VIA").expect("the trip exists");
        let pattern_of = |trip: &Trip| consumer.trip_pattern_by_trip(trip.index).index;

        assert_ne!(pattern_of(north), pattern_of(early), "another headsign");
        assert_ne!(pattern_of(via), pattern_of(early), "another stop");
        let headsign = consumer.pattern_stops_by_trip_pattern(pattern_of(north))[0]
            .headsign
            .as_option()
            .map(|slice| consumer.string(slice));
        assert_eq!(headsign, Some("North"), "the pattern keeps the headsign");
    });
}

#[test]
fn a_trip_reads_its_own_times() {
    with_consumer(|consumer| {
        let late = consumer.trip_by_id("LATE").expect("the trip exists");

        let timings = consumer.timings_by_trip(late.index);

        assert_eq!(timings.len(), 2, "one for each stop of the pattern");
        assert_eq!(
            timings[0]
                .departure
                .as_option()
                .map(|time| time.to_string()),
            Some("09:00:00".to_owned()),
            "LATE leaves A at nine"
        );
        assert_eq!(
            timings[1].arrival.as_option().map(|time| time.to_string()),
            Some("09:10:00".to_owned()),
            "and reaches B ten minutes later"
        );
    });
}

#[test]
fn the_timings_lie_pattern_by_pattern_with_nothing_left_over() {
    with_consumer(|consumer| {
        let mut next = 0;
        for pattern in consumer.trip_patterns() {
            for &trip in consumer.trips_in_trip_pattern(pattern.index) {
                let range = consumer.trip(trip).stop_times.range();
                assert_eq!(range.start, next, "a trip starts where the one before ends");
                next = range.end;
            }
        }

        assert_eq!(next, consumer.timings().len(), "every timing is a trip's");
    });
}

fn damaged_timings(damage: impl FnOnce(&mut [Timing])) -> Vec<u8> {
    common::damage_section(compile(), |header| header.timings, damage)
}

#[test]
fn a_trip_whose_times_are_not_where_the_layout_puts_them_is_rejected() {
    // NORTH and VIA are each alone in a pattern with two stops, so trading their times breaks
    // nothing but where each trip's times lie.
    let bytes = common::damage_section::<Trip>(
        compile(),
        |header| header.trips,
        |trips| {
            let (north, via) = (trips[3].stop_times, trips[4].stop_times);
            trips[3].stop_times = via;
            trips[4].stop_times = north;
        },
    );

    assert!(
        common::is_rejected(&bytes),
        "a trip's times start where the trip before it ends"
    );
}

#[test]
fn a_trip_with_a_time_for_too_few_stops_is_rejected() {
    let bytes = common::damage_section::<Trip>(
        compile(),
        |header| header.trips,
        |trips| trips[0].stop_times.count = 1,
    );

    assert!(
        common::is_rejected(&bytes),
        "a trip has one time for each stop of its pattern"
    );
}

#[test]
#[allow(
    unsafe_code,
    reason = "mapping a file is unsafe in memmap2, and this file is not modified"
)]
fn a_trip_that_overtakes_the_one_before_it_is_rejected() {
    let bytes = compile();
    let (_file, mmap): (_, Mmap) = common::map(&bytes).expect("the file maps");
    let consumer = Consumer::new(&mmap).expect("the file is valid");
    let start_of = |id: &str| {
        let trip = consumer.trip_by_id(id).expect("the trip exists");
        trip.stop_times.start as usize
    };
    let (early, late) = (start_of("EARLY"), start_of("LATE"));

    // EARLY and LATE trade times, so the trip that follows is the earlier one.
    let bytes = damaged_timings(|timings| {
        timings.swap(early, late);
        timings.swap(early + 1, late + 1);
    });

    assert!(
        common::is_rejected(&bytes),
        "a later trip is never earlier at a stop"
    );
}

#[test]
fn sequence_numbers_that_fall_along_a_pattern_are_rejected() {
    let bytes = common::damage_section::<PatternStop>(
        compile(),
        |header| header.pattern_stops,
        <[PatternStop]>::reverse,
    );

    assert!(
        common::is_rejected(&bytes),
        "the stops of a pattern are in sequence order"
    );
}

#[test]
fn a_timing_no_trip_owns_is_rejected() {
    let mut bytes = compile();
    let mut header: Header = bytemuck::pod_read_unaligned(&bytes[..size_of::<Header>()]);
    header.timings.count += 1;
    bytes[..size_of::<Header>()].copy_from_slice(bytemuck::bytes_of(&header));

    assert!(
        common::is_rejected(&bytes),
        "the timings are the trips' and nothing else"
    );
}
