use std::{io::Write, path::PathBuf};

use gtfs_bin::{
    compiler::Compiler,
    consumer::{self, Consumer},
    models::{Date, Delay, Sentinel, ServiceIndex},
    rt::RealtimeBuilder,
};
use gtfs_rt::{
    FeedEntity, FeedHeader, FeedMessage, TripDescriptor, TripUpdate,
    trip_update::{StopTimeEvent, StopTimeUpdate},
};
use memmap2::Mmap;
use tempfile::NamedTempFile;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gtfs")
}

fn compile_fixture() -> Vec<u8> {
    Compiler::new(&fixture_path())
        .compile()
        .expect("the fixture is valid")
}

#[allow(
    unsafe_code,
    reason = "mapping a file is unsafe in memmap2, and this file is not modified"
)]
fn map(bytes: &[u8]) -> (NamedTempFile, Mmap) {
    let mut file = NamedTempFile::new().expect("a temp file can be created");
    file.write_all(bytes).expect("the temp file is writable");
    // SAFETY: the file is private to this test and isn't modified while mapped.
    let mmap = unsafe { Mmap::map(file.as_file()) }.expect("the temp file can be mapped");
    (file, mmap)
}

#[test]
fn compiling_twice_gives_identical_bytes() {
    assert_eq!(
        compile_fixture(),
        compile_fixture(),
        "output must not depend on hash map iteration order"
    );
}

#[test]
fn truncated_files_are_rejected_not_panicked_on() {
    let bytes = compile_fixture();

    for length in [0, 3, 8, 100, bytes.len() / 2, bytes.len() - 8] {
        let (_file, mmap) = map(&bytes[..length]);
        // An empty file can't be mapped on every platform, so it is covered by the length check.
        assert!(
            Consumer::new(&mmap).is_err(),
            "a file cut to {length} bytes must be an error"
        );
    }
}

#[test]
fn corrupted_files_never_panic() {
    let bytes = compile_fixture();
    // A small xorshift keeps the corruption the same on every run.
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for round in 0..200 {
        let mut corrupted = bytes.clone();
        // Hit the header often, because that is where the section offsets live.
        let limit = if round % 2 == 0 { 400 } else { corrupted.len() };
        for _ in 0..4 {
            let position = (next() % limit as u64) as usize;
            corrupted[position] = next().to_le_bytes()[0];
        }
        let (_file, mmap) = map(&corrupted);

        // Opening may succeed or fail, but it must not panic or read out of bounds.
        let _ = Consumer::new(&mmap);
    }
}

#[test]
fn wrong_magic_is_reported() {
    let mut bytes = compile_fixture();
    bytes[0] = b'X';
    let (_file, mmap) = map(&bytes);

    assert!(
        matches!(Consumer::new(&mmap), Err(consumer::Error::InvalidMagic)),
        "the magic number is checked first"
    );
}

#[test]
fn an_unknown_service_is_never_active() {
    let bytes = compile_fixture();
    let (_file, mmap) = map(&bytes);
    let consumer = Consumer::new(&mmap).expect("the fixture compiles to a valid file");

    assert!(
        !consumer.is_service_active(ServiceIndex::NONE, Date(0)),
        "a trip with no service must not panic"
    );
}

fn delay_feed(trip_id: &str, sequence: u32, delay: i32) -> FeedMessage {
    FeedMessage {
        header: FeedHeader {
            gtfs_realtime_version: "2.0".to_string(),
            ..Default::default()
        },
        entity: vec![FeedEntity {
            id: "update".to_string(),
            trip_update: Some(TripUpdate {
                trip: TripDescriptor {
                    trip_id: Some(trip_id.to_string()),
                    ..Default::default()
                },
                stop_time_update: vec![StopTimeUpdate {
                    stop_sequence: Some(sequence),
                    departure: Some(StopTimeEvent {
                        delay: Some(delay),
                        ..Default::default()
                    }),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        }],
    }
}

#[test]
fn delays_carry_to_later_stops_unless_turned_off() {
    let bytes = compile_fixture();
    let (_file, mmap) = map(&bytes);
    let consumer = Consumer::new(&mmap).expect("the fixture compiles to a valid file");
    let trip = consumer.trip_by_id("trip2").expect("the fixture has trip2");
    let stop_times = consumer.stop_times_by_trip(trip.index);
    assert!(
        stop_times.len() > 1,
        "the test needs a trip with later stops"
    );
    let first_sequence = stop_times[0].sequence;
    let last = trip.stop_times.start as usize + stop_times.len() - 1;

    let carried = RealtimeBuilder::new(&consumer)
        .build([delay_feed("trip2", first_sequence, 120)].into_iter());
    let not_carried = RealtimeBuilder::new(&consumer)
        .with_cascading_delays(false)
        .build([delay_feed("trip2", first_sequence, 120)].into_iter());

    assert_eq!(
        carried.stop_time_departure_delays()[last].as_option(),
        Some(Delay(120)),
        "later stops without an update take the last known delay"
    );
    assert_eq!(
        not_carried.stop_time_departure_delays()[last].as_option(),
        None,
        "with cascading off, later stops stay untouched"
    );
}
