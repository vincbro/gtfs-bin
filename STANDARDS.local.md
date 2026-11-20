# Project standards

What is specific to this project. The shared rules are in `code-standards/`. Where this file disagrees with them, this file wins.

## Commands

All through `just`, which `just toolchain` sets up (the pinned Rust, the pinned nightly `rustfmt`, the cargo tools and the git hooks).

- **Gate** (must pass before any change is done): `just check`, which runs `fmt-check`, `clippy`, `doc`, `test`, `features` and `deps`
- **Full gate** (the gate with every property test run for 40000 cases, for after touching the validators, the compiler or the realtime builder): `just check-full`
- **Commit-message check** (takes a message file): `just commit-msg <file>`, which the `commit-msg` hook runs
- **Real data** (optional): `just real-data <folder with static.zip and *.pb>`
- **Benchmarks** (for P-MEASURE): `just bench`

## Hot paths

The code a router runs per query, by name. `PERFORMANCE.md` rules about hot paths apply to these and what they call.

- The `Consumer` accessors in `crates/flatnet-core/src/consumer/` that return slices (`timings_by_trip`, `pattern_stops_by_trip_pattern`, `trip_patterns_by_stop`, `trips_by_route`, `outbound_transfers_by_stop`, `trip_patterns_by_stop`) and the `*_by_id` lookups.
- `Consumer::stops_near`, which fills a buffer it is given.
- `Realtime` getters and `arrival_time`/`departure_time`, which a router calls per stop time.

`RealtimeBuilder::apply` runs once per feed, a few seconds apart, so it is not a hot path.

## Known performance opportunities

Measured, not yet changed. A review doesn't repeat these. It reports one only if a change makes it worse, or notes when a change fixes one.

| Where | What | Numbers |
|---|---|---|
| `Consumer::search_by_name` | Folds every name on each call, so it allocates per name | 0.9 ms over 7,000 places. Fine for a search box, not for a loop |
| `Consumer::new` | Reads every table once to validate | 4.8 ms for 1M stop times, 10 ms for SL (2.5M). `new_unvalidated` takes 44 ns |
| `SliceBuilder::add` | Stores each unique string twice (the buffer and the map key) | Compile time only. 3 s for SL, 480 MB peak |

## Overrides

Rules from `code-standards/` this project bends, each with the reason.

- **S-NAMES-TYPES / S-NAMES-ABBREVIATIONS**: none now. `Idx` and `Opt` were spelled out.
- **S-TESTS**: integration tests in `crates/flatnet-core/tests/` aren't in a `mod tests`, because that is where the standards put them. `clippy::tests_outside_test_module` is therefore left off in `Cargo.toml`.
- **S-VISIBILITY**: `unreachable_pub` is on, so the helper modules shared by the integration tests use `pub(crate)`.
- **S-INVARIANTS**: `Consumer` is immutable and built from untrusted bytes, so its invariants are checked by `validate` with a returned error, not by a debug-only `check_invariants`. `Realtime` has the debug-only check, and `crates/flatnet-core/tests/fuzz_rt.rs` drives it.
- **rustfmt options** (`imports_granularity`, `group_imports`) need nightly, so formatting runs through `just fmt` on the nightly pinned in the `justfile`. A plain `cargo fmt` on stable warns and skips them.
