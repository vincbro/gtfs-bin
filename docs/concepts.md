# Concepts

Six ideas explain everything else. Read this page once and the rest of the docs will make sense.

## Compile once, read in place

flatnet works in two phases.

```
 GTFS feed ─► Reader ─► Network ─► Compiler ─► network.flnt ─► Consumer ─► your code
               (parse)   (in memory)  (build)     (one file)    (map, read)
```

- **Compile.** The `Compiler` takes a `Network`, which is a transit network in memory with no wire format in it, and builds everything derived from it: trip patterns, the times of every trip, shapes cut at stops, a grid of stops, a name index, id lookups and the days each service runs. It writes one file. This is slow (a few seconds for a regional network) and you do it once.
- **Consume.** The `Consumer` maps the file into memory and reads it where it lies. There is no parsing and no copy. The operating system pages data in when you touch it. Opening a file is a header check and a validation pass, about 10 ms for 2.5 million stop times.

The same network always compiles to the same bytes. The compiler never reads the clock or the path, so a feed gives one file wherever and whenever you compile it.

## Tables, not objects

The file is a set of flat arrays of fixed-size records, called **tables** or **sections**. A `Stop` is 28 bytes, a `Trip` is 40, and a `Timing` is 8. There are no pointers. A record refers to another by its position in the other table.

That is why `consumer.stops()` is a plain `&[Stop]`: it is a slice into the map. Scanning every stop is a linear read of memory.

## Indices, not ids

A feed names things with strings (`"9021001002010000"`). The compiler turns every one into a position:

- `StopIndex`, `RouteIndex`, `TripIndex`, `ServiceIndex` and so on are small integers. Each type is its own type, so you can't pass a `RouteIndex` where a `StopIndex` is wanted.
- A record's `index` field is its own position. `consumer.stop(stop.index)` is `&consumer.stops()[stop.index.as_usize()]`.
- The original string id is kept (unless you compile `--without ids`), and `stop_by_id`, `route_by_id`, `trip_by_id` and `service_by_id` find a record by it with a binary search.

Make an index from a position with `StopIndex::from(7_usize)`, and get the position back with `.as_usize()`.

```rust
use flatnet_core::{consumer::Consumer, models::StopIndex};

fn first_stop(consumer: &Consumer<'_>) {
    let index = StopIndex::from(0_usize);
    let stop = consumer.stop(index);
    assert_eq!(stop.index, index, "a stop knows its own position");
    assert_eq!(consumer.stops()[index.as_usize()].index, index);
}
```

Use indices in your own data structures. A `Vec<f32>` with one entry per stop, indexed by `stop.index.as_usize()`, is how a router keeps its labels.

## Absent values

A field that may be missing is an `Optional<T>`. It is the same size as `T`, with one value of `T` reserved to mean "none" (`Sentinel::NONE`). Call `.as_option()` to get an `Option<T>`.

```rust
use flatnet_core::consumer::Consumer;

fn name_of<'a>(consumer: &Consumer<'a>, stop: flatnet_core::models::StopIndex) -> Option<&'a str> {
    // `name` is an `Optional<StringSlice>`: a stop can be without one.
    let slice = consumer.stop_info(stop).name.as_option()?;
    Some(consumer.string(slice))
}
```

The same goes for `Time` (a stop with no scheduled time), `Date` and `Coordinate`. A `StopIndex::NONE` is never a valid position, and passing it to an accessor panics like any other index out of range.

## Slices

A record that owns a run of another table holds a **slice**: a `start` and a `count`. A trip's `stop_times` is a slice of `timings`; a pattern's `trips` is a slice of `trip_patterns_trips`. You rarely touch them. The accessors do it for you:

```rust
use flatnet_core::{consumer::Consumer, models::TripIndex};

fn times_of(consumer: &Consumer<'_>, trip: TripIndex) {
    // The run of `timings` that `consumer.trip(trip).stop_times` names, without the slicing.
    for timing in consumer.timings_by_trip(trip) {
        println!("{:?} -> {:?}", timing.arrival.as_option(), timing.departure.as_option());
    }
}
```

`(0, 0)` is both "none" and "empty". The two read the same.

## Patterns: where the times live

Trips of a route usually call at the same stops in the same order, all day. flatnet groups them into **trip patterns** and stores the stops once per pattern, then one row of times per trip. On SL's regional network, 124,000 trips share about 5,000 patterns, and storing the stops once per pattern took the file from 132 MB to about 55 MB.

A pattern holds trips that:

- call at the same stops in the same order,
- have the same sequence numbers, boarding rules, headsigns and distances at each stop,
- never overtake each other.

The times of a trip are one `Timing` (an arrival and a departure) for each stop of its pattern, in the pattern's order. So position `n` in `stop_sequence_by_trip_pattern(pattern)` and position `n` in `timings_by_trip(trip)` are the same call. See [Reading a file](reading.md#a-trip-stop-by-stop).

## Parts

Some of the file is optional. You can leave a part out to make the file smaller, and a reader can ask which are present:

| Part | What it holds | Without it |
|---|---|---|
| `SHAPES` | The path between each pair of stops | `shapes_by_trip` is empty |
| `TEXT` | Names, codes, descriptions, headsigns, blocks | Text fields are `None` |
| `SEARCH` | Finding a place by name (needs `TEXT`) | `search_by_name` finds nothing |
| `IDS` | The feed's ids and the lookups by id | `*_by_id` find nothing, so realtime matches no trip |
| `FOOTPATHS` | Places and the walks between them | No footpaths, every place is empty |

```rust
use flatnet_core::{consumer::Consumer, models::Parts};

fn can_search(consumer: &Consumer<'_>) -> bool {
    consumer.parts().contains(Parts::SEARCH)
}
```

How to choose parts: [Compiling](compiling.md#parts) and [the command line](cli.md#leaving-parts-out).

## Settings live in the file

Choices that shape the tables, such as how far a footpath reaches, are stored in the file, so a reader knows what the tables mean and doesn't have to guess:

```rust
use flatnet_core::consumer::Consumer;

fn reach(consumer: &Consumer<'_>) {
    let settings = consumer.settings();
    println!(
        "footpaths reach {} m, {} per place",
        settings.footpath_max_meters, settings.footpath_max_per_place
    );
}
```

## Units

| Thing | Unit |
|---|---|
| `Time` | Seconds since the start of the service day. It can pass 86,400 for a trip after midnight. |
| `Date` | Days since 1970-01-01. |
| `Coordinate` | Degrees times 10,000,000, as two `i32`. Seven decimals, about 1 cm. |
| `Distance` | Meters, as an `f32`. |
| `Delay` | Seconds, as an `i16`, so about ±9 hours. |

Read a coordinate with `coordinate.to_f64_pair()`, which gives `(latitude, longitude)`.

## What a file never does

- It never needs a parser or a schema at runtime.
- It never contains a pointer, so it can't point outside itself, and `Consumer::new` checks that no index does either.
- It never changes. Live data is a separate `Realtime` value that sits next to the file: see [Realtime](realtime.md).

Next: [the command line](cli.md), or straight to [Reading a file](reading.md).
