# TODO

What is done, what is open, and what is deliberately left out. Update it when the answer changes.

Status: `[x]` done and tested, `[ ]` open, `[-]` decided against, with the reason.

Measured on SL's regional feed (Trafiklab, 21,872 stops, 123,866 trips, 2.5M stop times): compile 3.8 s and 364 MB peak, output 54 MB, open and validate 10 to 16 ms, checksum 12 to 17 ms, 456 of 457 realtime trip updates matched. Benchmarks on a generated network of 1M stop times are in `crates/flatnet-core/benches/transit.rs`.

## Own schema and sources

The direction, decided 2026-11-20: flatnet becomes its own transit network format, built to make a network easy and fast to work with at any size. It does not aim to cover GTFS, GTFS-RT, NeTEx or SIRI. Each source is a reader behind a cargo feature, and a field is in the format because a consumer needs it, not because a source has it. Reasoning and the comparison of the four formats: `docs/OWN-SCHEMA.md`.

- [ ] Go through every field of every table and write down what reads it and what it costs. Drop what nothing reads, add what a consumer needs (`blaise` first). Measure size and speed on SL and the generated feed
- [ ] Crates instead of features (2026-11-20): one crate per source, each depending only on `flatnet-model` (static readers) or `flatnet-core` (realtime decoders), so a source's dependencies, tests and releases stay its own:
  - `flatnet-model`: `Network`, the in-memory, source-neutral network a reader produces, and what it is checked against. Not `Pod`, not the file layout. A reader depends on this and nothing else
  - `flatnet-gtfs`: today's parser (`source`, `table`, `records`). Owns what is GTFS: interpolating untimed stops, route and location type codes, `calendar` and `calendar_dates` into service days, repeated ids, lenient mode
  - `flatnet-netex`, `flatnet-siri`: later. `flatnet-gtfs-rt`: the GTFS-RT decoder, which today lives in core
  - `flatnet-compiler`: takes a `Network` and builds everything derived, then writes the file: patterns and their time tables, shapes cut at stops, the grid, walking paths, search, the id lookups, the metadata. It reads no source format
  - `flatnet-core`: the file, the `Consumer` and `Realtime`, plus the source-neutral `Update`
  - `flatnet-cli`: picks readers with cargo features (`gtfs` by default), since the binary is where a user chooses what to carry
- [ ] Port the GTFS parser out of `flatnet-compiler` into the reader crate, producing a `Network`, and the compiler to consume one. A pure refactor, so the SL output is byte for byte the same. The check ends at the first deliberate format change
- [ ] Realtime split: a source-neutral `Update` (aimed, expected and actual time and a status per call), a decoder per source, and `Realtime` applies updates without knowing the wire format
- [ ] Schema pass, agreed 2026-11-20. On SL, `stop_times` is 100 MB of the 132 MB file (2.5M items × 40 B), `stop_to_trips` 10 MB, `trips` 8 MB. 124k trips share 3,710 patterns. In order, each measured on SL (size, open, full scan, a blaise query) before the next:
  - [x] Prototype measured on SL (2026-11-20, a throwaway that read the compiled file): grouping trips by stops, `stop_sequence`, boarding rules, headsign and distance gives 3,897 groups (3,710 patterns today), and splitting the 995 groups whose trips overtake gives 5,161 patterns (at most 7 chains in one group). The times are 20.0 MB at 8 B a cell and 15.0 MB at 6 B, plus 2.8 MB for 115k pattern positions at 24 B, against 100.2 MB of `StopTime`s: 22.8 MB or 17.8 MB. The file goes from 132 MB to about 55 MB, and about 45 MB once `stop_to_trips` is gone. Scanning every departure took 1,674 µs in `StopTime`s and 209 µs in a flat `u32` array
  - [x] Times per pattern (format 9): a pattern holds its stops once, with `sequence`, boarding rules, headsign and distance per position, and a dense table of times (`trips × positions`, trips sorted by departure) in 8 bytes a cell, or 6 with a `u16` dwell. A pattern splits when those fields differ between trips, and when trips overtake. Estimated 132 MB → about 55 MB. Realtime keys become trip plus position. Done 2026-11-20: `StopTime` is gone. A trip's `Timing`s (8 B) lie pattern by pattern in `timings`, and `pattern_stops` holds what the pattern says at each stop. SL is 5,184 patterns (the prototype said 5,161: overtaking is checked on arrivals and departures both), the file 132 MB → 54.5 MB, open and validate 44 ms → 10–16 ms, checksum 34 ms → 12–17 ms, compile still 3.9 s and the peak memory 364 MB (it was 480 MB). `timings_by_trip` and `pattern_stops_by_trip_pattern` replace `stop_times_by_trip`, and `Realtime::arrival_time` and `departure_time` take the trip, the slot and the `Timing`. blaise needs the same change. A hostile feed with thousands of trips that all overtake one another makes the chain search quadratic (compile time only)
  - [ ] Split hot from cold: `Stop` into routing (`index`, `coordinate`, `parent`, `location_type`) and info (`code`, `name`, `description`); `Trip` into routing (`pattern`, `service`, position) and info (`headsign`, `short_name`, `block`), with `route_index` derived from the pattern
  - [ ] A header flag per optional section, so a compile can leave out shapes, search, names or ids. A routing-only file is about 40 MB or less on SL
  - [ ] Drop `stop_to_trips` (10 MB): stop to patterns with the position in each, then the trips of a pattern. blaise calls `trips_by_stop` twice
  - [ ] `Service`: a base date and the mask, without `weekday`, `start_date` and `end_date` if nothing reads them
  - [ ] Footpaths: stop to stop walking distances in meters from the compiler, so blaise doesn't query the grid per query. Meters, so walking speed stays the caller's
  - [ ] Trip start time, derived, and an index sorted by route, direction and start, so realtime matching has no ambiguous case. A run key is trip plus service day
  - [ ] Ids with a namespace, and several sources in one file, for a national network
  - [-] Until a consumer asks: a line above the route, operator apart from authority, a stop place apart from the stop point, validity per object, accessibility values
- [ ] `frequencies.txt` and `timepoint` are decided in the schema, once, for every source (see Parser)
- [ ] A `netex` reader for the Nordic profile only
- [ ] A `siri` decoder, estimated timetable first
- [x] Renamed to `flatnet` (2026-11-20: a flat network, which is the layout): crates, binary, magic `FLNT`, extension `.flnt`, format version 8. Still open: the GitHub repo, blaise, and a crates.io check. Check crates.io first. Crates `flatnet-model`, `flatnet-core`, `flatnet-compiler`, `flatnet-cli` and one per source (`flatnet-gtfs`, `flatnet-gtfs-rt`, `flatnet-netex`, `flatnet-siri`). A rename also means the header magic, the file extension and blaise's imports and README
- [-] Full NeTEx or SIRI coverage. Fares, vehicle scheduling, duties, facility monitoring and station navigation stay out until a consumer asks

## Parser (own GTFS reader)

- [x] Directory, `.zip`, and a `.zip` that holds a folder
- [x] Byte order mark, Windows line endings, quoted fields, columns in any order, extra columns
- [x] Errors that name the file, line, column and value
- [x] Required files and columns, `calendar.txt` or `calendar_dates.txt` required
- [x] Own date parsing, route type and location type codes, extended route types
- [x] Repeated ids: first wins. Rows pointing at unknown things are skipped
- [x] Service range limits (a bad end date can't cost hundreds of megabytes)
- [x] Streaming of `stop_times.txt` and `shapes.txt`
- [x] Deterministic output
- [x] Times between timed stops are filled in, by distance when the feed has it
- [x] `pickup_type` and `drop_off_type` on every stop time
- [x] `direction_id` and `block_id` on trips
- [x] `from_trip_id` and `to_trip_id` on transfers (in-seat transfers, types 4 and 5)
- [x] `agency.txt` time zone and `feed_info.txt` validity stored in the file
- [x] Lenient mode: bad rows are skipped and reported (`Compiler::lenient`, `compile_with_report`)
- [x] Tested on a real feed (SL)
- [x] Property tests that damage feeds, including `feed_info.txt` and `agency.txt`
- [ ] `frequencies.txt`. Needs a decision on whether to expand a headway into trips (new ids, bigger file) or store the headway. SL has no `frequencies.txt`, so nothing needs it yet
- [ ] `timepoint` (exact or approximate times). SL sets it on every stop time. Interpolation currently treats any time as exact
- [ ] Booking rules (`booking_rules.txt`, demand-responsive transport). SL has the file
- [ ] Pathways and levels, for routing inside stations
- [ ] Fares
- [ ] Pre-1970 dates are rejected, which suits feeds. Documented, not changed
- [-] Hand-written CSV parsing: the `csv` crate handles the edge cases

## Consumer and the binary format

- [x] Private fields, getters, invariants documented
- [x] Every section, index and slice validated, and every item's own index
- [x] Id lookups checked strictly sorted. Each trip's stop times checked to be its own and in order
- [x] Header holds the file length (always checked) and a checksum (`verify_checksum`)
- [x] `Consumer::new_unvalidated` for a file the program wrote itself
- [x] Format version 8 (the rename: magic `FLNT`), documented in `docs/FORMAT.md`
- [x] Metadata in the file: a fixed `Metadata` (compile time, active day range, compiler version, source, feed_info and agency fields) and a JSON section from `--meta` and `--meta-json`. `Consumer::is_stale(today)` is two comparisons
- [ ] Metadata JSON is text for the caller to parse. A typed reader for a few well known keys would need a JSON dependency in core
- [x] Shapes cut at the stops: a `Shape` is the path between two stops, stored once however many trips use it, and a trip holds one per hop. On SL 2.79M shape points became 610k, 28 MB off the file. Hops are snapped to the nearest shape vertex going forward, so a loop that passes a stop twice can snap to the wrong pass
- [ ] Cut shapes with `shape_dist_traveled` when the feed has it, and report stops that are far from their shape (on SL 750 of 81,486 are more than 50 m away)
- [ ] A shape and its reverse hold the same points backwards: store them once
- [x] `advise` to tell the operating system how the file is read
- [x] Grid index: `stops_near` fills a buffer, nearest first, and agrees with a scan of every stop (property test)
- [x] `search_by_name` ignoring case and accents, ranked exact, prefix, word, contains
- [x] Stops with no parent station are searchable (they weren't)
- [x] Property tests: arbitrary bytes, truncation, damage aimed at tables, swapped words. Each validator shown to matter
- [x] Time arithmetic never panics
- [ ] A spatial query for stops near a line or inside a box
- [ ] Search by stop code and by route name, and typo-tolerant search
- [ ] `search_by_name` allocates per name. Index folded names at compile time if it is ever called in a loop
- [ ] Decide the future of `Slice` and `Index` as 23 macro-made types. Kept for type safety. Revisit when the API next breaks
- [-] Replace the flat `Pod` layout or `Optional<T>`: kept on purpose, and SL's numbers (39 ms open, 2 ms scan of 2.5M stop times) don't call for it

## Realtime

- [x] Trip updates by `trip_id`, or by route, start time, date and direction
- [x] Ambiguous matches are reported and applied to neither trip
- [x] Stop time updates by `stop_sequence` or `stop_id`
- [x] Delays from `delay` or from `time`, exact with the service date
- [x] Agency time zone from the file (follows daylight saving), or a named zone, or a fixed offset
- [x] Cascading delays, with `Skipped` and `NoData` following the spec
- [x] Each trip update replaces what the trip had. Stale updates are ignored
- [x] One `Realtime` per service day. Updates for another day are ignored and counted
- [x] Full dataset feeds replace only the kinds they contain. `is_deleted` is honored
- [x] Vehicle positions with stop, bearing, speed, occupancy, timestamp
- [x] Added trips kept beside the static ones
- [x] Alerts resolved to routes, stops, trips. `alerts_for_stop`, `alerts_for_route`, `alerts_for_trip`
- [x] `ApplyReport`, query helpers, `decode_feed`, debug-only `check_invariants`
- [x] Tested on real SL feeds: 456 of 457 trip updates, 546 of 558 vehicles, 184 of 187 alerts matched, each in about a millisecond
- [x] Property tests with arbitrary feeds and extreme timestamps
- [x] `Realtime` and `Consumer` are `Send + Sync` (tested)
- [ ] Added trips are stored but a router can't use them. They have no index in the static tables
- [ ] Trip modifications, shapes and stops sent in a feed. The spec marks them experimental, and SL sends none (`unsupported` in the report)
- [ ] Replaced and duplicated trips (`unsupported`)
- [ ] Frequency based trips (`unscheduled`), which need `frequencies.txt` first
- [ ] Vehicle descriptor (id, label, license plate) from trip updates and vehicle positions. Needs strings, so not `Copy` like `Vehicle`
- [ ] Occupancy per stop time and carriage details
- [ ] A helper that swaps a `Realtime` behind an `Arc` for a router that reads while a feed is applied (the pattern is documented)
- [ ] Fetching feeds. This crate decodes and applies, and leaves the HTTP and the API key to the caller

## Tests and tooling

- [x] 177 tests: unit, integration, parser, features, realtime, scale, and six property test files, two of which (`fuzz_shapes`, `fuzz_metadata`) compile generated feeds and check what the compiler promises, not only that nothing panics
- [ ] The generated feed (`tests/common/synthetic.rs`) has no shapes, so the scale tests and benchmarks don't cover them
- [x] Larger generated feed (`crates/flatnet-core/tests/common/synthetic.rs`, up to 1M stop times)
- [x] Real data test, run with `FLATNET_REAL_FEED`
- [x] Benchmarks (`just bench`)
- [x] `crates/flatnet-cli/examples/inspect.rs` and `apply_feed.rs`
- [x] `crates/flatnet-viewer`: an egui app to walk a compiled file and see its stops and shapes on a map (`just view`). Left out of `cargo deny`, see `deny.toml`
- [x] CI on the `just` recipes: format, clippy, docs, tests, features, dependencies, and a long property run on `main` (`.github/workflows/ci.yml`, not run yet: it needs pushing)
- [x] The same lint policy as `wend` (clippy `all`, `pedantic`, `cargo` and chosen `restriction` lints, rustc and rustdoc lints), `clippy.toml`, `rust-version`, a `dist` profile, `CONTRIBUTING.md`
- [x] CI also checks the commit messages of a PR and builds on `rust-version`
- [x] Saved failing cases are committed (`crates/flatnet-core/tests/*.proptest-regressions`)
- [x] `.env` is ignored (it held API keys and wasn't)
- [-] `cargo fuzz`: the proptest suites cover the parser, the consumer and the realtime builder, and found every bug so far. A coverage-guided fuzzer needs nightly and libFuzzer for little more. Revisit if a bug turns up that proptest missed
- [x] `justfile`, `rustfmt.toml` (import grouping on a pinned nightly), `taplo.toml`, `deny.toml`, `rust-toolchain.toml` and a `commit-msg` hook, as `wend` has them
- [x] `cargo deny` found seven advisories in `zip` 7, and ones in `memmap2` and `crossbeam`. All fixed by updating (`zip` is now 9)
- [ ] Run the benchmarks in CI and keep the numbers, so a slowdown shows
- [ ] Run the `review` skill again on the whole tree: this change is large
- [ ] Publish: bump the crate version for the format and API breaks, and write the changelog
