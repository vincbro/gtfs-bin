//! A generated transit network, timed at growing sizes.
//!
//! Each phase is its own group with one entry per size, named by how many stop times the network
//! has, and reports the time of a single action: compiling the feed, opening the file, scanning
//! its stop times, one lookup, or applying a realtime feed. The networks are generated, so the
//! numbers repeat and nothing real is committed. Generating them, writing them to disk and
//! mapping the file are outside the timer.

use std::{hint, io::Write};

use criterion::{BenchmarkId, Criterion, Throughput, measurement::WallTime};
use gtfs_bin::{
    compiler::Compiler,
    consumer::Consumer,
    rt::{Realtime, RealtimeBuilder},
};
use gtfs_rt::{
    FeedEntity, FeedHeader, FeedMessage, TripDescriptor, TripUpdate,
    trip_update::{StopTimeEvent, StopTimeUpdate},
};
use memmap2::Mmap;
use tempfile::{NamedTempFile, TempDir};

// The generator the tests use, so a benchmark and a test see the same network.
#[path = "../tests/common/synthetic.rs"]
mod synthetic;

use synthetic::Size;

/// Network sizes: about 12 thousand, 120 thousand and 1.2 million stop times.
const SIZES: [Size; 3] = [Size::SMALL, Size::MEDIUM, Size::LARGE];

/// How many trips the realtime feed updates, two stops each.
const UPDATED_TRIPS: u32 = 1000;

/// A feed of the given size, written to a folder.
fn feed(size: Size) -> TempDir {
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    synthetic::write_feed(directory.path(), size);
    directory
}

/// A compiled file on disk and mapped, so a phase can open it as often as it likes.
struct Compiled {
    mmap: Mmap,
    /// Kept so the file outlives the map.
    _file: NamedTempFile,
}

#[allow(
    unsafe_code,
    reason = "mapping a file is unsafe in memmap2, and this file is not modified"
)]
fn compiled(size: Size) -> Compiled {
    let feed = feed(size);
    let bytes = Compiler::new(feed.path())
        .compile()
        .expect("a generated feed is valid");
    let mut file = NamedTempFile::new().expect("a temp file can be created");
    file.write_all(&bytes).expect("the temp file is writable");
    // SAFETY: the file is private to this benchmark and isn't modified while mapped.
    let mmap = unsafe { Mmap::map(file.as_file()) }.expect("the file maps");

    Compiled { mmap, _file: file }
}

/// Runs `measure` for every size, in a group of its own.
fn phase(
    criterion: &mut Criterion,
    name: &str,
    measure: impl Fn(&mut criterion::BenchmarkGroup<'_, WallTime>, Size, &Consumer<'_>),
) {
    let mut group = criterion.benchmark_group(name);

    for size in SIZES {
        let compiled = compiled(size);
        let consumer = Consumer::new(&compiled.mmap).expect("the compiler wrote a valid file");
        measure(&mut group, size, &consumer);
    }

    group.finish();
}

/// Reads the feed's text files and writes the file. Slow, so it takes fewer samples.
fn compile(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("compile");
    group.sample_size(10);

    for size in SIZES {
        let feed = feed(size);
        let compiler = Compiler::new(feed.path());

        group.throughput(Throughput::Elements(size.stop_times() as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(size.stop_times()),
            &compiler,
            |bencher, compiler| {
                bencher.iter_with_large_drop(|| compiler.compile().expect("a generated feed"));
            },
        );
    }

    group.finish();
}

/// Opens the mapped file and checks every table, which is what `Consumer::new` costs.
fn open(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("open");

    for size in SIZES {
        let compiled = compiled(size);

        group.throughput(Throughput::Bytes(compiled.mmap.len() as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(size.stop_times()),
            &compiled.mmap,
            |bencher, mmap| {
                bencher.iter(|| hint::black_box(Consumer::new(mmap).expect("a valid file")));
            },
        );
    }

    group.finish();
}

/// Opens the file with the table checks left out, the cost of the header and the sections alone.
fn open_unvalidated(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("open_unvalidated");

    for size in SIZES {
        let compiled = compiled(size);

        group.bench_with_input(
            BenchmarkId::from_parameter(size.stop_times()),
            &compiled.mmap,
            |bencher, mmap| {
                bencher.iter(|| {
                    hint::black_box(Consumer::new_unvalidated(mmap).expect("a valid file"))
                });
            },
        );
    }

    group.finish();
}

/// Adds up every departure time, the way a router's inner loop reads them.
fn scan_stop_times(criterion: &mut Criterion) {
    phase(criterion, "scan", |group, size, consumer| {
        group.throughput(Throughput::Elements(size.stop_times() as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(size.stop_times()),
            consumer,
            |bencher, consumer| {
                bencher.iter(|| hint::black_box(departure_seconds(consumer)));
            },
        );
    });
}

fn departure_seconds(consumer: &Consumer<'_>) -> u64 {
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

/// Looks a trip up by its id, a binary search over the sorted ids.
fn trip_by_id(criterion: &mut Criterion) {
    phase(criterion, "trip_by_id", |group, size, consumer| {
        let id = format!("trip{}", size.trips / 2);

        group.bench_with_input(
            BenchmarkId::from_parameter(size.stop_times()),
            &id,
            |bencher, id| {
                bencher.iter(|| hint::black_box(consumer.trip_by_id(id)));
            },
        );
    });
}

/// Finds the stops within 500 meters of a point, into a buffer that is reused.
fn stops_near(criterion: &mut Criterion) {
    phase(criterion, "stops_near", |group, size, consumer| {
        let mut found = Vec::new();

        group.bench_function(BenchmarkId::from_parameter(size.stop_times()), |bencher| {
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
    });
}

/// Looks a place up by name, which reads every name.
fn search_by_name(criterion: &mut Criterion) {
    phase(criterion, "search_by_name", |group, size, consumer| {
        let mut found = Vec::new();

        group.bench_function(BenchmarkId::from_parameter(size.stop_times()), |bencher| {
            bencher.iter(|| {
                consumer.search_by_name(hint::black_box("stop number 7"), &mut found);
                hint::black_box(found.len())
            });
        });
    });
}

/// A feed that updates `UPDATED_TRIPS` trips, two stops each.
fn update_feed(size: Size) -> FeedMessage {
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
                trip_id: Some(format!("trip{}", number as usize * 7 % size.trips)),
                ..Default::default()
            },
            stop_time_update: vec![stop_update(1), stop_update(2)],
            ..Default::default()
        }),
        ..Default::default()
    };

    FeedMessage {
        header: FeedHeader {
            gtfs_realtime_version: "2.0".to_owned(),
            ..Default::default()
        },
        entity: (0..UPDATED_TRIPS).map(entity).collect(),
    }
}

/// Applies a realtime feed. Each update replaces what its trip had, so applying the same feed
/// again does the same work.
fn apply_realtime(criterion: &mut Criterion) {
    phase(criterion, "apply_realtime", |group, size, consumer| {
        let builder = RealtimeBuilder::new(consumer);
        let message = update_feed(size);
        let mut realtime = Realtime::new(consumer);

        group.throughput(Throughput::Elements(u64::from(UPDATED_TRIPS)));

        group.bench_function(BenchmarkId::from_parameter(size.stop_times()), |bencher| {
            bencher.iter(|| {
                hint::black_box(builder.apply([message.clone()].into_iter(), &mut realtime))
            });
        });
    });
}

criterion::criterion_group!(
    benches,
    compile,
    open,
    open_unvalidated,
    scan_stop_times,
    trip_by_id,
    stops_near,
    search_by_name,
    apply_realtime
);

criterion::criterion_main!(benches);
