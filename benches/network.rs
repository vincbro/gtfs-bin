//! Benchmarks for the parts a routing engine leans on. Run with `cargo bench`.

use std::{hint, io::Write};

use criterion::{Criterion, criterion_group, criterion_main};
use gtfs_bin::{
    compiler::Compiler,
    consumer::Consumer,
    rt::{Realtime, RealtimeBuilder},
};
use gtfs_rt::{
    FeedEntity, FeedMessage, TripDescriptor, TripUpdate,
    trip_update::{StopTimeEvent, StopTimeUpdate},
};
use memmap2::Mmap;

#[path = "../tests/common/synthetic.rs"]
mod synthetic;

fn compile(c: &mut Criterion) {
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    synthetic::write_feed(directory.path(), synthetic::Size::LARGE);
    let compiler = Compiler::new(directory.path());

    let mut group = c.benchmark_group("compile");
    group.sample_size(10);
    group.bench_function("one million stop times", |bencher| {
        bencher.iter(|| hint::black_box(compiler.compile().expect("a valid feed")));
    });
    group.finish();
}

#[allow(
    unsafe_code,
    reason = "mapping a file is unsafe in memmap2, and this file is not modified"
)]
fn consumer(c: &mut Criterion) {
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    synthetic::write_feed(directory.path(), synthetic::Size::LARGE);
    let bytes = Compiler::new(directory.path())
        .compile()
        .expect("a valid feed");
    let mut file = tempfile::NamedTempFile::new().expect("a temp file can be created");
    file.write_all(&bytes).expect("the temp file is writable");
    // SAFETY: the file is private to this benchmark and isn't modified while mapped.
    let mmap = unsafe { Mmap::map(file.as_file()) }.expect("the file maps");

    c.bench_function("open and validate", |bencher| {
        bencher.iter(|| hint::black_box(Consumer::new(&mmap).expect("a valid file")));
    });
    c.bench_function("open without validating", |bencher| {
        bencher.iter(|| hint::black_box(Consumer::new_unvalidated(&mmap).expect("a valid file")));
    });

    let consumer = Consumer::new(&mmap).expect("a valid file");
    c.bench_function("scan every stop time", |bencher| {
        bencher.iter(|| hint::black_box(scan(&consumer)));
    });
    c.bench_function("stops within 500 m", |bencher| {
        let mut found = Vec::new();
        bencher.iter(|| {
            consumer.stops_near(
                hint::black_box(59.35),
                hint::black_box(18.05),
                500.0,
                &mut found,
            );
            hint::black_box(found.len())
        });
    });
    c.bench_function("look up a trip by id", |bencher| {
        bencher.iter(|| hint::black_box(consumer.trip_by_id(hint::black_box("trip12345"))));
    });

    let message = update_feed();
    let builder = RealtimeBuilder::new(&consumer);
    let mut realtime = Realtime::new(&consumer);
    c.bench_function("apply 1000 trip updates", |bencher| {
        bencher
            .iter(|| hint::black_box(builder.apply([message.clone()].into_iter(), &mut realtime)));
    });
}

/// Adds up every departure time, the way a router's inner loop reads them.
fn scan(consumer: &Consumer) -> u64 {
    let mut seconds = 0_u64;
    for trip in consumer.trips() {
        for stop_time in consumer.stop_times_by_trip(trip.index) {
            seconds += u64::from(
                stop_time
                    .departure_time
                    .as_option()
                    .map_or(0, |time| time.0),
            );
        }
    }
    seconds
}

/// A feed that updates a thousand trips, two stops each.
fn update_feed() -> FeedMessage {
    let stop_update = |sequence| StopTimeUpdate {
        stop_sequence: Some(sequence),
        departure: Some(StopTimeEvent {
            delay: Some(60),
            ..Default::default()
        }),
        ..Default::default()
    };
    let entity = |number: u32| FeedEntity {
        id: number.to_string(),
        trip_update: Some(TripUpdate {
            trip: TripDescriptor {
                trip_id: Some(format!("trip{}", number * 7)),
                ..Default::default()
            },
            stop_time_update: vec![stop_update(1), stop_update(2)],
            ..Default::default()
        }),
        ..Default::default()
    };
    FeedMessage {
        header: gtfs_rt::FeedHeader {
            gtfs_realtime_version: "2.0".to_string(),
            ..Default::default()
        },
        entity: (0..1000).map(entity).collect(),
    }
}

criterion_group!(benches, compile, consumer);
criterion_main!(benches);
