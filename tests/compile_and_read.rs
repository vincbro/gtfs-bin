use std::{io::Write, path::PathBuf};

use gtfs_bin::{
    compiler::Compiler,
    consumer::Consumer,
    models::{Date, Weekday},
};
use memmap2::Mmap;
use tempfile::NamedTempFile;

// Helper to compile the GTFS and map it into memory.
// Returns the NamedTempFile alongside the Mmap so the file isn't deleted
// until the test finishes.
pub fn compile_test() -> (NamedTempFile, Mmap) {
    let mut input_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    input_path.push("tests/fixtures/gtfs");

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

    assert_eq!(consumer.trips.len(), 7);

    let trip1 = consumer
        .trip_by_id("trip1")
        .expect("Trip 'trip1' not found");
    assert_eq!(consumer.trip_id(trip1.id), "trip1");

    let route1 = consumer.route_by_id("route1").unwrap();

    assert_eq!(trip1.route_idx, route1.idx);

    let trips_in_route: Vec<_> = consumer.iter_trips_by_route(route1.idx).collect();
    assert_eq!(trips_in_route.len(), 2);
    assert_eq!(trips_in_route[0].idx, trip1.idx);
}

#[test]
fn test_stop_times_and_trip_iterations() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    let trip1 = consumer.trip_by_id("trip1").unwrap();

    let stop_times = consumer.stop_times_by_trip(trip1.idx);
    assert_eq!(stop_times.len(), 2, "trip1 should have 2 stop times");

    let stop2 = consumer.stop_by_id("stop2").unwrap();
    let stop3 = consumer.stop_by_id("stop3").unwrap();

    assert_eq!(stop_times[0].stop_idx, stop2.idx);
    assert_eq!(stop_times[1].stop_idx, stop3.idx);

    let stop2_trips: Vec<_> = consumer.iter_trips_by_stop(stop2.idx).collect();
    assert_eq!(stop2_trips.len(), 2);
    assert_eq!(stop2_trips[0].idx, trip1.idx);

    let stop3_trips: Vec<_> = consumer.iter_trips_by_stop(stop3.idx).collect();
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

    assert_eq!(consumer.trips.len(), 7);
}

#[test]
fn test_strings() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    let trip = consumer
        .trip_by_id("trip4")
        .expect("Failed to get trip4 by id");
    let headsign = trip
        .headsign
        .get()
        .expect("Failed to get trip4 headsign slice");

    assert_eq!("85088455", consumer.string(headsign));

    let trip = consumer
        .trip_by_id("trip1")
        .expect("Failed to get trip1 by id");
    let headsign = trip
        .headsign
        .get()
        .expect("Failed to get trip1 headsign slice");

    assert_eq!("85088452", consumer.string(headsign));

    let trip = consumer
        .trip_by_id("trip3")
        .expect("Failed to get trip3 by id");
    let headsign = trip
        .headsign
        .get()
        .expect("Failed to get trip3 headsign slice");

    assert_eq!("85088454", consumer.string(headsign));

    let route = consumer
        .route_by_id("route4")
        .expect("Failed to get route4 by id");
    let short_name = route
        .short_name
        .get()
        .expect("Failed to get route4 short name slice");

    assert_eq!("F1", consumer.string(short_name));
}

#[test]
fn test_shapes() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    let trip = consumer
        .trip_by_id("trip5")
        .expect("Failed to get trip5 by id");
    let shape = trip.shape.get().expect("Failed to get trip5 shape slice");

    let shapes = consumer.shapes(shape);
    assert_eq!(shapes.len(), 3);

    let dist = shapes[0]
        .distance_traveled
        .get()
        .expect("Failed to get dist traveled");
    assert_eq!(dist.0, 0.0);

    let dist = shapes[1]
        .distance_traveled
        .get()
        .expect("Failed to get dist traveled");
    assert_eq!(dist.0, 6.8310);

    let dist = shapes[2]
        .distance_traveled
        .get()
        .expect("Failed to get dist traveled");
    assert_eq!(dist.0, 15.8765);
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

    assert_eq!(trip_ids.len(), 7);
    for i in 1..=7 {
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

    let route1_trips: Vec<_> = consumer.iter_trips_by_route(route1.idx).collect();
    let route2_trips: Vec<_> = consumer.iter_trips_by_route(route2.idx).collect();
    let route3_trips: Vec<_> = consumer.iter_trips_by_route(route3.idx).collect();
    let route4_trips: Vec<_> = consumer.iter_trips_by_route(route4.idx).collect();

    assert_eq!(route1_trips.len(), 2);
    assert_eq!(route2_trips.len(), 3);
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

    let stop2_trips: Vec<_> = consumer.iter_trips_by_stop(stop2.idx).collect();
    let stop6_trips: Vec<_> = consumer.iter_trips_by_stop(stop6.idx).collect();
    let stop8_trips: Vec<_> = consumer.iter_trips_by_stop(stop8.idx).collect();

    assert_eq!(stop2_trips.len(), 2);
    assert_eq!(stop6_trips.len(), 3);
    assert_eq!(stop8_trips.len(), 3);
}

#[test]
fn test_transfers_count() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    assert_eq!(consumer.transfers.len(), 6);
}

#[test]
fn test_outbound_transfers() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    let stop5 = consumer.stop_by_id("stop5").unwrap();
    let stop10 = consumer.stop_by_id("stop10").unwrap();
    let stop8 = consumer.stop_by_id("stop8").unwrap();
    let stop2 = consumer.stop_by_id("stop2").unwrap();

    let outbound_from_stop5: Vec<_> = consumer.outbound_transfers_by_stop(stop5.idx).to_vec();
    assert_eq!(outbound_from_stop5.len(), 1);
    assert_eq!(
        consumer.stop_id(consumer.stop(outbound_from_stop5[0].to_stop_idx).id),
        "stop1"
    );
    assert_eq!(outbound_from_stop5[0].transfer_type, 0);

    let outbound_from_stop10: Vec<_> = consumer.outbound_transfers_by_stop(stop10.idx).to_vec();
    assert_eq!(outbound_from_stop10.len(), 1);
    assert_eq!(
        consumer.stop_id(consumer.stop(outbound_from_stop10[0].to_stop_idx).id),
        "stop6"
    );
    assert_eq!(outbound_from_stop10[0].transfer_type, 1);

    let outbound_from_stop8: Vec<_> = consumer.outbound_transfers_by_stop(stop8.idx).to_vec();
    assert_eq!(outbound_from_stop8.len(), 1);
    assert_eq!(
        consumer.stop_id(consumer.stop(outbound_from_stop8[0].to_stop_idx).id),
        "stop12"
    );
    assert_eq!(outbound_from_stop8[0].transfer_type, 2);

    let outbound_from_stop2: Vec<_> = consumer.outbound_transfers_by_stop(stop2.idx).to_vec();
    assert_eq!(outbound_from_stop2.len(), 1);
    assert_eq!(
        consumer.stop_id(consumer.stop(outbound_from_stop2[0].to_stop_idx).id),
        "stop9"
    );
    assert_eq!(outbound_from_stop2[0].transfer_type, 0);
}

#[test]
fn test_inbound_transfers() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    let stop1 = consumer.stop_by_id("stop1").unwrap();
    let stop6 = consumer.stop_by_id("stop6").unwrap();
    let stop12 = consumer.stop_by_id("stop12").unwrap();

    let inbound_to_stop1: Vec<_> = consumer.iter_inbound_transfers_by_stop(stop1.idx).collect();
    assert_eq!(inbound_to_stop1.len(), 1);
    assert_eq!(
        consumer.stop_id(consumer.stop(inbound_to_stop1[0].from_stop_idx).id),
        "stop5"
    );

    let inbound_to_stop6: Vec<_> = consumer.iter_inbound_transfers_by_stop(stop6.idx).collect();
    assert_eq!(inbound_to_stop6.len(), 1);
    assert_eq!(
        consumer.stop_id(consumer.stop(inbound_to_stop6[0].from_stop_idx).id),
        "stop10"
    );

    let inbound_to_stop12: Vec<_> = consumer
        .iter_inbound_transfers_by_stop(stop12.idx)
        .collect();
    assert_eq!(inbound_to_stop12.len(), 1);
    assert_eq!(
        consumer.stop_id(consumer.stop(inbound_to_stop12[0].from_stop_idx).id),
        "stop8"
    );
}

#[test]
fn test_services_and_calendar() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    assert_eq!(consumer.services.len(), 2, "Expected exactly 2 services");

    let service1 = consumer
        .service_by_id("service1")
        .expect("service1 not found");
    let service2 = consumer
        .service_by_id("service2")
        .expect("service2 not found");

    assert_eq!(service1.start_date.to_string(), "2017-01-01");
    assert_eq!(service1.end_date.to_string(), "2017-01-15");

    assert!(service1.weekdays.contains(Weekday::SATURDAY));
    assert!(service1.weekdays.contains(Weekday::SUNDAY));
    assert!(!service1.weekdays.contains(Weekday::MONDAY));

    assert_eq!(service2.start_date.to_string(), "2017-01-01");
    assert_eq!(service2.end_date.to_string(), "2017-01-07");
    assert!(!service2.weekdays.contains(Weekday::SUNDAY));

    assert!(
        !consumer.is_service_active(service1.idx, Date(service1.start_date.0)),
        "service1 should be removed on 2017-01-01"
    );
    assert!(
        !consumer.is_service_active(service1.idx, Date(service1.start_date.0 + 1)),
        "service1 should be inactive on 2017-01-02"
    );
    assert!(
        consumer.is_service_active(service1.idx, Date(service1.start_date.0 + 6)),
        "service1 should be active on 2017-01-07 (Sat)"
    );
    assert!(
        consumer.is_service_active(service2.idx, Date(service2.start_date.0)),
        "service2 should be added on 2017-01-01"
    );
    assert!(
        !consumer.is_service_active(service2.idx, Date(service2.start_date.0 + 1)),
        "service2 should be inactive on 2017-01-02"
    );
    assert!(
        consumer.is_service_active(service2.idx, Date(service2.start_date.0 + 6)),
        "service2 should be inactive on 2017-01-07"
    );
}

#[test]
fn test_trip_patterns() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    assert_eq!(consumer.trip_patterns.len(), 6);

    let trip3 = consumer.trip_by_id("trip3").expect("trip3 not found");
    let trip7 = consumer.trip_by_id("trip7").expect("trip7 not found");

    let pattern_idx_3 = consumer.trip_to_trip_pattern[trip3.idx.as_usize()];
    let pattern_idx_7 = consumer.trip_to_trip_pattern[trip7.idx.as_usize()];

    assert_eq!(
        pattern_idx_3, pattern_idx_7,
        "trip3 and trip7 should share a pattern"
    );

    let trips_in_pattern: Vec<_> = consumer
        .iter_trips_in_trip_pattern(pattern_idx_3)
        .map(|t| consumer.trip_id(t.id))
        .collect();

    assert_eq!(trips_in_pattern.len(), 2);
    assert!(trips_in_pattern.contains(&"trip3"));
    assert!(trips_in_pattern.contains(&"trip7"));

    let stops_in_pattern: Vec<_> = consumer
        .iter_stop_sequence_by_trip_pattern(pattern_idx_3)
        .map(|s| consumer.stop_id(s.id))
        .collect();

    assert_eq!(stops_in_pattern.len(), 3);
    assert!(stops_in_pattern.contains(&"stop6"));
    assert!(stops_in_pattern.contains(&"stop7"));
    assert!(stops_in_pattern.contains(&"stop8"));

    let trip1 = consumer.trip_by_id("trip1").expect("trip1 not found");
    let pattern_idx_1 = consumer.trip_to_trip_pattern[trip1.idx.as_usize()];

    let trips_in_pattern_1: Vec<_> = consumer
        .iter_trips_in_trip_pattern(pattern_idx_1)
        .map(|t| consumer.trip_id(t.id))
        .collect();

    assert_eq!(trips_in_pattern_1.len(), 1);
    assert_eq!(trips_in_pattern_1[0], "trip1");

    let stop6 = consumer.stop_by_id("stop6").expect("stop6 not found");
    let patterns: Vec<_> = consumer.iter_trip_patterns_by_stop(stop6.idx).collect();
    assert_eq!(2, patterns.len())
}
