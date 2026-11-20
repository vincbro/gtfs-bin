# TODO

What is done, what is open, and what is deliberately left out. Update it when the answer changes.

Status: `[x]` done and tested, `[ ]` open, `[-]` decided against, with the reason.

Measured on SL's regional feed (Trafiklab, 21,872 stops, 123,866 trips, 2.5M stop times): compile 3.8 s and 364 MB peak, output 44 MB, open and validate 9 ms, checksum 10 ms, 456 of 457 realtime trip updates matched. Benchmarks on a generated network of 1M stop times are in `crates/flatnet-core/benches/transit.rs`.

## Own schema and sources

The direction, decided 2026-11-20: flatnet becomes its own transit network format, built to make a network easy and fast to work with at any size. It does not aim to cover GTFS, GTFS-RT, NeTEx or SIRI. Each source is a reader behind a cargo feature, and a field is in the format because a consumer needs it, not because a source has it. Reasoning and the comparison of the four formats: `docs/OWN-SCHEMA.md`.

- [ ] Go through every field of every table and write down what reads it and what it costs. Drop what nothing reads, add what a consumer needs (`blaise` first). Measure size and speed on SL and the generated feed
- [x] Features, not crates (2026-11-20). The compiler takes a source-neutral `Network` (`network.rs`) and builds everything derived from it: patterns and their timings, shapes cut at stops, the grid, search, the id lookups, the metadata. Each source is a reader behind a cargo feature of `flatnet-compiler`, today `gtfs` (`gtfs::Reader`, on by default, which owns the CSV and zip parsing and resolves ids to positions), and later `netex` and `siri`. Without a reader feature the compiler has no `csv` or `zip` dependency and takes a `Network` you build. `Compiler::compile_gtfs(path)` reads and compiles. On SL, SL's dummy feed and the test fixture the output is byte for byte what the compiler wrote before the change. `flatnet-core` has the matching `gtfs-rt` feature (was `rt`). A `flatnet-model` crate for `Network` is only worth it if a reader has to live in a crate of its own
- [x] Realtime split (2026-11-20): `rt::Feed` holds source-neutral `Update`s (`Trip` with a `CallUpdate` per stop, `Vehicle`, `Alert`, `Remove`, `Unsupported`) that name trips and stops by the source's ids, and `RealtimeBuilder::apply_feeds` applies them with no wire format in sight. The GTFS-realtime decoder is `rt::to_feed` behind the `gtfs-rt` feature, which also keeps `apply` and `build` taking `FeedMessage`s. Alert cause, effect and severity and the vehicle's stop status and occupancy are own enums now. New `realtime` feature of `flatnet-core` (needs `jiff`), `gtfs-rt` builds on it. Still open: an event has a `delay` and a `time` but no separate aimed, expected and actual time. SIRI's estimated calls need that
- [ ] Schema pass, agreed 2026-11-20. On SL, `stop_times` is 100 MB of the 132 MB file (2.5M items × 40 B), `stop_to_trips` 10 MB, `trips` 8 MB. 124k trips share 3,710 patterns. In order, each measured on SL (size, open, full scan, a blaise query) before the next:
  - [x] Prototype measured on SL (2026-11-20, a throwaway that read the compiled file): grouping trips by stops, `stop_sequence`, boarding rules, headsign and distance gives 3,897 groups (3,710 patterns today), and splitting the 995 groups whose trips overtake gives 5,161 patterns (at most 7 chains in one group). The times are 20.0 MB at 8 B a cell and 15.0 MB at 6 B, plus 2.8 MB for 115k pattern positions at 24 B, against 100.2 MB of `StopTime`s: 22.8 MB or 17.8 MB. The file goes from 132 MB to about 55 MB, and about 45 MB once `stop_to_trips` is gone. Scanning every departure took 1,674 µs in `StopTime`s and 209 µs in a flat `u32` array
  - [x] Times per pattern (format 9): a pattern holds its stops once, with `sequence`, boarding rules, headsign and distance per position, and a dense table of times (`trips × positions`, trips sorted by departure) in 8 bytes a cell, or 6 with a `u16` dwell. A pattern splits when those fields differ between trips, and when trips overtake. Estimated 132 MB → about 55 MB. Realtime keys become trip plus position. Done 2026-11-20: `StopTime` is gone. A trip's `Timing`s (8 B) lie pattern by pattern in `timings`, and `pattern_stops` holds what the pattern says at each stop. SL is 5,184 patterns (the prototype said 5,161: overtaking is checked on arrivals and departures both), the file 132 MB → 54.5 MB, open and validate 44 ms → 10–16 ms, checksum 34 ms → 12–17 ms, compile still 3.9 s and the peak memory 364 MB (it was 480 MB). `timings_by_trip` and `pattern_stops_by_trip_pattern` replace `stop_times_by_trip`, and `Realtime::arrival_time` and `departure_time` take the trip, the slot and the `Timing`. blaise needs the same change. A hostile feed with thousands of trips that all overtake one another makes the chain search quadratic (compile time only)
  - [x] Split hot from cold (format 11): `Stop` is 28 B (was 52) with `StopInfo` (code, name, description) in `stop_infos`, and `Trip` is 40 B (was 64) with `TripInfo` (headsign, short name, block) in `trip_infos`, each at the position of its stop or trip. Reached by `Consumer::stop_info` and `trip_info`. The file is the same size (44.3 MB, 32 B more for the header), compile still 4 s and 340 MB. Still open: `route_index` stays in `Trip`, because a pattern doesn't know its route (trips of one route and another can share stops, headsigns and distances), so deriving it means the route joins the pattern key. blaise needs `stop_info` and `trip_info` for names
  - [x] A header flag per optional section (format 12): `Header.parts` holds `Parts` (`SHAPES`, `SEARCH`, `TEXT`, `IDS`), `Compiler::parts` chooses them and `flatnet --without shapes|search|text|ids` does the same. A part left out has empty sections, and `Consumer::parts` says what the file has. The consumer rejects a file that says a part is absent and holds it, an unknown bit, and search without text. On SL (44.3 MB): without shapes 38.9 MB, without text 43.9 MB, without ids 41.2 MB, all three 35.4 MB, which opens in 6 ms. Without text a headsign is not part of what makes a pattern. Without ids `*_by_id` find nothing, so realtime matches no trip. Search needs text. Still open: the stop grid is always written
  - [x] Dropped `stop_to_trips` (10 MB, format 10): the file is 44.3 MB and opens in about 9 ms. `iter_trips_by_stop` walks the patterns that visit the stop and their trips, and `trip_patterns_by_stop` tells a router whether a stop has any trips. blaise called `trips_by_stop` twice, both as "has trips"
  - [x] `Service` without `weekday` (format 13): nothing in the consumer read it, the compiler only needs it to fill the mask, so it builds the mask and doesn't store it. A service is 28 B (was 32). `start_date` and `end_date` stay: `is_service_active`, the metadata's active range and the viewer read them, and the mask's slice includes padding up to a byte, so the end can't be derived from it
  - [x] Footpaths (format 14): walks between places only, decided 2026-11-20. A place is the top of a stop's parent chain (`Place::root_of`): a station with the stops under it, or a stop with no parent. Walking between the stops of one place is a given and has no footpath, so SL's 99,390 stop to stop walks became 19,434 place to place walks (24% were inside one station, and a neighboring station was listed once for each of its stops). Tables `places` (one `Place` for each stop: its stops and its footpaths, empty unless the stop is a place), `place_stops` and `footpaths` (the place it ends at and the meters). A footpath is the straight line in meters between the nearest stops of two places. There is no detour: it is the same as a shorter reach, so the file holds raw meters (decided 2026-11-20). Reached by `Consumer::place_of`, `stops_by_place`, `footpaths_by_place` and `footpaths_by_stop`. Stations, entrances and generic nodes are not among a place's stops. Every choice is a setting kept in the file's `Metadata` as `Settings`, so a reader knows what the tables mean: `footpath_max_meters` (400, a straight line) and `footpath_max_per_place` (32), set with `Compiler::settings` or `--footpath-meters` and `--footpath-max`, and read back with `Consumer::settings`. The consumer rejects settings the compiler would refuse, and a place that lists a stop of another place. `Parts::FOOTPATHS` leaves all of it out. On SL the defaults add 0.56 MB (44.3 → 44.9 MB) and no compile time. The walk inside a place is not a setting and not in the file: a router assumes it, or reads `transfers.txt`. The file holds straight meters, no detour and no walking speed, and no helper turns them into a walk or a time: the reader does, with `Coordinate::distance_meters` for a pair the file has no footpath for. The consumer checks each place's footpaths are nearest first. A place that lost a neighbor to the per-place limit can still be in that neighbor's list, so the footpaths are not always symmetric. Still open: `Settings` for the other compile choices (`Parts` is in the header, and interpolation and the grid cell size are fixed)
  - [x] Trip start time and a route index by start (format 15): a trip's start is its departure from the first stop, or its arrival there, derived and not stored on `Trip` (`Consumer::trip_start`). `route_to_trips` is now sorted within a route by start and then trip index, and `route_trip_starts` holds each start at the same position, so `Consumer::trips_by_route_start(route, start)` is a binary search over contiguous `u32`s and gives the run of trips that start then. Direction is not in the sort: a trip with no direction fits any, so the run for a start is contiguous and the caller filters it by direction and by service day. The realtime builder uses it for trips named by route, start time, date and direction. On SL: 22 ns a query against 2.6 µs for the scan of the route's trips (about 120 times), the same answers, and 0.5 MB more in the file (44.9 → 45.4 MB). A run key is a trip plus a service day, and `Realtime` is already one per service day, so no type is added for it. Two trips of one route, direction and service that start at the same minute are still ambiguous, because the feed names nothing to tell them apart: the index makes them cheap to find, not unique. The consumer checks each start is the trip's own, belongs to its route, and the order
  - [ ] Ids with a namespace, and several sources in one file, for a national network
  - [-] Until a consumer asks: a line above the route, operator apart from authority, a stop place apart from the stop point, validity per object, accessibility values
- [ ] `frequencies.txt` and `timepoint` are decided in the schema, once, for every source (see Parser)
- [ ] A `netex` feature of `flatnet-compiler`: a reader for the Nordic profile only
- [ ] A `siri` feature of `flatnet-core`: a decoder, estimated timetable first
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
