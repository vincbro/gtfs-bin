# Troubleshooting

Find your message, read what it means, and do the thing in the last column.

- [Opening a file](#opening-a-file)
- [Compile errors](#compile-errors)
- [Realtime does not match](#realtime-does-not-match)
- [Queries that find nothing](#queries-that-find-nothing)
- [Panics](#panics)
- [Questions](#questions)

## Opening a file

`Consumer::new` returns `flatnet_core::consumer::Error`.

| Message | Means | Do this |
|---|---|---|
| `file is too small to contain a valid header` | Fewer bytes than a header. Often an empty or half-downloaded file. | Download again. Check the length. |
| `invalid magic number: not a compiled flatnet file` | The first four bytes aren't `FLNT`. | You opened another kind of file. Compile your feed with `flatnet`. |
| `unsupported flatnet version: expected 15, got N` | The file was compiled by a different version of flatnet. | Compile again with the same version you read with. A file is never migrated. |
| `file is N bytes but the header says M` | The file is truncated or has extra bytes. | Download or copy again. |
| `section is out of bounds` | A section of the header points past the end of the file. | The file is damaged. Download or compile again. |
| `section is misaligned or not a whole number of items` | A section's size isn't a whole number of records. | The file is damaged. |
| `<name> section is not valid utf-8` | An id or string section has bytes that aren't text. | The file is damaged. |
| `<field> of <table> points outside what it refers to` | An index or slice inside a table is out of range. | The file is damaged, or was made by a buggy compiler. Please report it with the file if it came from `flatnet`. |
| `<lookup> does not have one entry per item of <table>` | A lookup table is the wrong length. | The file is damaged. |
| `<table> is not in the order its lookups need` | A table that is meant to be sorted isn't. | The file is damaged. |
| `the checksum does not match, so the file is damaged` | `verify_checksum`: a byte after the header changed. | Download or copy again. |
| `the operating system refused the advice` | `consumer::advise` was refused. | Safe to ignore. It is only a hint. |

Anything other than the first four means the bytes were changed after the compiler wrote them. To see which case you have, run `verify_checksum` on the file with `Consumer::new_unvalidated`: if the checksum fails, the file changed on disk or in transit.

```rust
use flatnet_core::consumer::{Consumer, Error};
use memmap2::Mmap;

fn diagnose(mmap: &Mmap) {
    match Consumer::new(mmap) {
        Ok(_) => println!("fine"),
        Err(Error::UnsupportedVersion { expected, actual }) => {
            println!("recompile: this build reads version {expected}, the file is {actual}");
        }
        Err(error) => {
            println!("{error}");
            if let Ok(consumer) = Consumer::new_unvalidated(mmap) {
                println!("checksum ok: {}", consumer.verify_checksum().is_ok());
            }
        }
    }
}
```

## Compile errors

`flatnet_compiler::Error`. The `flatnet` command prints the message with the file it was about.

### Reading a feed

| Message | Means | Do this |
|---|---|---|
| `failed to read <path>` | The file or folder can't be opened. | Check the path and permissions. |
| `failed to read the zip archive` | The input isn't a valid zip. | Check the file. Try unzipping it by hand. |
| `<file> is not valid csv` | A `.txt` file is not parseable as csv. | Open it. Look for an unclosed quote. |
| `the feed has no <file>` | A required file is missing: `stops.txt`, `routes.txt`, `trips.txt` or `stop_times.txt`, and a calendar. | Check the feed. The file may be in a folder inside the zip: that is fine. |
| `<file> has no <column> column` | A required column is missing from the header. | Check the header row. Lenient mode doesn't help: there is nothing to read. |
| `<file> line N: invalid <column> "<value>"` | A value is malformed: a time like `soon`, a date that isn't a real day or is past 9999, a latitude outside ±90 or a longitude outside ±180. | Fix the feed, or compile with `Reader::lenient()` to leave that row out and report it. |
| `service "<id>" ends before it starts` | A calendar row runs backwards. | Fix the feed. Lenient mode drops the service. |

### Compiling a network

| Message | Means | Do this |
|---|---|---|
| `the network has two <table> with the id "<id>"` | Two stops, routes, trips or services share an id. | Make ids unique when you build the `Network`. The GTFS reader keeps the first. |
| `<field> of the network's <table> points at nothing` | A position is out of range, such as a trip whose `route` doesn't exist. | Resolve ids to positions in the vectors you pass. Leave out what points at nothing. |
| `the network's service "<id>" has no days` | A service has neither a weekly pattern nor exceptions. | Give it days, or leave it out. |
| `service "<id>" runs for N days, more than the 36600 allowed` | A service spans more than about 100 years. | Check its dates for a typo. |
| `<field> of the network's <table> is not a date up to the year 9999` | A date is past 9999-12-31, or is the `Date::NONE` sentinel. | Use `Date::parse_gtfs` or `Date::from_year_month_day`. |
| `<field> of the network's <table> is not a position on the globe` | A latitude outside ±90, a longitude outside ±180, or NaN. | Leave the position out (`None`) if you don't have one. |
| `the settings are not usable: a footpath reach of at most 50000 meters` | `footpath_max_meters` is above 50,000. | Use a shorter reach. |
| `search needs the text part, because a place is found by its name` | `Parts::SEARCH` without `Parts::TEXT`. | Add `TEXT`, or leave `SEARCH` out. On the command line `--without text` already takes search with it. |
| `the metadata is not valid json` / `the metadata json is not an object` | `metadata_json` got something that isn't a JSON object. | Pass `{"key": "value"}`. |

## Realtime does not match

You applied a feed and nothing changed. Check the report first:

```rust
use flatnet_core::rt::ApplyReport;

fn why(report: &ApplyReport) {
    println!("{report:#?}");
}
```

| You see | Likely cause | Do this |
|---|---|---|
| `trip_updates: 0`, `trip_updates_unmatched` large | The feed is for another network, or the file was compiled `--without ids`. | Check `consumer.parts().contains(Parts::IDS)`. Check the feed's `trip_id`s against `trip_by_id`. |
| `trip_updates_ambiguous` large | The feed names trips by route and start time, and several trips share a start. | Ask the publisher for `trip_id`s, or accept that those are skipped. |
| `wrong_service_date` large | The `Realtime` was made with `for_service_date` for another day. | Make one `Realtime` per service day, or use `Realtime::new`. |
| `trip_updates_stale` large | You applied an older feed after a newer one. | Apply feeds in time order. |
| `stop_time_updates_unmatched` large | Stop ids or sequences in the feed aren't in the trip's pattern. | The static feed and the realtime feed are from different publications. Recompile from the current static feed. |
| `unsupported` | Shapes, stops, trip modifications or replaced trips. | Nothing to do. They aren't applied. |
| Times don't move | You read the scheduled `Timing` instead of `realtime.departure_time(...)`. | See [Realtime](realtime.md#reading-a-delayed-time). |
| Times are off by an hour | The feed's timestamps are being read in the wrong zone. | Check `consumer.timezone()`, or set `with_time_zone` / `with_utc_offset_seconds`. |

## Queries that find nothing

| Call | Why it can be empty |
|---|---|
| `stop_by_id`, `trip_by_id`, ... | The id isn't in the file, or the file was compiled `--without ids`. |
| `search_by_name` | The file was compiled `--without text` or `--without search`, or the query is empty. |
| `stops_near` | The radius is zero or not finite, the point is far from any stop, or the stops have no coordinates. Stops with no position are never found. |
| `stops_in_box` | A bound is NaN, south is above north, or west is east of east. A box doesn't wrap over the 180th meridian: make two. |
| `shapes_by_trip` | The trip has no shape, or the file was compiled `--without shapes`. |
| `footpaths_by_place` | `--without footpaths`, the stop isn't a place (it has a parent), or no place is in reach. Raise `--footpath-meters`. |
| `is_service_active` | The date is outside the service's days, or the trip has no service. Check `service_range()`. |

## Panics

Accessors panic on an index that is out of range, `NONE` included. A file that passed `Consumer::new` never gives you such an index, so the index came from your code:

- You used a `StopIndex` as a `TripIndex` through `as_usize()`. The types are separate for this reason: avoid converting.
- You kept an index from one file and used it on another. Indices belong to one file.
- You read an `Optional` index without `.as_option()`. A `NONE` index panics when used.

`RealtimeBuilder::apply_feeds` panics if the `Realtime` was built from a different consumer. Build the `Realtime` from the same `Consumer` you apply with.

If you hit a panic with an index the file gave you, that is a bug: please report it with the file.

## Questions

**Can I modify a file?** No. Compile a new one. A file never changes, and live data goes in a `Realtime`.

**Can I merge two feeds?** Not into one file yet. Ids with a namespace are on the [roadmap](design.md#ideas-not-yet-decided).

**Is a file portable?** Between machines of the same byte order, yes. Little-endian, which is nearly all of them.

**Will an old file work after I update flatnet?** Only if the format version is the same. `Consumer::new` refuses any other version. Recompile from the feed.

**Does it use a lot of memory?** The map is paged in on demand, so a regional network costs what you touch. Compiling is the expensive step: about 340 MB peak for SL.

**Why is my file bigger than the feed?** Compile `--without` the parts you don't need. The [command line page](cli.md#leaving-parts-out) has sizes.

**How do I get a GTFS-realtime feed?** From the publisher, with their API key. flatnet decodes and applies feeds. It doesn't fetch them.
