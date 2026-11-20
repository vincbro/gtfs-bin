# Contributing to flatnet

Thanks for taking an interest. For anything bigger than a bug fix, please open an issue first, so we can check it fits before you put time into it.

## Setup

You need [rustup](https://rustup.rs), [just](https://github.com/casey/just) and `protoc` (`brew install protobuf` or `apt install protobuf-compiler`), which the `gtfs-rt` dependency needs to build.

The coding rules are in the `code-standards/` submodule: clone with `--recurse-submodules`, or run `git submodule update --init` in a clone you already have. Building flatnet doesn't need it, reading and reviewing code does.

```sh
just toolchain          # installs the pinned Rust, nightly rustfmt, the cargo tools and the git hooks
just compile FEED       # compiles a feed (a zip or a folder) into output.flnt
just inspect FILE       # opens a compiled file, times it, and can query it
just check              # what CI runs: fmt, clippy, docs, tests, features, dependencies
just check-full         # the same, with the property tests run for 40000 cases
```

Run `just` on its own to list every recipe.

## Layout

A workspace of three crates under `crates/`, all versioned together:

- `flatnet-core`: what reads the file.
  - `src/models/`: the fixed-size types the compiled file is made of, and the sentinel and index types they use.
  - `src/consumer/`: maps the file, checks all of it (`validate`) and answers queries.
  - `src/rt/`: applies realtime updates on top of a consumer (`update.rs` holds the source-neutral types, `gtfs.rs` the GTFS-realtime decoder).
  - `tests/`: integration and property tests, and `tests/common/` for what they share, including a generated feed. They compile feeds with `flatnet-compiler`, a dev-dependency.
  - `benches/`: the criterion benchmarks.
- `flatnet-compiler`: turns a `Network` (`network.rs`) into the file (one builder per table, `writer`), and with the `gtfs` feature reads a GTFS feed into one (`gtfs.rs` and `gtfs/`: `source`, `table`, `records`).
- `flatnet`: the command line tool and its examples.
- `docs/FORMAT.md`: the layout of the file. Change it in the same commit as the layout.

## Style

The coding rules are shared by every project through the `code-standards/` submodule. [code-standards/rust/STYLE.md](code-standards/rust/STYLE.md) is how Rust is written: layout, naming, errors, API shape and tests, each rule with its reason. Most rules are enforced by the checks below. The rest are flagged in review, citing the rule's `S-…` id.

[code-standards/rust/PERFORMANCE.md](code-standards/rust/PERFORMANCE.md) is the same for the code a router runs on every query: what may allocate, what is borrowed and what is measured, each rule with an id (`P-…`) and a reason.

[STANDARDS.local.md](STANDARDS.local.md) holds what is specific to flatnet: the gate, the hot paths, the performance problems that are measured and not yet fixed, and the overrides of the shared rules.

To check a change against the rules tools can't enforce, ask your coding agent to run the `review` skill (`.claude/skills/review/SKILL.md`). It runs `just check`, then checks the code against both rule files, and lists what it found without changing anything. The `commit` skill does the same before it commits. Any agent that reads [AGENTS.md](AGENTS.md) finds them.

## Before you push

`just check` needs to pass before you push. It checks formatting, clippy and rustdoc warnings, tests, each feature on its own, and dependencies. On a pull request, CI also checks every commit message (see [Commits](#commits)).

## Tests

- **Unit tests** sit at the bottom of the file they test, in `mod tests`, named for the behavior.
- **Integration tests** in `crates/flatnet-core/tests/` drive the public API: `parser.rs` reads feeds, `features.rs` the file's tables and queries, `rt_full.rs` realtime.
- **Property tests** (`proptest`) damage files, feeds and realtime messages and check that nothing panics and that what the compiler writes the consumer reads: `fuzz.rs`, `fuzz_compiler.rs`, `fuzz_rt.rs`. `fuzz_shapes.rs` compiles random networks and checks that each trip's shapes join end to end, `fuzz_patterns.rs` that a trip reads back the times and rules its feed gave it and that a pattern's trips leave in order, and `fuzz_metadata.rs` that any JSON the compiler is given comes back merged or is an error. A saved failing case goes in the `*.proptest-regressions` file beside the test, and is committed. Before you trust a new check, break the code on purpose once and see the test fail. `just test-full` runs them for 40000 cases.
- **Real data** is tried with `just real-data FOLDER`, where the folder holds `static.zip` and any realtime `*.pb` files. The test does nothing without it, so nothing real is committed.

## Benchmarks

`just bench` runs the benchmarks with [criterion](https://github.com/criterion-rs/criterion.rs). They live in `crates/flatnet-core/benches/transit.rs` and time compiling, opening a file with and without its checks, scanning every stop time, looking a trip up by id, finding stops near a position, searching by name, and applying a realtime feed. Each is a group with one entry per size, over generated networks of about 12 thousand, 120 thousand and 1.2 million stop times. criterion compares each run with the previous one on the same machine. To compare against a fixed point, save a baseline:

```sh
just bench --save-baseline before
just bench --baseline before
```

Benchmarks aren't part of `just check`, but clippy lints them like any other code. A change made for speed puts its numbers in the commit message (code-standards/rust/PERFORMANCE.md P-MEASURE).

## Dependencies

`just deps` runs:

- `cargo machete`, which fails on dependencies declared but never used.
- `cargo deny` (configured in `deny.toml`), which fails on crates with known vulnerabilities, licenses that don't fit MIT OR Apache-2.0, and sources other than crates.io. It also reports duplicate versions in the tree, without failing.

`anyhow` is only for the command line tool and the examples. The library returns typed `thiserror` errors (code-standards/rust/STYLE.md S-ERRORS).

`just fix` applies clippy's automatic fixes and formats, which clears most lints on its own. If you're stuck on a lint, push anyway and say so in the PR: a maintainer can fix it up, and a lint should never be the reason a good change doesn't land.

## Lints

The lint policy lives in `[workspace.lints]` in the root `Cargo.toml`. In short:

- clippy's `all`, `pedantic` and `cargo` groups are on, plus a set of `restriction` lints chosen to enforce [STYLE.md](code-standards/rust/STYLE.md). Each is listed with the `S-…` rule it enforces. Thresholds, such as nesting depth and the banned names, live in `clippy.toml`.
- **No `unwrap()` or `panic!` outside tests.** Use `?`, or `expect("...")` stating the invariant when one really holds. A public function that can panic says so under `# Panics`.
- **Docs where they add insight, not everywhere.** Document what a type is for, what a function promises, and when it panics. Don't restate what names and types already say (code-standards/rust/STYLE.md S-DOCS).
- **Fields are all public or all private.** Stateful structs keep every field private (code-standards/rust/STYLE.md S-FIELDS).
- **The narrowest visibility that works.** `unreachable_pub` is on, so an item that nothing outside the crate can reach is `pub(crate)` or `pub(super)`.
- **Code stays flat.** Nothing nests deeper than 4 blocks (code-standards/rust/STYLE.md S-FLAT).
- **Unsafe is opt-in per site.** Mark it with `#[allow(unsafe_code, reason = "...")]` and put a `// SAFETY:` comment on every `unsafe` block.
- **Escape hatches are local.** Silence a lint with `#[allow(lint)]` on the smallest item that needs it, never crate-wide.

The toolchain is pinned in `rust-toolchain.toml`, so everyone sees the same lints. A toolchain bump goes in its own change, which fixes or allows (with a reason) any new lints it brings.

## Formatting

`just fmt` formats everything, and `just check` fails on anything it would change:

- Rust: rustfmt with the options in `rustfmt.toml`. The import options (code-standards/rust/STYLE.md S-IMPORTS) are nightly-only, so `just fmt` runs the nightly pinned as `fmt_toolchain` in the `justfile`, and `just toolchain` installs it. A plain `cargo fmt` on stable warns and skips them.
- `Cargo.toml` dependency tables: sorted alphabetically by `cargo sort`.
- Every TOML file: formatted by `taplo` (`taplo.toml`). It never reorders keys, so hand-grouped tables such as the lints keep their order.

## Commits

Commit messages follow code-standards/COMMITS.md S-COMMITS: `area: lowercase imperative summary`, at most 72 characters with no trailing period, then a blank line and a body saying why.

```
consumer: check every index when a file is opened

A file with an index past the end of its table made an accessor panic.
Check each index and slice against the table it points into, and
return an error from `Consumer::new` instead.
```

`just hooks`, run by `just toolchain`, installs a commit-msg hook that rejects anything else, and CI runs the same check (`just commits`) on every commit in a pull request.

## Changing the file format

The compiled file has a version (`FLATNET_VERSION`). Any change to a table, a header field or what a field means bumps it, updates `docs/FORMAT.md`, adds a validator in `crates/flatnet-core/src/consumer/validate.rs`, and extends the property tests so that removing the validator makes one fail.

## Releases

Change the version with `just bump <version>` (`cargo set-version --workspace`), never by hand. The shipped binary is built with `just dist` (the `dist` profile: fat LTO, one codegen unit, stripped). `cargo build --release` stays quick for day-to-day profiling.

## License

Unless you say otherwise, any contribution you submit for inclusion in flatnet is dual-licensed as MIT OR Apache-2.0, as described in the README, with no additional terms or conditions.
