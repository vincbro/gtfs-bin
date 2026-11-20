//! Property tests that compile random feeds and check what the compiler promises of shapes: a
//! trip with a shape has one for each pair of stops, and they join end to end into one path.

mod common;

use std::{collections::HashSet, fmt::Write as _, fs, path::Path};

use flatnet_compiler::Compiler;
use flatnet_core::consumer::Consumer;
use proptest::{collection, option, prelude::*};

/// A position in hundred thousandths of a degree, on a grid a few kilometers wide.
type Point = (u32, u32);

#[derive(Debug, Clone)]
struct Trip {
    /// The shape it follows, or `None` for no shape. An index past the shapes names no shape.
    shape: Option<usize>,
    /// The stops it calls at, as indices past which they wrap.
    stops: Vec<usize>,
}

#[derive(Debug, Clone)]
struct Network {
    stops: Vec<Point>,
    shapes: Vec<Vec<Point>>,
    trips: Vec<Trip>,
}

fn point() -> impl Strategy<Value = Point> {
    (0_u32..5_000, 0_u32..5_000)
}

fn network() -> impl Strategy<Value = Network> {
    (
        collection::vec(point(), 2..10),
        collection::vec(collection::vec(point(), 1..40), 1..4),
        collection::vec(
            (option::of(0_usize..5), collection::vec(0_usize..10, 2..8))
                .prop_map(|(shape, stops)| Trip { shape, stops }),
            1..8,
        ),
    )
        .prop_map(|(stops, shapes, trips)| Network {
            stops,
            shapes,
            trips,
        })
}

fn degrees(grid: u32, origin: f64) -> f64 {
    origin + f64::from(grid) / 100_000.0
}

fn write_network(directory: &Path, network: &Network) {
    let mut stops = "stop_id,stop_name,stop_lat,stop_lon\n".to_owned();
    for (index, &(north, east)) in network.stops.iter().enumerate() {
        let (latitude, longitude) = (degrees(north, 59.0), degrees(east, 18.0));
        writeln!(stops, "S{index},S{index},{latitude:.5},{longitude:.5}")
            .expect("writing to a string works");
    }
    let mut shapes = "shape_id,shape_pt_lat,shape_pt_lon,shape_pt_sequence\n".to_owned();
    for (shape, points) in network.shapes.iter().enumerate() {
        for (sequence, &(north, east)) in points.iter().enumerate() {
            let (latitude, longitude) = (degrees(north, 59.0), degrees(east, 18.0));
            writeln!(shapes, "H{shape},{latitude:.5},{longitude:.5},{sequence}")
                .expect("writing to a string works");
        }
    }
    let mut trips = "route_id,service_id,trip_id,shape_id\n".to_owned();
    let mut stop_times = "trip_id,arrival_time,departure_time,stop_id,stop_sequence\n".to_owned();
    for (index, trip) in network.trips.iter().enumerate() {
        let shape = trip
            .shape
            .map_or_else(String::new, |shape| format!("H{shape}"));
        writeln!(trips, "R,D,T{index},{shape}").expect("writing to a string works");
        for (sequence, stop) in trip.stops.iter().enumerate() {
            let stop = stop % network.stops.len();
            let minute = sequence * 3;
            writeln!(
                stop_times,
                "T{index},08:{minute:02}:00,08:{minute:02}:00,S{stop},{sequence}"
            )
            .expect("writing to a string works");
        }
    }
    let files = [
        (
            "agency.txt",
            "agency_id,agency_name,agency_url,agency_timezone\nA,A,https://example.com,Europe/Stockholm\n".to_owned(),
        ),
        ("routes.txt", "route_id,route_short_name,route_type\nR,R,3\n".to_owned()),
        (
            "calendar.txt",
            "service_id,monday,tuesday,wednesday,thursday,friday,saturday,sunday,start_date,end_date\nD,1,1,1,1,1,1,1,20260101,20261231\n".to_owned(),
        ),
        ("stops.txt", stops),
        ("shapes.txt", shapes),
        ("trips.txt", trips),
        ("stop_times.txt", stop_times),
    ];
    for (name, contents) in files {
        fs::write(directory.join(name), contents).expect("the temp directory is writable");
    }
}

/// Compiles the network and checks the promises about shapes.
fn check(network: &Network) {
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    write_network(directory.path(), network);
    let bytes = Compiler::new(directory.path())
        .compile()
        .expect("a well formed feed compiles");
    let (_file, mmap) = common::map(&bytes).expect("a compiled file is never empty");
    let consumer = Consumer::new(&mmap)
        .map_err(|error| error.to_string())
        .expect("the compiler wrote a file it can't read");
    common::exercise(&consumer);

    for (trip, source) in consumer.trips().iter().zip(&network.trips) {
        let stops = consumer.stop_times_by_trip(trip.index).len();
        let shapes = consumer.shapes_by_trip(trip.index);
        let names_a_shape = source
            .shape
            .is_some_and(|shape| shape < network.shapes.len());
        assert_eq!(
            !shapes.is_empty(),
            names_a_shape,
            "a trip has shapes exactly when it names a shape the feed has"
        );
        if shapes.is_empty() {
            continue;
        }
        assert_eq!(shapes.len(), stops - 1, "one shape for each pair of stops");
        for pair in shapes.windows(2) {
            let (first, second) = (
                consumer.shape_coordinates(pair[0]),
                consumer.shape_coordinates(pair[1]),
            );
            assert_eq!(
                first.last(),
                second.first(),
                "a shape ends where the next one starts, or the path has a gap"
            );
        }
        for &shape in shapes {
            let points = consumer.shape_coordinates(shape);
            let (first, last) = (points[0], points[points.len() - 1]);
            let chord = first.distance_meters(last);
            let length = f64::from(consumer.shape(shape).length.0);
            assert!(
                length + 0.5 >= chord,
                "a path is never shorter than the straight line between its ends"
            );
        }
    }

    let distinct: HashSet<Vec<(u64, u64)>> = (0..consumer.shapes().len())
        .map(|index| {
            consumer
                .shape_coordinates(index.into())
                .iter()
                .map(|point| {
                    let (latitude, longitude) = point.to_f64_pair();
                    (latitude.to_bits(), longitude.to_bits())
                })
                .collect()
        })
        .collect();
    assert_eq!(
        distinct.len(),
        consumer.shapes().len(),
        "two shapes with the same points are stored once"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn cut_shapes_join_into_one_path(network in network()) {
        check(&network);
    }
}
