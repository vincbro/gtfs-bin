# gtfs-bin

*gtfs-bin* is a library for building standard GTFS data into highly optimized, memory-mappable binary graphs (flat, relational arrays).

It provides the data structures, serialization tools, and zero-copy reading interfaces needed to build distributed transit applications, whether for high-throughput routing engines, research simulations, or spatial analytics.

## The Architecture

In many transit applications, servers parse raw GTFS text files directly into memory at startup. For large networks, this creates massive CPU overhead, slow initialization times, and cost-prohibitive memory requirements when scaling horizontally.

*gtfs-bin* facilitates a different architecture: **Compile once, distribute everywhere.**

1. **The Master Node (Compiler)**: A single server or build pipeline uses the `gtfs-bin` builder API to ingest raw GTFS data. It computes spatial hashes, groups routes, generates walkable transfers, and packs the results into a flat, contiguous `.gtfs` binary file.
2. **The Fleet (Consumers)**: The compiled artifact is distributed to any number of worker nodes.
3. **Instant Memory Mapping**: The consumer nodes use `mmap` to map the file directly into virtual memory. Because the structures are `#[repr(C)]` and rely on zero-copy deserialization, startup is completely decoupled from dataset size.

By delegating the parsing and graph generation to a single master process, consumer nodes only consume physical RAM for the specific disk pages they actively query.

## Core Features

- **Library**: Exposes both the builder API for generating binary structures and the zero-copy reader API for consuming them. What you build on top of the data (routing, analytics, rendering) is up to you.
- **Zero-Copy Deserialization**: Uses `bytemuck` to cast raw bytes directly to typed Rust arrays in memory. No allocations or parsing occur on the consumer nodes.
- **Memory-Mapped**: Leverages the operating system's virtual memory management (`mmap`). Consumer applications load instantly, bypassing standard I/O bottlenecks.
- **Cache-Friendly Layout**: Data entities are strictly flat and pointer-free, using integer slices to define relationships, making traversal highly predictable for CPU caches.

## Current Status

The project is in early development. The following components are currently implemented:

- **Data Models**: Core GTFS entities are defined with `#[repr(C)]`, `bytemuck::Pod`, and `bytemuck::Zeroable` for zero-copy serialization. Supported entities include:
  - Stops (including stations and entrances)
  - Routes
  - Trips
  - StopTimes
  - Shapes
  - Coordinates (Scaled to 7 decimal places, packing `f64` into `i32` for ~1.1cm precision), distances, and time types
- **Compiler API**: For compiling GTFS feeds into the optimized binary format.
- **Consumer API**: For zero-copy memory-mapped reading.

## Usage

`gtfs-bin` can be used both as a standalone CLI tool and as a Rust library.

### As a CLI Tool

You can compile a standard GTFS zip file or directory into a `.gtfs` binary using the included executable:

```bash
cargo run --release -- -i path/to/gtfs.zip -o output.gtfs
````

### As a Library

Add `gtfs-bin` to your `Cargo.toml`:

```toml
[dependencies]
gtfs-bin = "0.1.2"
memmap2 = "0.9"
```

or run

```bash
cargo add gtfs-bin
cargo add memmap2
```

#### 1\. Compiling GTFS Data (The Master Node)

Use the `Compiler` to parse raw GTFS data and generate the optimized binary format.

```rust
use gtfs_bin::compiler::Compiler;
use std::fs::File;
use std::io::Write;

fn main() {
    // Read from a zip file or a directory
    let compiler = Compiler::new("path/to/gtfs.zip");
    let bytes = compiler.compile().expect("Failed to compile GTFS");

    let mut file = File::create("network.gtfs").unwrap();
    file.write_all(&bytes).unwrap();
}
```

#### 2\. Reading GTFS Data (The Consumer Node)

Use `memmap2` and the `Consumer` to instantly map the compiled file into memory with zero parsing overhead.

```rust
use gtfs_bin::consumer::Consumer;
use memmap2::MmapOptions;
use std::fs::File;

fn main() {
    let file = File::open("network.gtfs").expect("Failed to open binary");
    let mmap = unsafe { MmapOptions::new().map(&file).expect("Failed to map memory") };
    
    // Instantly load the entire transit network
    let consumer = Consumer::new(&mmap).expect("Failed to parse binary header");

    // Look up a route by its original GTFS ID
    if let Some(route) = consumer.route_by_id("route_1") {
        println!("Found route: {}", consumer.string(route.short_name.get().unwrap()));
        
        // Iterate over all trips on this route
        for trip in consumer.iter_route_trips(route.idx) {
            println!("Trip ID: {}", consumer.trip_id(trip.id));
        }
    }
}
```

## Roadmap

  - [x] Zero-copy deserialization using `bytemuck`
  - [x] Flat, relational array data structures
  - [x] Compiler API for compiling GTFS feeds
  - [x] Consumer API for memory-mapped reading
  - [ ] Expose prefetching (`madvise`) helper functions for pathfinding loops
  - [ ] Multi-threaded graph compilation API

## References

  - [GTFS Specification](https://gtfs.org/documentation/schedule/reference/)
  - [bytemuck](https://docs.rs/bytemuck/latest/bytemuck/)
  - [memmap2](https://docs.rs/memmap2/latest/memmap2/)
  - [mmap](https://en.wikipedia.org/wiki/Mmap)
  - [madvise](https://man7.org/linux/man-pages/man2/madvise.2.html)
  - [blaise](https://github.com/vincbro/blaise) - A high-performance transit routing engine (soon to be) built on top of `gtfs-bin`.

## License

Licensed under either of

  - Apache License, Version 2.0 ([LICENSE-APACHE](https://www.google.com/search?q=LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
  - MIT license ([LICENSE-MIT](https://www.google.com/search?q=LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
