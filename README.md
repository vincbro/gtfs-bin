# gtfs-bin

*gtfs-bin* is a library for building standard GTFS data into highly optimized, memory-mappable binary graphs.

It provides the data structures, serialization tools, and zero-copy reading interfaces needed to build distributed transit applications, whether for high-throughput routing engines, research simulations, or spatial analytics.

## The Architecture

In many transit applications, servers parse raw GTFS text files directly into memory at startup. For large networks, this creates massive CPU overhead, slow initialization times, and cost-prohibitive memory requirements when scaling horizontally.

*gtfs-bin* facilitates a different architecture: **Builder once, distribute everywhere.**

1. **The Master Node (Builder)**: A single server or build pipeline uses the `gtfs-bin` builder API to ingest raw GTFS data. It computes spatial hashes, groups routes, generates walkable transfers, and packs the results into a flat, contiguous `.gtfs` binary file.
2. **The Fleet (Consumers)**: The compiled artifact is distributed to any number of worker nodes.
3. **Instant Memory Mapping**: The consumer nodes use `mmap` to map the file directly into virtual memory. Because the structures are `#[repr(C)]` and rely on zero-copy deserialization, startup is completely decoupled from dataset size.

By delegating the parsing and graph generation to a single master process, consumer nodes only consume physical RAM for the specific disk pages they actively query.

## Core Features

- **Library**: Exposes both the builder API for generating binary graphs and the zero-copy reader API for consuming them. What you build on top of the graph (routing, analytics, rendering) is up to you.
- **Zero-Copy Deserialization**: Uses `bytemuck` to cast raw bytes directly to typed Rust arrays in memory. No allocations or parsing occur on the consumer nodes.
- **Memory-Mapped**: Leverages the operating system's virtual memory management (`mmap`). Consumer applications load instantly, bypassing standard I/O bottlenecks.
- **Cache-Friendly Layout**: Data entities are strictly flat and pointer-free, using integer slices to define relationships, making graph traversal highly predictable for CPU caches.

## Current Status

The project is in early development. The following components are in progress:

- **Data Models**: Core GTFS entities are defined with `#[repr(C)]`, `bytemuck::Pod`, and `bytemuck::Zeroable` for zero-copy serialization. Supported entities include:
  - Stops (including stations and entrances)
  - Routes
  - Trips
  - StopTimes
  - Shapes
  - Coordinates, distances, and time types

- **Builder API**: Coming soon - for compiling GTFS feeds into binary format
- **Consumer API**: Coming soon - for zero-copy memory-mapped reading

## Roadmap

- [x] Zero-copy deserialization using `bytemuck`
- [x] Flat, relational array data structures
- [ ] Builder API for compiling GTFS feeds
- [ ] Consumer API for memory-mapped reading
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
