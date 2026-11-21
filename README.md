# 📦 flatnet

[![Crates.io](https://img.shields.io/crates/v/flatnet.svg)](https://crates.io/crates/flatnet)
[![Documentation](https://docs.rs/flatnet/badge.svg)](https://docs.rs/flatnet)
[![License](https://img.shields.io/crates/l/flatnet.svg)](LICENSE-MIT)

A transit network as one flat file you can memory-map.

*flatnet* is a file format, a tool and a library. It reads a transit feed once and writes a single `.flnt` file, and your application reads that file in place, with nothing to parse at startup. GTFS is the first source it reads, and the format is its own: it holds what is worth having to work with a network, not everything a source can say. It is the data layer under [blaise](https://github.com/vincbro/blaise), a transit router, and it suits any fast transit application: routing engines, spatial analytics, stop search.

- No parsing at startup. The network opens in the time the OS takes to set up a memory map.
- Safe on any file. A corrupt or hostile file is an error, never undefined behavior.
- Easy and fast at any size: SL's regional network (2.5 million stop times) opens and validates in about 10 ms.
- Real-time delays, cancellations and skipped stops, when you have a GTFS-realtime feed.

Files end in `.flnt`, and begin with the bytes `FLNT`.

## How it works

Working with flatnet is split into two phases, **compilation** and **consumption**. The heavy lifting happens once, and the result is what you distribute.

- **Compile.** The `Compiler` reads a GTFS feed, a directory or a `.zip`, with its own parser. String ids become integer indices, so a trip points straight at its route. Text is deduplicated into one contiguous block. The flat arrays are written to one file, and the same feed always compiles to the same bytes.
- **Consume.** The `Consumer` maps the file with `mmap` and reads it where it lies. Data is paged into RAM by the OS only when you touch it, and because the tables are flat and pointer-free, walking from a route to its trips is friendly to the CPU cache.
- **Validated.** `Consumer::new` checks the header, every section, that all text is UTF-8, and that every index and slice points inside the table it names. The checks are covered by property tests. Opening SL's whole regional feed (54 MB, 2.5 million stop times) takes about 10 ms with them. `Consumer::verify_checksum` also detects any changed byte.
- **Live.** With the `realtime` feature, updates from a realtime source are applied on top of a consumer (the `gtfs-rt` feature decodes GTFS-realtime into them), and reading a delay is an array index.

The documentation is in [docs/](docs/README.md), and the layout of the file in [docs/file-format.md](docs/file-format.md).

## Installation

```bash
cargo install flatnet-cli   # the `flatnet` command
cargo add flatnet-compiler  # write a file from your own code
cargo add flatnet-core      # read one
cargo add memmap2
```

### Crates

| Crate | What it is |
|---|---|
| `flatnet-cli` | the `flatnet` command, which compiles a feed |
| `flatnet-compiler` | `Compiler`, which turns a `Network` into the binary file, and with the `gtfs` feature `gtfs::Reader`, which reads a GTFS feed into one |
| `flatnet-core` | the types of the file, `Consumer`, which maps and reads it, and GTFS-realtime on top of it |
| `flatnet-viewer` | an egui app to walk a compiled file and see it on a map (`just view`). Not published |

`flatnet-core` has three features, all on by default: `consumer` (needs `memmap2`), `realtime` (applies source-neutral updates, needs `consumer`) and `gtfs-rt` (decodes GTFS-realtime into those updates, needs `realtime`). A library that only reads files can turn the last two off with `default-features = false, features = ["consumer"]`.

`flatnet-compiler` has one, `gtfs`, on by default: it adds `gtfs::Reader`, which reads a GTFS feed into a `Network`. Without it the compiler has no source of its own and takes a `Network` that you build, which also leaves out the `csv` and `zip` dependencies.

## Quick start

Compile your feed once, with the command line:

```bash
flatnet -i path/to/gtfs.zip -o network.flnt
```

Add your own metadata to the file with `--meta key=value` (repeat it) or `--meta-json file.json`.

Or from Rust, if you are building a data pipeline:

```rust
use std::{fs, path::Path};

use flatnet_compiler::Compiler;

fn main() {
    // Read from a zip file or an extracted directory.
    let bytes = Compiler::new()
        .compile_gtfs(Path::new("path/to/gtfs.zip"))
        .expect("Failed to compile GTFS");
    fs::write("network.flnt", &bytes).unwrap();
}
```

Then map the file and read it:

```rust
use std::fs::File;

use flatnet_core::consumer::Consumer;
use memmap2::Mmap;

fn main() {
    let file = File::open("network.flnt").expect("Failed to open binary");
    // SAFETY: the file is only read, and nothing modifies it while it is mapped.
    let mmap = unsafe { Mmap::map(&file).expect("Failed to map memory") };
    let consumer = Consumer::new(&mmap).expect("Failed to open the network");

    // Look up a route by its original GTFS id.
    if let Some(route) = consumer.route_by_id("route_1") {
        let route_name = consumer.string(route.short_name.as_option().unwrap());
        println!("Found route: {route_name}");

        // Iterate over all trips belonging to this route.
        for trip in consumer.iter_trips_by_route(route.index) {
            println!("Trip ID: {}", consumer.trip_id(trip.id));
        }
    }
}
```

### Finding stops

By position or by name:

```rust
// Fills a buffer you keep between calls, nearest first, with the distance in meters.
let mut near = Vec::new();
consumer.stops_near(59.3303, 18.0586, 250.0, &mut near);

// Case and accents are ignored, so "sodermalmstorg" finds "Södermalmstorg".
let mut found = Vec::new();
consumer.search_by_name("centralen", &mut found);

// Check the file is byte for byte what the compiler wrote (reads all of it).
consumer.verify_checksum()?;

// Is the file out of date? Two integer comparisons, so ask on every launch.
if consumer.is_stale(today) {
    eprintln!("no service runs after {:?}", consumer.service_range());
}
```

### Metadata

The file says where it came from. `consumer.metadata()` holds a fixed set of fields: when it was compiled, the first and last day any service runs, the compiler version, the name of the source, and the publisher, language, version and contact from `feed_info.txt` and the agency's name and URL. `consumer.metadata_json()` gives back, as text, whatever JSON object the compiler was given with `--meta` or `--meta-json`, or `Compiler::metadata_json` and `metadata_entry`. The compiler reads neither the clock nor the path, so the compile time and the source name are only recorded when they are passed to `Compiler::compiled_at` and `Compiler::source` (the command does). That keeps one feed giving one file.

### Compiling a messy feed

The parser reads byte order marks, Windows line endings, quoted fields and columns in any order, and a `.zip` that holds a folder. A bad value is an error that names the file, line and column. Rows that point at an unknown route, trip or stop are skipped, and the first of a repeated id wins. Stops with no times between two that have them get times spread evenly, by distance along the shape when the feed has it.

`gtfs::Reader::lenient()` skips rows with a bad value instead of failing, and `read_with_report` says which. Compile what it read with `Compiler::compile`.

### Realtime

With the `gtfs-rt` feature of `flatnet-core`, apply GTFS-RT `FeedMessage` updates on top of a consumer. A `Realtime` has one slot per stop time and per trip, so reading it is an array index.

```rust
use flatnet_core::{models::Date, rt::{Realtime, RealtimeBuilder}};

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

let first = trip.stop_times.start as usize;
for (position, timing) in consumer.timings_by_trip(trip.index).iter().enumerate() {
    // The scheduled time with the live delay applied, or `None` if the stop is skipped.
    let departure = today.departure_time(trip.index, StopTimeIndex::from(first + position), *timing);
}
```

`rt::decode_feed` decodes the bytes a publisher serves, so you don't need a protobuf crate of your own. A `Realtime` is plain data and is `Send + Sync`: to read it from many threads while a feed is applied, put it in a `rt::SharedRealtime`: readers take `current()`, and `update` applies a feed to a copy and swaps it in.

What is supported:

- **Trip updates**, matched by `trip_id`, or by route, start time and date when there is no id. A start time that fits two trips is reported as ambiguous and applied to neither.
- **Stop time updates**, matched by `stop_sequence` or by `stop_id`, with delays from `delay` or computed from `time`. `Skipped` and `NoData` follow the spec, and each trip update replaces what the trip had before.
- **Trip status:** canceled, deleted, added and unscheduled, and the trip-level delay.
- **Added trips** that aren't in the static data, kept in `Realtime::added_trips`.
- **Vehicle positions** with the current stop, bearing, speed and occupancy.
- **Alerts**, resolved to routes, stops and trips, with `active_alerts(time)`.
- **Stale and deleted data:** older timestamps are ignored, `is_deleted` removes an entity, and a full dataset replaces the kinds of entity it contains.

Delays computed from a timestamp use the agency time zone stored in the file, so they stay exact on the days the clocks change.

What is not: shapes and stops sent in a feed, trip modifications, and replaced or duplicated trips. They are counted as `unsupported` in the report. See [TODO.md](TODO.md).

## What is in the file

- Stops, including stations and entrances
- Routes, trips and stop times
- Shapes, cut at the stops so that trips over the same road share the pieces
- Services, from calendar and calendar dates
- Transfers, including trip-specific ones
- Trip patterns, the grouped stop sequences
- Pickup and drop-off rules, trip direction and block
- The agency time zone and the feed's validity dates
- What the file says about itself: where it came from, when it was compiled and the days any service runs, so `Consumer::is_stale` checks a file without reading a table, and room for your own JSON
- A grid index for stops near a position, and name search that ignores case and accents
- Coordinates, scaled to 7 decimal places for about 1.1 cm of precision

## Where it is going

flatnet is moving from a compiled GTFS to a format of its own, with a reader for each source. The first two changes below exist. The plan and the reasons are in [TODO.md](TODO.md) and [docs/design.md](docs/design.md).

- **A schema we own.** A field is in the file because something reads it, not because a source has it. The first change is done: times are stored once for each trip pattern, which took SL's file from 132 MB to 54 MB.
- **One feature for each source.** The compiler already takes a source-neutral `Network`, and GTFS is a reader behind the `gtfs` feature. A NeTEx reader would be a `netex` feature beside it, and the realtime side gets a `siri` feature next to `gtfs-rt`. The compiler builds everything derived from a `Network` (patterns, grids, walking paths, search) and writes the file.
- **A book on the schema,** with a chapter for each table and the reasons behind its layout.

## Development

You need [rustup](https://rustup.rs) and [just](https://github.com/casey/just). The coding rules are in the `code-standards/` submodule, so clone with `--recurse-submodules`. The `gtfs-rt` feature also needs `protoc`, because `gtfs-rt` compiles its protobuf files when it builds (`brew install protobuf` or `apt install protobuf-compiler`).

```sh
just toolchain   # the pinned Rust, nightly rustfmt, the cargo tools and the git hooks
just check       # what CI runs: fmt, clippy, docs, tests, features, dependencies
just check-full  # the same, with the property tests run for more cases
just bench       # the compiler and the consumer, timed
```

Run `just` on its own to list every recipe, including `compile`, `inspect`, `apply` and `real-data`. [CONTRIBUTING.md](CONTRIBUTING.md) covers setup and commits, and [STANDARDS.local.md](STANDARDS.local.md) holds what is specific to this project: the hot paths and the measured performance problems.

## Roadmap

- [x] Flat, relational array data structures
- [x] Compiler API for building GTFS feeds
- [x] Consumer API for memory-mapped reading
- [x] GTFS Realtime data ingestion and mapping
- [x] Own GTFS parser, with no GTFS library dependency
- [x] Validation of every table when a file is opened, with property tests
- [x] Metadata in the file, shapes cut at stops, and a viewer
- [x] Times stored once for each pattern
- [ ] The rest of the schema pass
- [ ] A reader crate for each source, over a source-neutral `Network`
- [ ] A book on the schema
- [ ] Expose prefetching (`madvise`) helper functions for pathfinding loops
- [ ] Multi-threaded graph compilation API

## References

- [GTFS Specification](https://gtfs.org/documentation/schedule/reference/)
- [GTFS Realtime Specification](https://gtfs.org/documentation/realtime/reference/)
- [bytemuck](https://docs.rs/bytemuck/latest/bytemuck/)
- [memmap2](https://docs.rs/memmap2/latest/memmap2/)
- [mmap](https://en.wikipedia.org/wiki/Mmap)
- [blaise](https://github.com/vincbro/blaise), a transit router built on top of flatnet

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
