# gtfs-bin

*gtfs-bin* is a tool and library for compiling standard GTFS data into highly optimized, memory-mappable binary files. 

It provides the data structures and interfaces needed to build fast transit applications, such as routing engines or spatial analytics tools, with virtually instant startup times and low memory overhead.

## The Architecture

Working with `gtfs-bin` is split into two distinct phases: **Compilation** and **Consumption**. This separation of concerns allows you to do the heavy lifting of parsing data once, and distribute a highly optimized artifact to your actual applications.

### 1. The Builder API (Compilation)
During the build phase, the `Compiler` ingests raw GTFS text files (from a directory or `.zip`). It performs several optimization passes:
- Reads the feed with its own parser: a directory or a `.zip` (also one that holds a folder), byte order marks, Windows line endings, quoted fields and columns in any order. A bad value is an error that names the file, line and column. Rows that point at an unknown route, trip or stop are skipped, and the first of a repeated id wins. Stops with no times between two that have them get times spread evenly, by distance along the shape when the feed has it. `Compiler::lenient()` skips rows with a bad value instead of failing, and `compile_with_report` says which.
- Resolves all string-based IDs into integer indices to establish direct, pointer-free relationships (e.g., linking a `Trip` to its `Route`).
- Deduplicates and packs all textual data (like stop names and headsigns) into a single contiguous string block.
- Writes the finalized, flat relational arrays into a single `.gtfs` binary artifact. The same feed always compiles to the same bytes.

### 2. The Consumer API (Consumption)
In your actual transit application, you use the `Consumer` to read the data. Instead of parsing files into memory, the application uses the operating system's `mmap` to map the compiled `.gtfs` file directly into virtual memory. 

**What you can expect:**
- **Instant Startup:** The network loads in the time it takes the OS to set up a memory map (microseconds), bypassing traditional I/O and deserialization bottlenecks.
- **Low Memory Footprint:** Data is lazily paged into physical RAM by the OS only when you actively query it.
- **Safe on any file:** `Consumer::new` checks the header, every section, that all text is UTF-8, and that every index and slice in every table points inside the table it names. A corrupt or hostile file is an error, never undefined behavior, and the checks are covered by property tests. Opening SL's whole regional feed (159 MB, 2.5 million stop times) takes 39 ms with the checks. `Consumer::verify_checksum` also detects any changed byte.
- **Cache Locality:** Because the data is strictly flat and pointer-free, traversing relationships (like iterating over all trips on a route) is highly predictable for CPU caches.

## Supported Data Models

The compiled binary graph currently supports the following GTFS entities:
- Stops (including stations and entrances)
- Routes
- Trips
- StopTimes
- Shapes
- Services (Calendar & Calendar Dates)
- Transfers
- Trip Patterns (Grouped stop sequences)
- Pickup and drop-off rules, trip direction and block, trip-specific transfers
- The agency time zone and the feed's validity dates
- A grid index for stops near a position, and name search that ignores case and accents
- Coordinates (Scaled to 7 decimal places for ~1.1cm precision)

## Cargo features

| Feature | Adds |
|---|---|
| `compiler` | `Compiler`, which reads a feed and writes the binary file |
| `consumer` | `Consumer`, which maps and reads it (needs `memmap2`) |
| `rt` | GTFS-realtime on top of a consumer (needs `consumer`) |
| `cli` | the `gtfs-bin` command and the `inspect` example |

All four are on by default. Libraries that only read files can use `default-features = false, features = ["consumer"]`.

## Usage

`gtfs-bin` can be used as a standalone CLI tool to compile data, or integrated directly into your Rust project as a library.

### As a CLI Tool

You can compile a standard GTFS zip file or directory into a `.gtfs` binary using the included executable:

```bash
cargo run --release -- -i path/to/gtfs.zip -o output.gtfs
```

### As a Library

Add `gtfs-bin` to your `Cargo.toml`:

```toml
[dependencies]
gtfs-bin = "0.2"
memmap2 = "0.9"
```

#### Phase 1: Compiling Data

If you are building a data pipeline, use the `Compiler` to generate the `.gtfs` binary format.

```rust
use gtfs_bin::compiler::Compiler;
use std::{fs, path::Path};

fn main() {
    // Read from a zip file or an extracted directory
    let compiler = Compiler::new(Path::new("path/to/gtfs.zip"));
    
    // Parse the data and generate the binary byte array
    let bytes = compiler.compile().expect("Failed to compile GTFS");

    // Save the artifact to disk
    fs::write("network.gtfs", &bytes).unwrap();
}
```

#### Phase 2: Consuming Data

In your routing engine or API, map the compiled file and pass it to the `Consumer`.

```rust
use gtfs_bin::consumer::Consumer;
use memmap2::MmapOptions;
use std::fs::File;

fn main() {
    let file = File::open("network.gtfs").expect("Failed to open binary");
    
    // Memory-map the file
    let mmap = unsafe { MmapOptions::new().map(&file).expect("Failed to map memory") };
    
    // Instantly load the entire transit network
    let consumer = Consumer::new(&mmap).expect("Failed to parse binary header");

    // Example: Look up a route by its original GTFS text ID
    if let Some(route) = consumer.route_by_id("route_1") {
        let route_name = consumer.string(route.short_name.as_option().unwrap());
        println!("Found route: {}", route_name);
        
        // Iterate over all trips belonging to this route
        for trip in consumer.iter_trips_by_route(route.index) {
            println!("Trip ID: {}", consumer.trip_id(trip.id));
        }
    }
}
```

Finding stops by position or by name:

```rust
// Fills a buffer you keep between calls, nearest first, with the distance in meters.
let mut near = Vec::new();
consumer.stops_near(59.3303, 18.0586, 250.0, &mut near);

// Case and accents are ignored, so "sodermalmstorg" finds "Södermalmstorg".
let mut found = Vec::new();
consumer.search_by_name("centralen", &mut found);

// Check the file is byte for byte what the compiler wrote (reads all of it).
consumer.verify_checksum()?;
```

#### Phase 3: GTFS Realtime (Optional)

If you enable the `rt` feature, you can apply GTFS-RT `FeedMessage` updates on top of your static consumer graph. A `Realtime` has one slot per stop time and per trip of its consumer, so reading it is an array index.

```rust
use gtfs_bin::{models::Date, rt::{Realtime, RealtimeBuilder}};

// `feed_messages` is an Iterator of parsed gtfs_rt::FeedMessage.
let builder = RealtimeBuilder::new(&consumer)
    // A stop time without an update takes the delay of the stop before it (on by default).
    .with_cascading_delays(true)
    // Stop times are local times, so delays computed from timestamps need the agency offset.
    .with_utc_offset_seconds(3600)
    .with_preferred_language("sv");

// One state per service day, so a trip that runs past midnight can't mix up two days.
let mut today = Realtime::for_service_date(&consumer, Date::parse_gtfs("20240115").unwrap());
let report = builder.apply(feed_messages, &mut today);
println!("{report:?}"); // how many updates matched, were stale, or named unknown trips

for stop_time in consumer.stop_times_by_trip(trip.index) {
    // The scheduled time with the live delay applied, or `None` if the stop is skipped.
    let departure = today.departure_time(stop_time);
}
```

What is supported:
- **Trip updates**, matched by `trip_id`, or by route, start time and date when there is no id. A start time that fits two trips is reported as ambiguous and applied to neither.
- **Stop time updates**, matched by `stop_sequence` or by `stop_id`, with delays from `delay` or computed from `time`. `Skipped` and `NoData` follow the spec, and each trip update replaces what the trip had before.
- **Trip status:** canceled, deleted, added and unscheduled, and the trip-level delay.
- **Added trips** that aren't in the static data, kept in `Realtime::added_trips`.
- **Vehicle positions** with the current stop, bearing, speed and occupancy.
- **Alerts**, resolved to routes, stops and trips, with `active_alerts(time)`.
- **Stale and deleted data:** older timestamps are ignored, `is_deleted` removes an entity, and a full dataset replaces the kinds of entity it contains.

Delays computed from a timestamp use the agency time zone stored in the file, so they stay exact on the days the clocks change.

`rt::decode_feed` decodes the bytes a publisher serves, so you don't need a protobuf crate of your own. A `Realtime` is plain data and is `Send + Sync`: to read it from many threads while a feed is applied, apply to a second copy and swap an `Arc`.

What is not: shapes and stops sent in a feed, trip modifications, and replaced or duplicated trips. They are counted as `unsupported` in the report. See [TODO.md](TODO.md).

## Development

The `rt` feature needs `protoc` installed, because `gtfs-rt` compiles its protobuf files when it builds (`brew install protobuf` or `apt install protobuf-compiler`).

Everything runs through [`just`](https://github.com/casey/just). `just toolchain` installs the pinned Rust, the nightly `rustfmt`, the cargo tools and the git hooks, and `just check` is the same gate CI runs. `just --list` shows the rest: `compile`, `inspect`, `apply`, `bench`, `real-data` and `check-full`.

## Roadmap

  - [x] Flat, relational array data structures
  - [x] Compiler API for building GTFS feeds
  - [x] Consumer API for memory-mapped reading
  - [x] GTFS Realtime data ingestion and mapping
  - [x] Own GTFS parser, with no GTFS library dependency
  - [x] Validation of every table when a file is opened, with property tests
  - [ ] Expose prefetching (`madvise`) helper functions for pathfinding loops
  - [ ] Multi-threaded graph compilation API

## References

  - [GTFS Specification](https://gtfs.org/documentation/schedule/reference/)
  - [GTFS Realtime Specification](https://gtfs.org/documentation/realtime/reference/)
  - [bytemuck](https://docs.rs/bytemuck/latest/bytemuck/)
  - [memmap2](https://docs.rs/memmap2/latest/memmap2/)
  - [mmap](https://en.wikipedia.org/wiki/Mmap)
  - [blaise](https://github.com/vincbro/blaise) - A high-performance transit routing engine (soon to be) built on top of `gtfs-bin`.

## License

Licensed under either of

  - Apache License, Version 2.0 ([LICENSE-APACHE](https://www.google.com/search?q=LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
  - MIT license ([LICENSE-MIT](https://www.google.com/search?q=LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
