# Design

Why flatnet is built the way it is, what it leaves out on purpose, and where it is going. Read it if you are deciding whether flatnet fits, or if you are about to change it.

The status of each piece of work is in [TODO.md](../TODO.md). This page holds the reasons.

## The goal

> A tool that makes working with a transit network easy and fast, however large the network is.

It is **not** a goal to cover everything GTFS, GTFS-realtime, NeTEx or SIRI can say. Three rules follow:

1. **The format is ours.** A field is in the file because a consumer reads it, and out of it when none does, whatever the source calls it.
2. **No promise of a one-to-one mapping with GTFS.** A reader may drop what we don't use, derive what we do, and the format may hold values no source has.
3. **Every field earns its place.** What reads it, and what it costs in size and time. Speed is a goal, so a change that can be measured is measured on SL's regional feed and on the generated one (`just bench`).

The main consumer today is [blaise](https://github.com/vincbro/blaise), a transit router. flatnet is the data layer under it: it should give a router the questions it asks in the shape it asks them.

## One flat file

A transit network is read far more often than it is written, and most of what a router reads is numbers: which stop, which trip, what time. So the file is arrays of fixed-size records that refer to each other by position, laid out so that the data a loop needs is next to each other.

| Choice | Why |
|---|---|
| **Memory-mapped, no parsing** | Start-up is a header check and a validation pass. The OS pages in only what you touch. |
| **Fixed-size, pointer-free records** (`bytemuck::Pod`) | A table is a slice into the map. There is nothing to copy and nothing to point outside the file. |
| **Indices instead of ids** | A reference is an integer you can use to index a `Vec` of your own. The strings are kept for lookup and display. |
| **Hot and cold split** | A `Stop` is 28 bytes of what routing reads. Its name, code and description are in `StopInfo` at the same position, so scanning stops never reads text. The same goes for `Trip` and `TripInfo`. |
| **Optional values as sentinels** | `Optional<T>` is the same size as `T`. No tag byte, no padding. |
| **Validation on open** | A mapped file can't be trusted, so every index is checked once, and then every accessor is unchecked. See [Safety](safety.md). |
| **Same input, same bytes** | The compiler never reads the clock or the path, so a build can be compared byte for byte. |

## Trip patterns

The biggest table of a feed is `stop_times.txt`: every stop of every trip. On SL it was 100 MB of a 132 MB file. But trips of a route run the same stops all day. So the compiler groups trips into **patterns** that call at the same stops in the same order, with the same sequence numbers, boarding rules, headsigns and distances, and that never overtake each other. A pattern stores its stops once, and each trip stores only its times.

On SL, 124,000 trips share about 5,000 patterns. The file went from 132 MB to about 45 MB and opens in about 10 ms (it was 44 ms). In a prototype, scanning every departure took 209 µs over the flat times against 1,674 µs over per-trip stop times.

Patterns split in two cases, both on purpose: when the stop sequence or its per-stop rules differ, and when trips overtake each other. The second matters because inside a pattern a router can binary-search trips by time, which is only right if a trip that leaves earlier is never later further along.

## Places

GTFS says a station contains stops and walking between them is possible. Storing every stop-to-stop walk repeats the station: a neighboring station is listed once for each of its stops. flatnet groups stops into **places**, the top of a stop's parent chain, and stores walks between places only. Walking inside a place is a given. On SL, 99,390 stop-to-stop walks became 19,434 place-to-place walks.

A footpath is meters in a straight line, nothing more. No speed, no detour, no time. A time depends on who is walking, and a file shouldn't decide that. The reach and the per-place limit are settings kept in the file, so a reader knows what the tables mean.

## Settings live in the file

Anything the compile options change in the tables is written into the file (`Metadata::settings`), and the consumer rejects a file whose settings the compiler would not have accepted. A reader never has to guess how far footpaths reach or what was left out. `Parts` in the header does the same for the optional sections.

## Data, not helpers

The file stores data. It does not store policy. There is no `walk_duration`, no transfer penalty, and no preferred walking speed. A function belongs in the library only when it applies data the file stores: `is_service_active`, `trips_by_route_start`, `place_of`. How to weigh a transfer is the router's decision.

## The seam: any source, one format

```
 GTFS reader  ─┐
 NeTEx reader  ├─►  Network  ─►  Compiler  ─►  .flnt  ─►  Consumer
 your reader  ─┘   (in memory)
```

The compiler takes a source-neutral `Network` and builds everything derived from it: patterns and their timings, shapes cut at stops, the grid, search, the id lookups and the metadata. A **reader** turns a source into a `Network`. GTFS is the first, behind the `gtfs` cargo feature of `flatnet-compiler`. A NeTEx reader would be a `netex` feature beside it.

The realtime side has the same seam. `RealtimeBuilder` applies a source-neutral `Feed` and knows no wire format. GTFS-realtime is one decoder behind the `gtfs-rt` feature; SIRI would be another.

This is the reason `Network` exists. It is the one thing to design carefully: it isn't the binary format and isn't `Pod`.

## What each format has, and what flatnet stores

A quick map, from the four formats flatnet can be fed or compared with.

| Format | What it is | In flatnet |
|---|---|---|
| **GTFS** | Static timetable, flat CSV tables | Read by `gtfs::Reader`. The shared core is stored: stops with parents, routes, trips, stop times, calendars, transfers, shapes, time zone, feed validity. |
| **GTFS-realtime** | Live overlay keyed by GTFS ids | Decoded by `rt::to_feed`. Trip updates, vehicle positions and alerts are applied. |
| **NeTEx** | The full network in nested XML | Not read. A Nordic-profile reader is planned. |
| **SIRI** | Live data, the counterpart of NeTEx | Not read. Estimated timetable is planned first. |

What stays out until a consumer asks for it: fares, vehicle scheduling and duties, facility monitoring, station navigation graphs, a line above the route, an operator apart from the authority, a stop place apart from the stop point, validity per object, and accessibility values.

### What the shared core is

The overlap of all four formats is nearly all of GTFS: stops and stations, routes, trips, stop times, service calendars with exceptions, transfers, an agency, shapes, alerts, vehicle positions, delays, canceled and added trips. That is what the file stores. The part that only NeTEx and SIRI have is the part the flat file leaves out.

## Ideas not yet decided

These come from comparing GTFS with NeTEx. Each is cheap in a `Network` and expensive in a file already shipped, so they are listed here to be decided once, together, before the format is frozen. None is built.

| Idea | Why it might matter |
|---|---|
| Ids with a namespace (`source:id`) | Two feeds merge into one file for a national network. |
| A stop place and a stop point, linked | A platform change becomes an assignment, not a new stop. |
| Line, route, pattern, trip | `trip_patterns` is derived from the trips today, and not a source concept. |
| A run key: trip plus service day as one id | Ends ambiguous matching of realtime updates that name a route and a start time. |
| Aimed, expected and actual time per call | One delay number can't carry SIRI's estimated timetable. |
| Frequencies as a headway with a pattern | Open: expand a headway into trips, or store the headway. SL has no `frequencies.txt`. |
| `timepoint` | SL marks every stop time. Interpolation treats every time as exact today. |

## Decisions made against

- **A wider schema for its own sake.** A schema that fits every format is a design for an unknown set. Its usual failure is the lowest common denominator or the union, which becomes NeTEx.
- **Full NeTEx or SIRI coverage.** NeTEx is large, and the same fact can be written several ways. A profile is the only sane target.
- **`cargo fuzz`.** The property-test suites found every bug so far. Revisit if one turns up that they missed.
- **Detour and speed in the file.** See Places above.

## Open questions

The field audit is next: go through every field of every table and write down what reads it and what it costs, then drop what nothing reads and add what a consumer needs. `blaise` comes first. See [TODO.md](../TODO.md).
