# The binary format

A compiled file is a header followed by sections. Everything is little-endian on the machine that wrote it: the file is meant to be mapped, so a file moves between machines of the same byte order only.

Version: `FLATNET_VERSION` in `crates/flatnet-core/src/lib.rs`, currently 12. It changes whenever a table or a header field does, and `Consumer::new` refuses any other version.

## Header

`Header` in `crates/flatnet-core/src/models/header.rs`, `repr(C)`:

| Field | Size | Meaning |
|---|---|---|
| `magic` | 4 | The bytes `FLNT` |
| `version` | 4 | The format version |
| `parts` | 8 | The optional parts the compiler wrote, as bits of `Parts`: shapes, search, text and ids. A part that is not set has empty sections, and the consumer rejects a file where it is not |
| 40 `Section` fields | 16 each | `offset` and `count`, both `u64`, of each section below |
| `feed_start_date`, `feed_end_date` | 4 each | `Date`, or `u32::MAX` when `feed_info.txt` has none |
| `file_length` | 8 | The length of the whole file, so a truncated one is noticed on open |
| `checksum` | 8 | `models::checksum` of every byte after the header. `Consumer::verify_checksum` checks it |

A section's `count` is the number of items, not bytes. Every section starts on an 8 byte boundary, and the gap before it is zero.

## Sections

Every table is an array of fixed size `repr(C)` structs, with no pointers: they refer to each other by position.

| Section | Item | Meaning |
|---|---|---|
| `stops`, `routes`, `trips`, `services` | `Stop`, `Route`, `Trip`, `Service` | The GTFS entities, in file order. An item's `index` field is its position. A `Stop` and a `Trip` hold what routing reads, 28 and 40 bytes |
| `stop_infos`, `trip_infos` | `StopInfo`, `TripInfo` | The text of each stop (code, name, description) and trip (headsign, short name, block), at the position of the stop or trip. Kept apart so a scan over the stops or trips doesn't read it |
| `timings` | `Timing` | An arrival and a departure. Pattern by pattern, and within a pattern trip by trip in the order the trips leave. `Trip.stop_times` is a trip's run, one `Timing` for each stop of its pattern |
| `shapes` | `Shape` | The path between two stops: a slice of `shape_points` and its length in meters. Each one is stored once, however many trips use it |
| `shape_points` | `Coordinate` | The points of every `Shape`, back to back |
| `trip_shapes` | `ShapeIndex` | For each trip, the `Shape` of every pair of stops it calls at in a row, back to back. `Trip.shape` is the slice of it for one trip |
| `*_ids` | bytes | The GTFS ids as UTF-8, laid out in index order |
| `*_id_lookup` | index | Indices sorted by id, for binary search |
| `strings` | bytes | Every name, headsign and description as UTF-8, each stored once |
| `active_mask` | bytes | One bit per service day, each service starting on a byte boundary |
| `route_to_trips` | `TripIndex` | Trips grouped by route, with a `route_to_trips_lookup` of slices by route. The trips that call at a stop are those of the patterns in `stop_to_trip_pattern` |
| `trip_patterns` and its four lookups | `TripPattern` | Trips that call at the same stops in the same order, with the same sequence numbers, boarding rules, headsigns and distances, and that never overtake each other. A pattern is the run of `trip_patterns_stop_seq` for its stops, the run of `pattern_stops` at the same positions, and its trips in the order they leave |
| `pattern_stops` | `PatternStop` | What a pattern says at each of its stops: the sequence, boarding rules, headsign and distance traveled. Stored once for the pattern, not for every trip |
| `transfers` and its lookups | `Transfer` | Sorted by the stop they start at, with slices by stop in both directions |
| `search_stops`, `search_to_stops` | `SearchStop` | Named places and the stops under each |
| `stop_grid`, `grid_stops` | `StopCell` | A grid of 0.005 degree cells, sorted, for finding stops near a position |
| `timezone` | bytes | The agency's time zone name, such as `Europe/Stockholm`, or empty |
| `metadata` | `Metadata` | Exactly one item, see below |
| `metadata_json` | bytes | A JSON object as UTF-8 with what the compiler was given besides the fixed fields, or empty |

## Metadata

The one `Metadata` item is `repr(C)`, 88 bytes: `compiled_at` (`u64`, seconds since the unix epoch, 0 if the compiler was not told), `active_from` and `active_until` (`Date`, the first and last day any service runs, both `NONE` for a feed with no services), then nine `Optional<StringSlice>` into `strings`: `compiler_version`, `source`, `publisher_name`, `publisher_url`, `language`, `feed_version`, `contact_email`, `agency_name` and `agency_url`.

The active range is what `Consumer::is_stale(today)` compares with, so a launch can check a file without reading a table. The compiler writes the JSON compact, with its keys sorted, so the same input gives the same bytes. The compiler never reads the clock: `compiled_at` and `source` are only set when the caller passes them, so the same feed gives the same bytes whether it is a folder or a zip.

## Conventions

- **Absent values** are a reserved value of the type (`Sentinel::NONE`), wrapped in `Optional<T>` where a field may be absent. `Optional<T>` is the same size as `T`.
- **Slices** are a `start` and a `count` into another section. `(0, 0)` is both "none" and "empty".
- **Padding** is an explicit zero field, because `Pod` forbids implicit padding.
- **Times** are seconds since the start of the service day, and may pass 86,400. **Dates** are days since 1970-01-01.
- **Coordinates** are degrees times 10,000,000 as `i32`.

## What `Consumer::new` guarantees

It checks the header, the length, that every section is in bounds and aligned, that text is UTF-8 and slices end on characters, that every index and slice points inside the table it names, that each item's `index` is its position, that the id lookups are strictly sorted, and that each pattern's sequence numbers rise, that its trips lie back to back in `timings` starting where the pattern before ends, each with a `Timing` for every stop of the pattern, that no trip is earlier than the one before it at any stop, and that no `Timing` is left over, that each shape has points and a length, and that a trip with shapes has one fewer than it has stop times. For the metadata: that it holds exactly one item, that its text slices lie inside `strings`, that the JSON is UTF-8, and that the active range has both ends or neither and doesn't run backwards. The property tests in `crates/flatnet-core/tests/fuzz.rs` damage files in the ways of the tables, and `tests/shapes.rs` and `tests/metadata.rs` damage a file once for each check of the shapes and the metadata. Each fails if its check is removed.

`Consumer::new_unvalidated` skips the table checks. It never causes undefined behavior, but a damaged file can make an accessor panic.
