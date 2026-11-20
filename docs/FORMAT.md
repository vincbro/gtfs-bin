# The binary format

A compiled file is a header followed by sections. Everything is little-endian on the machine that wrote it: the file is meant to be mapped, so a file moves between machines of the same byte order only.

Version: `GTFS_BIN_VERSION` in `src/lib.rs`, currently 5. It changes whenever a table or a header field does, and `Consumer::new` refuses any other version.

## Header

`Header` in `src/models/header.rs`, `repr(C)`:

| Field | Size | Meaning |
|---|---|---|
| `magic` | 4 | The bytes `GTFS` |
| `version` | 4 | The format version |
| 35 `Section` fields | 16 each | `offset` and `count`, both `u64`, of each section below |
| `feed_start_date`, `feed_end_date` | 4 each | `Date`, or `u32::MAX` when `feed_info.txt` has none |
| `file_length` | 8 | The length of the whole file, so a truncated one is noticed on open |
| `checksum` | 8 | `models::checksum` of every byte after the header. `Consumer::verify_checksum` checks it |

A section's `count` is the number of items, not bytes. Every section starts on an 8 byte boundary, and the gap before it is zero.

## Sections

Every table is an array of fixed size `repr(C)` structs, with no pointers: they refer to each other by position.

| Section | Item | Meaning |
|---|---|---|
| `stops`, `routes`, `trips`, `services` | `Stop`, `Route`, `Trip`, `Service` | The GTFS entities, in file order. An item's `index` field is its position |
| `stop_times` | `StopTime` | Sorted by trip, then by sequence. Each trip points at its run |
| `shapes` | `Shape` | The points of every shape, back to back |
| `*_ids` | bytes | The GTFS ids as UTF-8, laid out in index order |
| `*_id_lookup` | index | Indices sorted by id, for binary search |
| `strings` | bytes | Every name, headsign and description as UTF-8, each stored once |
| `active_mask` | bytes | One bit per service day, each service starting on a byte boundary |
| `route_to_trips`, `stop_to_trips` | `TripIndex` | Trips grouped by route and by stop, with a `*_lookup` of slices by key |
| `trip_patterns` and its four lookups | | Trips that visit the same stops in the same order, grouped |
| `transfers` and its lookups | `Transfer` | Sorted by the stop they start at, with slices by stop in both directions |
| `search_stops`, `search_to_stops` | `SearchStop` | Named places and the stops under each |
| `stop_grid`, `grid_stops` | `StopCell` | A grid of 0.005 degree cells, sorted, for finding stops near a position |
| `timezone` | bytes | The agency's time zone name, such as `Europe/Stockholm`, or empty |

## Conventions

- **Absent values** are a reserved value of the type (`Sentinel::NONE`), wrapped in `Optional<T>` where a field may be absent. `Optional<T>` is the same size as `T`.
- **Slices** are a `start` and a `count` into another section. `(0, 0)` is both "none" and "empty".
- **Padding** is an explicit zero field, because `Pod` forbids implicit padding.
- **Times** are seconds since the start of the service day, and may pass 86,400. **Dates** are days since 1970-01-01.
- **Coordinates** are degrees times 10,000,000 as `i32`.

## What `Consumer::new` guarantees

It checks the header, the length, that every section is in bounds and aligned, that text is UTF-8 and slices end on characters, that every index and slice points inside the table it names, that each item's `index` is its position, that the id lookups are strictly sorted, and that each trip's stop times are its own and in sequence order. The property tests in `tests/fuzz.rs` damage files in all of these ways, and fail if any one of the checks is removed.

`Consumer::new_unvalidated` skips the table checks. It never causes undefined behavior, but a damaged file can make an accessor panic.
