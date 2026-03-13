use std::{io::Write, path::PathBuf};

use gtfs_bin::{compiler::Compiler, consumer::Consumer};
use memmap2::Mmap;
use tempfile::NamedTempFile;

// Helper to compile the GTFS and map it into memory.
// Returns the NamedTempFile alongside the Mmap so the file isn't deleted
// until the test finishes.
fn compile_test() -> (NamedTempFile, Mmap) {
    let mut input_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    input_path.push("tests/fixtures/gtfs.zip");

    let compiled_bytes = Compiler::new(input_path)
        .compile()
        .expect("Failed to compile GTFS");

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file.write_all(&compiled_bytes).unwrap();

    let mmap = unsafe { Mmap::map(temp_file.as_file()).unwrap() };
    (temp_file, mmap)
}

#[test]
fn test_routes_lookup_and_indexing() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    assert_eq!(consumer.routes.len(), 4);

    let route = consumer
        .route_by_id("route1")
        .expect("Route 'route1' not found");
    assert_eq!(consumer.route_id(route.id), "route1");

    let route_by_idx = consumer.route(route.idx);
    assert_eq!(route_by_idx.idx, route.idx);
    assert_eq!(consumer.route_id(route_by_idx.id), "route1");
}

#[test]
fn test_stops_lookup_and_hierarchy() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    assert_eq!(consumer.stops.len(), 15);

    let stop1 = consumer
        .stop_by_id("stop1")
        .expect("Stop 'stop1' not found");
    assert_eq!(consumer.stop_id(stop1.id), "stop1");

    let stop3 = consumer
        .stop_by_id("stop3")
        .expect("Stop 'stop3' not found");
    assert!(stop3.parent_idx.is_some());
    assert_eq!(stop3.parent_idx.get().unwrap(), stop1.idx);

    let coord = stop1
        .coordinate
        .get()
        .expect("Expected coordinate on stop1");
    assert_eq!(coord.lat_f64(), 48.796058);
    assert_eq!(coord.lon_f64(), 2.449386);
}

#[test]
fn test_trips_lookup_and_routing() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    assert_eq!(consumer.trips.len(), 6);

    let trip1 = consumer
        .trip_by_id("trip1")
        .expect("Trip 'trip1' not found");
    assert_eq!(consumer.trip_id(trip1.id), "trip1");

    let route1 = consumer.route_by_id("route1").unwrap();

    assert_eq!(trip1.route_idx, route1.idx);

    let route_trips: Vec<_> = consumer.iter_route_trips(route1.idx).collect();
    assert_eq!(route_trips.len(), 2);
    assert_eq!(route_trips[0].idx, trip1.idx);
}

#[test]
fn test_stop_times_and_trip_iterations() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    let trip1 = consumer.trip_by_id("trip1").unwrap();

    let stop_times = consumer.trip_stop_times(trip1.idx);
    assert_eq!(stop_times.len(), 2, "trip1 should have 2 stop times");

    let stop2 = consumer.stop_by_id("stop2").unwrap();
    let stop3 = consumer.stop_by_id("stop3").unwrap();

    assert_eq!(stop_times[0].stop_idx, stop2.idx);
    assert_eq!(stop_times[1].stop_idx, stop3.idx);

    let stop2_trips: Vec<_> = consumer.iter_stop_trips(stop2.idx).collect();
    assert_eq!(stop2_trips.len(), 2);
    assert_eq!(stop2_trips[0].idx, trip1.idx);

    let stop3_trips: Vec<_> = consumer.iter_stop_trips(stop3.idx).collect();
    assert_eq!(stop3_trips.len(), 1);
    assert_eq!(stop3_trips[0].idx, trip1.idx);
}

#[test]
fn test_routes_count() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    assert_eq!(consumer.routes.len(), 4);
}

#[test]
fn test_stops_count() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    assert_eq!(consumer.stops.len(), 15);
}

#[test]
fn test_trips_count() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    assert_eq!(consumer.trips.len(), 6);
}

#[test]
fn test_all_routes_iteration() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    let route_ids: Vec<String> = consumer
        .routes
        .iter()
        .map(|r| consumer.route_id(r.id).to_string())
        .collect();

    assert_eq!(route_ids.len(), 4);
    assert!(route_ids.contains(&"route1".to_string()));
    assert!(route_ids.contains(&"route2".to_string()));
    assert!(route_ids.contains(&"route3".to_string()));
    assert!(route_ids.contains(&"route4".to_string()));
}

#[test]
fn test_all_trips_iteration() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    let trip_ids: Vec<String> = consumer
        .trips
        .iter()
        .map(|t| consumer.trip_id(t.id).to_string())
        .collect();

    assert_eq!(trip_ids.len(), 6);
    for i in 1..=6 {
        assert!(trip_ids.contains(&format!("trip{}", i)));
    }
}

#[test]
fn test_route_to_trips_mapping() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    let route1 = consumer.route_by_id("route1").unwrap();
    let route2 = consumer.route_by_id("route2").unwrap();
    let route3 = consumer.route_by_id("route3").unwrap();
    let route4 = consumer.route_by_id("route4").unwrap();

    let route1_trips: Vec<_> = consumer.iter_route_trips(route1.idx).collect();
    let route2_trips: Vec<_> = consumer.iter_route_trips(route2.idx).collect();
    let route3_trips: Vec<_> = consumer.iter_route_trips(route3.idx).collect();
    let route4_trips: Vec<_> = consumer.iter_route_trips(route4.idx).collect();

    assert_eq!(route1_trips.len(), 2);
    assert_eq!(route2_trips.len(), 2);
    assert_eq!(route3_trips.len(), 1);
    assert_eq!(route4_trips.len(), 1);
}

#[test]
fn test_stop_to_trips_mapping() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    let stop2 = consumer.stop_by_id("stop2").unwrap();
    let stop6 = consumer.stop_by_id("stop6").unwrap();
    let stop8 = consumer.stop_by_id("stop8").unwrap();

    let stop2_trips: Vec<_> = consumer.iter_stop_trips(stop2.idx).collect();
    let stop6_trips: Vec<_> = consumer.iter_stop_trips(stop6.idx).collect();
    let stop8_trips: Vec<_> = consumer.iter_stop_trips(stop8.idx).collect();

    assert_eq!(stop2_trips.len(), 2);
    assert_eq!(stop6_trips.len(), 2);
    assert_eq!(stop8_trips.len(), 2);
}
