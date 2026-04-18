# gtfs-bin

*gtfs-bin* is a tool and library for compiling standard GTFS data into highly optimized, memory-mappable binary files. 

It provides the data structures and interfaces needed to build fast transit applications, such as routing engines or spatial analytics tools, with virtually instant startup times and low memory overhead.

## The Architecture

Working with `gtfs-bin` is split into two distinct phases: **Compilation** and **Consumption**. This separation of concerns allows you to do the heavy lifting of parsing data once, and distribute a highly optimized artifact to your actual applications.

### 1. The Builder API (Compilation)
During the build phase, the `Compiler` ingests raw GTFS text files (from a directory or `.zip`). It performs several optimization passes:
- Parses and validates the raw CSV data.
- Resolves all string-based IDs into integer indices to establish direct, pointer-free relationships (e.g., linking a `Trip` to its `Route`).
- Deduplicates and packs all textual data (like stop names and headsigns) into a single contiguous string block.
- Writes the finalized, flat relational arrays into a single `.gtfs` binary artifact.

### 2. The Consumer API (Consumption)
In your actual transit application, you use the `Consumer` to read the data. Instead of parsing files into memory, the application uses the operating system's `mmap` to map the compiled `.gtfs` file directly into virtual memory. 

**What you can expect:**
- **Instant Startup:** The network loads in the time it takes the OS to set up a memory map (microseconds), bypassing traditional I/O and deserialization bottlenecks.
- **Low Memory Footprint:** Data is lazily paged into physical RAM by the OS only when you actively query it.
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
- Coordinates (Scaled to 7 decimal places for ~1.1cm precision)

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
gtfs-bin = "0.1.3"
memmap2 = "0.9"
```

#### Phase 1: Compiling Data

If you are building a data pipeline, use the `Compiler` to generate the `.gtfs` binary format.

```rust
use gtfs_bin::compiler::Compiler;
use std::fs::File;
use std::io::Write;

fn main() {
    // Read from a zip file or an extracted directory
    let compiler = Compiler::new("path/to/gtfs.zip");
    
    // Parse the data and generate the binary byte array
    let bytes = compiler.compile().expect("Failed to compile GTFS");

    // Save the artifact to disk
    let mut file = File::create("network.gtfs").unwrap();
    file.write_all(&bytes).unwrap();
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
        let route_name = consumer.string(route.short_name.get().unwrap());
        println!("Found route: {}", route_name);
        
        // Iterate over all trips belonging to this route
        for trip in consumer.iter_route_trips(route.idx) {
            println!("Trip ID: {}", consumer.trip_id(trip.id));
        }
    }
}
```

#### Phase 3: GTFS Realtime (Optional)

If you enable the `rt` feature in `Cargo.toml`, you can apply GTFS-RT `FeedMessage` updates directly on top of your static consumer graph.

```rust
use gtfs_bin::rt::RealtimeBuilder;

// ... Assumes `consumer` has been loaded ...
// `feed_messages` is an Iterator of parsed gtfs_rt::FeedMessage

let realtime_state = RealtimeBuilder::new(&consumer)
    .with_cascading_delays(true)
    .build(feed_messages);
    
// You can now query realtime delays, cancellations, and vehicle positions!
```

## Roadmap

  - [x] Flat, relational array data structures
  - [x] Compiler API for building GTFS feeds
  - [x] Consumer API for memory-mapped reading
  - [x] GTFS Realtime data ingestion and mapping
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
