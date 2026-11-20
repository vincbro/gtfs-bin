//! How the compiler cuts a feed's shapes at the stops, and shares the pieces.

use std::{fmt::Write as _, fs, path::Path};

use bytemuck::Pod;
use flatnet_compiler::Compiler;
use flatnet_core::{
    consumer::Consumer,
    models::{Header, Optional, Section, Shape, ShapeIndex, Slice, Trip, TripShapeSlice},
};
use memmap2::Mmap;

mod common;

/// Five stops a kilometer apart along one meridian, the last without a position. The `LINE`
/// shape runs through the first four with a vertex between each, and `COPY` has the same points
/// under another id.
fn write_feed(directory: &Path) {
    let files = [
        ("agency.txt", "agency_id,agency_name,agency_url,agency_timezone\nA,A,https://example.com,Europe/Stockholm\n".to_owned()),
        (
            "stops.txt",
            "stop_id,stop_name,stop_lat,stop_lon\nS0,S0,59.000,18.0\nS1,S1,59.009,18.0\nS2,S2,59.018,18.0\nS3,S3,59.027,18.0\nS4,S4,,\n".to_owned(),
        ),
        ("routes.txt", "route_id,route_short_name,route_type\nR,R,3\n".to_owned()),
        (
            "calendar.txt",
            "service_id,monday,tuesday,wednesday,thursday,friday,saturday,sunday,start_date,end_date\nD,1,1,1,1,1,1,1,20260101,20261231\n".to_owned(),
        ),
        (
            "trips.txt",
            "route_id,service_id,trip_id,shape_id\nR,D,LONG,LINE\nR,D,SHORT,LINE\nR,D,TWIN,COPY\nR,D,PLAIN,\nR,D,NOPOS,LINE\n".to_owned(),
        ),
        ("stop_times.txt", stop_times()),
        ("shapes.txt", shapes()),
    ];
    for (name, contents) in files {
        fs::write(directory.join(name), contents).expect("a feed file can be written");
    }
}

fn stop_times() -> String {
    let trips = [
        ("LONG", &["S0", "S1", "S2", "S3"][..]),
        ("SHORT", &["S0", "S1", "S2"][..]),
        ("TWIN", &["S0", "S1", "S2", "S3"][..]),
        ("PLAIN", &["S0", "S1"][..]),
        ("NOPOS", &["S0", "S1", "S4"][..]),
    ];
    let mut text = "trip_id,arrival_time,departure_time,stop_id,stop_sequence\n".to_owned();
    for (trip, stops) in trips {
        for (sequence, stop) in stops.iter().enumerate() {
            let minute = sequence * 5;
            writeln!(
                text,
                "{trip},08:{minute:02}:00,08:{minute:02}:00,{stop},{sequence}"
            )
            .expect("writing to a string works");
        }
    }
    text
}

fn shapes() -> String {
    let mut text = "shape_id,shape_pt_lat,shape_pt_lon,shape_pt_sequence\n".to_owned();
    for id in ["LINE", "COPY"] {
        // A vertex every 0.0045 degrees, so each stop is on one and there is one between stops.
        for step in 0..=6 {
            let latitude = 59.0 + 0.0045 * f64::from(step);
            writeln!(text, "{id},{latitude:.5},18.0,{step}").expect("writing to a string works");
        }
    }
    text
}

#[allow(
    unsafe_code,
    reason = "mapping a file is unsafe in memmap2, and this file is not modified"
)]
fn with_consumer(check: impl FnOnce(&Consumer<'_>)) {
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    write_feed(directory.path());
    let bytes = Compiler::new(directory.path())
        .compile()
        .expect("the feed compiles");
    let (_file, mmap): (_, Mmap) = common::map(&bytes).expect("the file maps");
    check(&Consumer::new(&mmap).expect("the file is valid"));
}

#[test]
fn a_trip_has_one_shape_for_each_pair_of_stops() {
    with_consumer(|consumer| {
        let long = consumer.trip_by_id("LONG").expect("the trip exists");
        let short = consumer.trip_by_id("SHORT").expect("the trip exists");

        assert_eq!(consumer.shapes_by_trip(long.index).len(), 3);
        assert_eq!(consumer.shapes_by_trip(short.index).len(), 2);
    });
}

#[test]
fn shapes_run_from_one_stop_to_the_next() {
    with_consumer(|consumer| {
        let long = consumer.trip_by_id("LONG").expect("the trip exists");

        for &shape in consumer.shapes_by_trip(long.index) {
            let points = consumer.shape_coordinates(shape);
            // Two vertices a hop apart, and the one between them.
            assert_eq!(points.len(), 3, "start, middle and end");
            let straight = points[0].distance_meters(points[2]);
            let length = f64::from(consumer.shape(shape).length.0);
            assert!(
                (length - straight).abs() < 1.0,
                "a straight path is as long as the distance between its ends"
            );
            assert!(
                (900.0..1100.0).contains(&length),
                "a hop is about a kilometer"
            );
        }
    });
}

#[test]
fn trips_over_the_same_road_share_their_shapes() {
    with_consumer(|consumer| {
        let long = consumer.trip_by_id("LONG").expect("the trip exists");
        let short = consumer.trip_by_id("SHORT").expect("the trip exists");
        let twin = consumer.trip_by_id("TWIN").expect("the trip exists");

        let long_shapes = consumer.shapes_by_trip(long.index);
        assert_eq!(
            consumer.shapes_by_trip(short.index),
            &long_shapes[..2],
            "the first two hops are the same pieces of road"
        );
        assert_eq!(
            consumer.shapes_by_trip(twin.index),
            long_shapes,
            "another id for the same points adds nothing"
        );
        assert_eq!(consumer.shapes().len(), 3, "three hops of road in all");
        assert_eq!(
            consumer.shape_points().len(),
            9,
            "each point is stored once per hop it is in"
        );
    });
}

#[test]
fn a_trip_that_cannot_be_cut_has_no_shape() {
    with_consumer(|consumer| {
        for id in ["PLAIN", "NOPOS"] {
            let trip = consumer.trip_by_id(id).expect("the trip exists");
            assert!(
                consumer.shapes_by_trip(trip.index).is_empty(),
                "{id} has no shape id, or a stop with no position"
            );
        }
    });
}

/// Compiles the feed and lets `damage` change one section of the file's bytes.
fn damaged<T: Pod>(section_of: fn(&Header) -> Section, damage: impl FnOnce(&mut [T])) -> Vec<u8> {
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    write_feed(directory.path());
    let bytes = Compiler::new(directory.path())
        .compile()
        .expect("the feed compiles");
    common::damage_section(bytes, section_of, damage)
}

#[test]
fn a_trip_shape_past_the_shapes_is_rejected() {
    let bytes = damaged::<ShapeIndex>(
        |header| header.trip_shapes,
        |indices| {
            indices[0] = ShapeIndex(1_000);
        },
    );

    assert!(common::is_rejected(&bytes), "the index points at no shape");
}

#[test]
fn a_shape_with_points_past_the_table_is_rejected() {
    let bytes = damaged::<Shape>(
        |header| header.shapes,
        |shapes| {
            shapes[0].points.count = 1_000;
        },
    );

    assert!(
        common::is_rejected(&bytes),
        "the points end outside shape_points"
    );
}

#[test]
fn a_shape_without_points_is_rejected() {
    let bytes = damaged::<Shape>(
        |header| header.shapes,
        |shapes| {
            shapes[0].points.count = 0;
        },
    );

    assert!(common::is_rejected(&bytes), "a piece of path needs a point");
}

#[test]
fn a_shape_with_no_length_is_rejected() {
    let bytes = damaged::<Shape>(
        |header| header.shapes,
        |shapes| {
            shapes[0].length.0 = f32::NAN;
        },
    );

    assert!(common::is_rejected(&bytes), "a length is a number");
}

#[test]
fn a_trip_with_the_wrong_number_of_shapes_is_rejected() {
    let bytes = damaged::<Trip>(
        |header| header.trips,
        |trips| {
            let trip = trips
                .iter_mut()
                .find(|trip| trip.shape.is_some())
                .expect("a trip has shapes");
            trip.shape = Optional::new(TripShapeSlice::new(0, 1));
        },
    );

    assert!(
        common::is_rejected(&bytes),
        "a trip needs one shape per hop, and this one has too few"
    );
}
