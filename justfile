set shell := ["bash", "-uc"]

# rustfmt's import options (code-standards/rust/STYLE.md S-IMPORTS) are nightly-only, so formatting
# runs on this pinned nightly. Bump it like the toolchain: in its own change.
fmt_toolchain := "nightly-2026-10-13"

# List available recipes
default:
    @just --list

# Compile a feed: just compile path/to/gtfs.zip [output.flatnet]
compile feed output="output.flatnet":
    cargo run --release -p flatnet-cli -- -i {{ quote(feed) }} -o {{ quote(output) }}

# Open a compiled file and time it, optionally with a query: just inspect network.flatnet [near LAT LON METERS | search TEXT]
inspect file *query:
    cargo run --release -p flatnet-cli --example inspect -- {{ quote(file) }} {{ query }}

# Open a compiled file in a window, to walk its tables and see it on a map. With no file it asks
# for one in the system's dialog: just view [network.flatnet]
view *file:
    cargo run --release -p flatnet-viewer -- {{ file }}

# Compile the dummy feed in data/dummy into data/dummy.flatnet, a small network to try the viewer on
dummy:
    cargo run --release -p flatnet-cli -- -i data/dummy -o data/dummy.flatnet

# Apply realtime files to a compiled file and report what matched: just apply network.flatnet TripUpdates.pb [more.pb ...]
apply file *feeds:
    cargo run --release -p flatnet-cli --example apply_feed -- {{ quote(file) }} {{ feeds }}

# Build everything
build:
    cargo build --workspace --all-targets --all-features

# Build the binary we ship (fat LTO, stripped) into target/dist/flatnet
dist:
    cargo build --profile dist -p flatnet-cli --locked

# Format every Rust and TOML file, and sort Cargo.toml dependency tables
fmt:
    cargo +{{ fmt_toolchain }} fmt --all
    cargo sort --workspace --no-format
    taplo fmt

# Apply clippy's automatic fixes, then format
fix:
    cargo clippy --fix --allow-dirty --allow-staged --workspace --all-targets --all-features
    cargo +{{ fmt_toolchain }} fmt --all

# The CI gate: run it before you push
check: fmt-check clippy doc test features deps

# The CI gate with every property test run for 40000 cases instead of 256, which takes
# a few minutes: run it before a release and after touching the validators, the compiler
# or the realtime builder
check-full: fmt-check clippy doc test-full features deps

# Fail on any file `just fmt` would change
fmt-check:
    cargo +{{ fmt_toolchain }} fmt --all -- --check
    cargo sort --workspace --no-format --check
    taplo fmt --check

# Fail on any lint
clippy:
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

# Build the docs; any rustdoc warning fails
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --document-private-items --locked

# Run every test
test:
    cargo test --workspace --all-features --locked

# Run every test, the property tests for 40000 cases each
test-full:
    PROPTEST_CASES=40000 cargo test --workspace --all-features --locked --release

# Run the tests against a real feed: a folder with static.zip and any number of .pb realtime
# files: just real-data path/to/folder
real-data folder:
    FLATNET_REAL_FEED={{ quote(folder) }} cargo test --release -p flatnet-core --all-features --test scale -- --nocapture

# Build each crate with each feature on its own, as a user of one of them would
features:
    cargo build -p flatnet-compiler --locked
    for features in "" consumer consumer,rt; do cargo build -p flatnet-core --no-default-features --features "$features" --locked || exit 1; done

# Time compiling, opening and querying a generated network of a million stop times.
# Extra args go to criterion: just bench [args]
bench *args:
    cargo bench -p flatnet-core --bench transit -- {{ args }}

# Fail on unused dependencies, or on any cargo-deny check in deny.toml:
# advisories, licenses, sources
deps:
    cargo machete
    cargo deny --locked check

# Build the docs, private items included, and open them
docs:
    cargo doc --workspace --all-features --no-deps --document-private-items
    open target/doc/flatnet_core/index.html

# Set the version of every crate: just bump 0.2.0
bump version:
    cargo set-version --workspace {{ quote(version) }}

# Install the pinned toolchain from rust-toolchain.toml and the nightly rustfmt
# `just fmt` uses, and off CI the cargo tools other recipes call (CI installs
# those prebuilt) and the git hooks
toolchain:
    rustup toolchain install
    rustup toolchain install {{ fmt_toolchain }} --profile minimal --component rustfmt
    @[ -n "${CI:-}" ] || just _tools hooks

# Use the hooks in .githooks: commit-msg checks code-standards/COMMITS.md S-COMMITS
hooks:
    git config core.hooksPath .githooks

# Fail on a commit message that breaks code-standards/COMMITS.md S-COMMITS: just commit-msg <file>
commit-msg file:
    #!/usr/bin/env bash
    set -euo pipefail
    # Comments and the diff `git commit -v` appends aren't part of the message
    message=$(sed '/^# ------------------------ >8 ------------------------$/,$d' {{ quote(file) }} | grep -v '^#' || true)
    subject=$(printf '%s\n' "$message" | head -n 1)
    fail() { printf 'commit message: %s\n  %s\nSee code-standards/COMMITS.md S-COMMITS.\n' "$1" "$subject" >&2; exit 1; }
    # Merges and fixups are rewritten or generated, not written
    [[ "$subject" =~ ^(Merge|fixup!|squash!|amend!) ]] && exit 0
    [[ "$subject" =~ ^[a-z0-9][a-z0-9_/-]*(,\ [a-z0-9][a-z0-9_/-]*)*:\ [^\ ] ]] || fail "first line must be 'area: summary', area lowercase"
    summary=${subject#*: }
    [[ "$summary" =~ ^[a-z0-9\`] ]] || fail "summary must start lowercase (or with \`code\`)"
    [[ "$subject" != *. ]] || fail "first line must not end with a period"
    (( ${#subject} <= 72 )) || fail "first line is ${#subject} characters, at most 72"
    second=$(printf '%s\n' "$message" | sed -n 2p)
    [[ -z "$second" ]] || fail "second line must be blank"

# Check every commit since `base` against code-standards/COMMITS.md S-COMMITS: just commits [base]
commits base="origin/main":
    #!/usr/bin/env bash
    set -euo pipefail
    status=0
    for commit in $(git rev-list --no-merges {{ quote(base) }}..HEAD); do
        git log -1 --format=%B "$commit" > "${TMPDIR:-/tmp}/flatnet-commit-msg"
        just commit-msg "${TMPDIR:-/tmp}/flatnet-commit-msg" || { echo "  in $(git log -1 --format=%h "$commit")" >&2; status=1; }
    done
    exit $status

# Each tool only if it's missing: command:crate
_tools:
    for tool in cargo-set-version:cargo-edit cargo-machete:cargo-machete cargo-deny:cargo-deny cargo-sort:cargo-sort taplo:taplo-cli; do command -v "${tool%%:*}" >/dev/null || cargo install --locked "${tool#*:}"; done
