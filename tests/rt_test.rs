use gtfs_bin::{
    compiler::Compiler,
    consumer::Consumer,
    models::Delay,
    rt::{RealtimeBuilder, TripStatus},
};
use gtfs_rt::{
    FeedEntity, FeedHeader, FeedMessage, TripDescriptor, TripUpdate, trip_descriptor,
    trip_update::{self, StopTimeUpdate},
};
use memmap2::Mmap;
use std::{
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
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

/// Helper to generate a fake GTFS-RT feed message.
/// Now accepts a `target_sequence` so we don't have to guess what sequence the fixture used.
pub fn create_fake_feed_message(trip1_target_seq: u32) -> FeedMessage {
    let current_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    FeedMessage {
        header: FeedHeader {
            gtfs_realtime_version: "2.0".to_string(),
            timestamp: Some(current_timestamp),
            ..Default::default()
        },
        entity: vec![
            // --- CANCEL TRIP 4 ---
            FeedEntity {
                id: "fake_cancellation_event".to_string(),
                trip_update: Some(TripUpdate {
                    trip: TripDescriptor {
                        trip_id: Some("trip4".to_string()),
                        schedule_relationship: Some(
                            trip_descriptor::ScheduleRelationship::Canceled as i32,
                        ),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                ..Default::default()
            },
            FeedEntity {
                id: "fake_delay_event".to_string(),
                trip_update: Some(TripUpdate {
                    trip: TripDescriptor {
                        trip_id: Some("trip1".to_string()),
                        schedule_relationship: Some(
                            trip_descriptor::ScheduleRelationship::Scheduled as i32,
                        ),
                        ..Default::default()
                    },
                    stop_time_update: vec![StopTimeUpdate {
                        stop_sequence: Some(trip1_target_seq),
                        arrival: Some(trip_update::StopTimeEvent {
                            delay: Some(120),
                            ..Default::default()
                        }),
                        departure: Some(trip_update::StopTimeEvent {
                            delay: Some(120),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            },
        ],
    }
}

#[test]
fn test_realtime_builder_ingestion() {
    let (_file, mmap) = compile_test();
    let consumer = Consumer::new(&mmap).expect("Failed to load graph");

    let trip1 = consumer
        .trip_by_id("trip1")
        .expect("Trip 'trip1' not found");
    let stop_times = consumer.stop_times_by_trip(trip1.idx);

    let target_seq = stop_times[0].sequence;

    let feed_message = create_fake_feed_message(target_seq);
    let realtime = RealtimeBuilder::new(&consumer)
        .with_cascading_delays(true)
        .build(vec![feed_message].into_iter());

    // --- Assert Trip 4 is Canceled ---
    let trip4 = consumer
        .trip_by_id("trip4")
        .expect("Trip 'trip4' not found");
    assert_eq!(
        realtime.trip_status[trip4.idx.as_usize()],
        TripStatus::Cancled,
        "Trip 4 should be marked as canceled"
    );

    // --- Assert Trip 1 is Delayed ---
    let seq_idx = stop_times
        .binary_search_by_key(&target_seq, |st| st.sequence)
        .expect("Sequence not found in binary search");

    let global_idx = trip1.stop_times.start as usize + seq_idx;

    let arrival_delay = realtime.stop_time_arrival_delays[global_idx];
    assert!(arrival_delay.is_some(), "Expected an arrival delay");
    assert_eq!(arrival_delay.get().unwrap(), Delay(120));

    let departure_delay = realtime.stop_time_departure_delays[global_idx];
    assert!(departure_delay.is_some(), "Expected a departure delay");
    assert_eq!(departure_delay.get().unwrap(), Delay(120));

    // --- Assert Trip 3 remains Unchanged ---
    let trip3 = consumer
        .trip_by_id("trip3")
        .expect("Trip 'trip3' not found");
    assert_eq!(
        realtime.trip_status[trip3.idx.as_usize()],
        TripStatus::Unchanged,
        "Trip 3 was not in the feed and should be unchanged"
    );
}
