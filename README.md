# gtfs-bin

*gtfs-bin* is a library for compiling standard GTFS data into highly optimized, memory-mappable binary graphs.

It provides the data structures, serialization tools, and zero-copy reading interfaces needed to build distributed transit applications,whether for high-throughput routing engines, research simulations, or spatial analytics.

## The Architecture

In standard transit applications, every server in a fleet parses raw GTFS text files directly into physical RAM at startup. For large networks, this creates massive CPU overhead, slow initialization times, and cost-prohibitive memory requirements when scaling horizontally.

*gtfs-bin* facilitates a different architecture: **Compile once, distribute everywhere.**

1. **The Master Node (Compiler)**: A single server or build pipeline uses the `gtfs-bin` builder API to ingest raw GTFS data. It computes spatial hashes, groups routes, generates walkable transfers, and packs the results into a flat, contiguous `.blaise` binary file.
2. **The Fleet (Consumers)**: The compiled artifact is distributed to any number of worker nodes.
3. **Instant Memory Mapping**: The consumer nodes use `mmap` to map the file directly into virtual memory. Because the structures are `#[repr(C)]` and rely on zero-copy deserialization, startup is completely decoupled from dataset size.

By delegating the parsing and graph generation to a single master process, consumer nodes only consume physical RAM for the specific disk pages they actively query.

## Core Features

- **Library**: Exposes both the builder API for generating binary graphs and the zero-copy reader API for consuming them. What you build on top of the graph (routing, analytics, rendering) is up to you.
- **Zero-Copy Deserialization**: Uses `bytemuck` to cast raw bytes directly to typed Rust arrays in memory. No allocations or parsing occur on the consumer nodes.
- **Memory-Mapped**: Leverages the operating system's virtual memory management (`mmap`). Consumer applications load instantly, bypassing standard I/O bottlenecks.
- **Cache-Friendly Layout**: Data entities are strictly flat and pointer-free, using integer slices to define relationships, making graph traversal highly predictable for CPU caches.

## Quick Start

### Compiling the Graph (Master Node)

```rust
use gtfsbin::Compiler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse the raw GTFS zip, run graph generation, and write to a binary file
    Compiler::new()
        .from_zip("sweden.zip")?
        .compile_to_file("sweden.blaise")?;
        
    println!("Graph compiled successfully.");
    Ok(())
}

```

### Consuming the Graph (Worker Node)

```rust
use std::fs::File;
use memmap2::MmapOptions;
use gtfsbin::Repository;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Open the pre-compiled binary file
    let file = File::open("sweden.blaise")?;
    
    // 2. Map the file into virtual memory
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    
    // 3. Cast the bytes into strongly-typed Rust slices (O(1) time)
    let repo = Repository::load_from_mmap(&mmap);
    
    println!("Loaded {} stops instantly. Ready for queries.", repo.stops.len());
    
    // Pass `repo` to your custom routing engine, analyzer, etc.
    Ok(())
}

```

## CLI Tool

While *gtfs-bin* is designed to be integrated directly into your Rust applications, it includes a convenience CLI tool for compiling feeds from the command line without writing a custom builder.

```bash
cargo install gtfs-bin
gtfs-bin compile ./sweden.zip -o sweden.blaise

```

## Roadmap

- [ ] Zero-copy deserialization using `bytemuck`
- [ ] Flat, relational array data structures
- [ ] Expose prefetching (`madvise`) helper functions for pathfinding loops
- [ ] Multi-threaded graph compilation API

## References

- [GTFS Specification](https://gtfs.org/documentation/schedule/reference/)
- [bytemuck](https://docs.rs/bytemuck/latest/bytemuck/)
- [memmap2](https://docs.rs/memmap2/latest/memmap2/)
- [mmap](https://en.wikipedia.org/wiki/Mmap)
- [blaise](https://github.com/vincbro/blaise) - A high-performance transit routing engine (soon to be) built on top of `gtfs-bin`.

## License

Licensed under either of

 - Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 - MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
