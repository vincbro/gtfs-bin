# Compiling

The `flatnet-compiler` crate turns a transit network into a `.flnt` file. This page covers the two ways to feed it: a GTFS feed, or a `Network` you build yourself.

```
cargo add flatnet-compiler
```

## From a GTFS feed

The shortest path reads a feed and compiles it in one call:

```rust
use std::{fs, path::Path};

use flatnet_compiler::Compiler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A zip, or a folder of .txt files.
    let bytes = Compiler::new().compile_gtfs(Path::new("path/to/gtfs.zip"))?;
    fs::write("network.flnt", &bytes)?;
    Ok(())
}
```

`compile_gtfs` is `compile(gtfs::Reader::new(path).read()?)`. Use the two steps when you want the reader's report or options.

### What the reader accepts

The reader is flatnet's own, with no GTFS library behind it. It handles:

- byte order marks, Windows line endings and quoted fields with commas and new lines,
- columns in any order, and extra columns it doesn't know,
- a `.zip` where the files sit in a folder inside it,
- `calendar.txt` alone, `calendar_dates.txt` alone, or both,
- times past midnight such as `25:10:00`,
- stop times without times: stops between two timed ones get times spread evenly, by distance along the shape when the feed has one.

A row that points at an unknown route, trip or stop is skipped, and the first of a repeated id wins.

A bad value is an error that names the file, line and column:

```
stop_times.txt line 3: invalid arrival_time "soon"
```

Coordinates have to be on the globe: a `stop_lat` outside ±90 or a `stop_lon` outside ±180 is a bad value, and so is a date after year 9999.

### Lenient mode

A feed with a few bad rows can still be useful. `lenient()` leaves out the rows with a bad value, reads the rest, and says what it skipped:

```rust
use std::path::Path;

use flatnet_compiler::{Compiler, gtfs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (network, report) = gtfs::Reader::new(Path::new("messy.zip"))
        .lenient()
        .read_with_report()?;

    println!("{} rows were left out", report.skipped_total);
    for skipped in &report.skipped_rows {
        // The first hundred are kept as text; the rest are only counted.
        println!("{}: {}", skipped.file, skipped.message);
    }

    let bytes = Compiler::new().compile(network)?;
    std::fs::write("network.flnt", bytes)?;
    Ok(())
}
```

A missing required file or column is an error in either mode, because there is nothing to read.

## Options

`Compiler` is a builder. Each method returns the compiler, so you chain them:

```rust
use std::path::Path;

use flatnet_compiler::Compiler;
use flatnet_core::models::{Parts, Settings};

fn compile(path: &Path) -> Result<Vec<u8>, flatnet_compiler::Error> {
    Compiler::new()
        .parts(Parts::ALL.without(Parts::SHAPES))
        .settings(Settings {
            footpath_max_meters: 600,
            footpath_max_per_place: 16,
        })
        .compiled_at(1_700_000_000)
        .source("sl.zip".to_owned())
        .metadata_entry("region".to_owned(), "stockholm".to_owned())
        .compile_gtfs(path)
}
```

| Method | What it sets |
|---|---|
| `parts(Parts)` | Which optional parts to write. Default: all. |
| `settings(Settings)` | The choices that shape the tables, such as footpath reach. Default: `Settings::default()`. |
| `compiled_at(u64)` | When the file was compiled, in seconds since the unix epoch. |
| `source(String)` | The name of the feed, such as `sl.zip`. |
| `metadata_json(String)` | A JSON object to put in the file. |
| `metadata_entry(key, value)` | One more text entry for that JSON object. |

### Parts

`Parts` is a set of bits. `Parts::ALL` has every part, `Parts::NONE` has none, and `with`, `without` and `contains` add, remove and test:

```rust
use flatnet_core::models::Parts;

fn parts() {
    let small = Parts::ALL.without(Parts::SHAPES).without(Parts::FOOTPATHS);
    assert!(small.contains(Parts::TEXT));
    assert!(!small.contains(Parts::SHAPES));
}
```

Search needs text. Asking for `SEARCH` without `TEXT` is an error (`SearchWithoutText`). What each part holds: [Concepts](concepts.md#parts).

### Settings

`Settings` has two fields and both are written into the file:

| Field | Default | Meaning |
|---|---|---|
| `footpath_max_meters` | `400` | The longest straight line between two places a footpath may hold. At most `Settings::MAX_FOOTPATH_METERS`, 50,000. |
| `footpath_max_per_place` | `32` | The most footpaths one place keeps, nearest first. `u32::MAX` keeps every one in reach. |

A reach above the maximum is an error (`InvalidSettings`). There is no walking speed and no detour factor: a footpath is a straight line in meters, and turning it into a time is your router's job. See [Finding things](finding-things.md#places-and-footpaths).

### Metadata

Besides the fields the compiler fills itself (version, validity range, agency), the file can carry your own JSON object:

```rust
use flatnet_compiler::Compiler;

fn with_metadata() -> Compiler {
    Compiler::new()
        // Must be a JSON object. It is stored compact, with its keys sorted.
        .metadata_json(r#"{"owner":"transit-team","tier":"gold"}"#.to_owned())
        // Added to the object, replacing a key with the same name.
        .metadata_entry("tier".to_owned(), "silver".to_owned())
}
```

The compiler never reads the clock or the path. That is what keeps one feed giving one file, so `compiled_at` and `source` are only recorded when you pass them (the `flatnet` command does). Reading it back: [Reading a file](reading.md#metadata).

## From your own source

`Compiler::compile` takes a `Network`: the same thing the GTFS reader produces. If you have data that isn't GTFS, such as a database, an API or another format, build a `Network` and compile it. There is no CSV or zip code in the way: turn off the default `gtfs` feature and those dependencies go away.

```toml
[dependencies]
flatnet-compiler = { version = "0.2", default-features = false }
```

A `Network` is plain vectors. Everything refers to something else by its **position in the vector that holds it**, not by an id. You resolve ids once, and leave out what points at nothing.

```rust
use flatnet_compiler::{
    Call, Compiler, Network, RouteRecord, ServiceRecord, StopRecord, TripRecord, Weekly,
};
use flatnet_core::models::{
    BoardingRule, Date, Direction, LocationType, Optional, RouteType, Sentinel, Time,
};

fn stop(id: &str, latitude: f64, longitude: f64) -> StopRecord {
    StopRecord {
        id: id.to_owned(),
        code: None,
        name: Some(format!("Stop {id}")),
        description: None,
        latitude: Some(latitude),
        longitude: Some(longitude),
        location_type: LocationType::STOP_POINT,
        parent: None,
    }
}

fn call(trip: usize, stop: usize, sequence: u32, seconds: u32) -> Call {
    Call {
        trip,
        stop,
        sequence,
        arrival: Optional::new(Time(seconds)),
        departure: Optional::new(Time(seconds)),
        headsign: None,
        distance: Optional::default(),
        pickup: BoardingRule::REGULAR,
        drop_off: BoardingRule::REGULAR,
    }
}

fn build() -> Result<Vec<u8>, flatnet_compiler::Error> {
    let network = Network {
        stops: vec![stop("A", 59.00, 18.00), stop("B", 59.01, 18.01)],
        routes: vec![RouteRecord {
            id: "R".to_owned(),
            short_name: Some("1".to_owned()),
            long_name: None,
            description: None,
            route_type: RouteType::BUS,
        }],
        services: vec![ServiceRecord {
            id: "daily".to_owned(),
            // Monday first. A week that repeats between two dates, both included.
            weekly: Some(Weekly {
                days: [true; 7],
                start: Date::parse_gtfs("20260101").expect("a date"),
                end: Date::parse_gtfs("20261231").expect("a date"),
            }),
            // Days added (true) or removed (false), in the order they apply.
            exceptions: vec![(Date::parse_gtfs("20261225").expect("a date"), false)],
        }],
        trips: vec![TripRecord {
            id: "T1".to_owned(),
            route: 0,           // position in `routes`
            service: Some(0),   // position in `services`
            headsign: None,
            short_name: None,
            line: None,
            block: None,
            direction: Direction::NONE,
        }],
        // In any order: the compiler orders them by trip and sequence.
        calls: vec![call(0, 1, 2, 8 * 3600 + 600), call(0, 0, 1, 8 * 3600)],
        ..Network::default()
    };
    Compiler::new().compile(network)
}
```

### What a `Network` holds

| Field | Item | Notes |
|---|---|---|
| `info` | `NetworkInfo` | Time zone, agency, validity dates, publisher, language, version, contact. All optional. |
| `stops` | `StopRecord` | `parent` is the position of the station it belongs to. A stop with no latitude and longitude has no position. |
| `routes` | `RouteRecord` | Names are optional. |
| `services` | `ServiceRecord` | A weekly pattern and/or exceptions. A service with neither is an error. |
| `trips` | `TripRecord` | `route` is required. `service` and `line` are optional. |
| `lines` | `Vec<Vec<Coordinate>>` | Shape paths. A trip names one by position in `line`. |
| `headsigns` | `Vec<String>` | Headsigns that calls use, each once. A call names one by position. |
| `calls` | `Call` | Every stop of every trip. Times are `Optional<Time>`, so a stop can have none. |
| `transfers` | `TransferRecord` | Between stops, optionally limited to a pair of trips. |

### What the compiler checks

`Compiler::compile` refuses a `Network` that would not make a valid file, and returns an error instead of writing one:

- two things of one kind with the same id,
- a reference that points at nothing, such as a trip whose `route` is out of range,
- a service with no days, or one that runs for more than 36,600 days (about 100 years),
- a date after year 9999, or a stop or shape point off the globe,
- JSON metadata that isn't an object,
- search without text, or settings out of range.

A file the compiler writes is always one `Consumer::new` accepts. That is checked by a property test over arbitrary networks (`crates/flatnet-core/tests/fuzz_network.rs`).

## Errors

`flatnet_compiler::Error` and its `flatnet_compiler::Result` alias are the only types you handle. The variants are listed with their causes in [Troubleshooting](troubleshooting.md#compile-errors).

## Speed and memory

On SL's regional feed (21,872 stops, 123,866 trips, 2.5 million stop times) a compile takes about 4 s and peaks around 340 MB. The footpath step uses all cores. Figures change as the compiler does; `just bench` measures the compiler and consumer on a generated network, and [TODO.md](../TODO.md) has the latest numbers from SL.

Next: [Reading a file](reading.md).
