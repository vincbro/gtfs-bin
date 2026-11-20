# TODO

What is done, what is open, and what is deliberately left out. Update it when the answer changes.

Status: `[x]` done and tested, `[ ]` open, `[-]` decided against, with the reason.

Measured on SL's regional feed (Trafiklab, 21,872 stops, 123,866 trips, 2.5M stop times): compile 3.0 s and 480 MB peak, output 159 MB, open and validate 39 ms, checksum 34 ms, 456 of 457 realtime trip updates matched. Benchmarks on a generated network of 1M stop times are in `crates/gtfs-bin-core/benches/transit.rs`.

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
- [x] Format version 5, documented in `docs/FORMAT.md`
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

- [x] 147 tests: unit, integration, parser, features, realtime, scale, and four property test files
- [x] Larger generated feed (`crates/gtfs-bin-core/tests/common/synthetic.rs`, up to 1M stop times)
- [x] Real data test, run with `GTFS_BIN_REAL_FEED`
- [x] Benchmarks (`just bench`)
- [x] `crates/gtfs-bin/examples/inspect.rs` and `apply_feed.rs`
- [x] CI on the `just` recipes: format, clippy, docs, tests, features, dependencies, and a long property run on `main` (`.github/workflows/ci.yml`, not run yet: it needs pushing)
- [x] The same lint policy as `wend` (clippy `all`, `pedantic`, `cargo` and chosen `restriction` lints, rustc and rustdoc lints), `clippy.toml`, `rust-version`, a `dist` profile, `CONTRIBUTING.md`
- [x] CI also checks the commit messages of a PR and builds on `rust-version`
- [x] Saved failing cases are committed (`crates/gtfs-bin-core/tests/*.proptest-regressions`)
- [x] `.env` is ignored (it held API keys and wasn't)
- [-] `cargo fuzz`: the proptest suites cover the parser, the consumer and the realtime builder, and found every bug so far. A coverage-guided fuzzer needs nightly and libFuzzer for little more. Revisit if a bug turns up that proptest missed
- [x] `justfile`, `rustfmt.toml` (import grouping on a pinned nightly), `taplo.toml`, `deny.toml`, `rust-toolchain.toml` and a `commit-msg` hook, as `wend` has them
- [x] `cargo deny` found seven advisories in `zip` 7, and ones in `memmap2` and `crossbeam`. All fixed by updating (`zip` is now 9)
- [ ] Run the benchmarks in CI and keep the numbers, so a slowdown shows
- [ ] Run the `review` skill again on the whole tree: this change is large
- [ ] Publish: bump the crate version for the format and API breaks, and write the changelog
