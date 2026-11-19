# Project standards

What is specific to this project. The shared rules are in `code-standards/`. Where this file disagrees with them, this file wins.

## Commands

- **Gate** (must pass before any change is done): `cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features`
