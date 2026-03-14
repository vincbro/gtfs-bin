#!/usr/bin/env bash

rm tests/fixtures/gtfs.zip
zip -r tests/fixtures/gtfs.zip tests/fixtures/gtfs
cargo test
