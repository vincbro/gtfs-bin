# Reading a file

The `Consumer` maps a compiled file and answers questions about it. This page covers opening a file and walking between the tables. Searching by position or name is in [Finding things](finding-things.md), and times in [Dates, times and service days](time-and-service-days.md).

```
cargo add flatnet-core memmap2
```

## Opening

```rust
use std::fs::File;

use flatnet_core::consumer::Consumer;
use memmap2::Mmap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("network.flnt")?;
    // SAFETY: the file is only read, and nothing modifies it while it is mapped.
    let mmap = unsafe { Mmap::map(&file)? };
    let consumer = Consumer::new(&mmap)?;

    println!("{} stops", consumer.stops().len());
    Ok(())
}
```

`Consumer<'a>` borrows the `Mmap` for `'a`, and everything it returns borrows the same map. Slices and records are `&'a`, so you can keep them as long as the map lives, and pass them around freely. A `Consumer` is `Send + Sync`: share it between threads by reference.

Mapping a file is `unsafe` in `memmap2` because another process could change the file under you. Don't modify a `.flnt` file while it is mapped (replace it by writing a new file and renaming it).

### The three ways to open

| Method | Checks | Use it for |
|---|---|---|
| `Consumer::new(&mmap)` | Header, sections, text, every index and slice, orders. About 10 ms for 2.5M stop times. | Any file you didn't just write yourself. This is the default. |
| `Consumer::new_unvalidated(&mmap)` | Header, length and that every section lies inside the file. | A file your own program wrote a moment ago. A damaged file can make an accessor panic, but never causes undefined behavior. |
| `consumer.verify_checksum()` | That every byte after the header is what the compiler wrote. Reads the whole file. | Detecting a changed byte, such as a bad download. `new` does not do it. |

```rust
use flatnet_core::consumer::Consumer;

fn check(consumer: &Consumer<'_>) -> Result<(), flatnet_core::consumer::Error> {
    // About as slow as copying the file: do it on download, not on every start.
    consumer.verify_checksum()
}
```

What `new` guarantees is on [Safety and validation](safety.md).

### Telling the OS how you will read

A router jumps around the tables. `advise` passes a hint to the operating system:

```rust
use flatnet_core::consumer::{self, Advice};
use memmap2::Mmap;

fn random_access(mmap: &Mmap) -> Result<(), consumer::Error> {
    consumer::advise(mmap, Advice::Random)
}
```

## The tables

Every table has an accessor named after it that returns a slice:

```rust
use flatnet_core::consumer::Consumer;

fn sizes(consumer: &Consumer<'_>) {
    println!("{} stops", consumer.stops().len());
    println!("{} routes", consumer.routes().len());
    println!("{} trips", consumer.trips().len());
    println!("{} stop times", consumer.timings().len());
    println!("{} trip patterns", consumer.trip_patterns().len());
    println!("{} services", consumer.services().len());
    println!("{} transfers", consumer.transfers().len());
}
```

Use these to scan. To follow a reference, use the by-index accessors below. They are the hot paths of a router: they return borrows and allocate nothing.

## Stops

```rust
use flatnet_core::consumer::Consumer;

fn stop_details(consumer: &Consumer<'_>) {
    let Some(stop) = consumer.stop_by_id("9021001002010000") else {
        return;
    };

    // The hot fields live in `Stop`: position, parent and kind.
    if let Some(position) = stop.coordinate.as_option() {
        let (latitude, longitude) = position.to_f64_pair();
        println!("at {latitude}, {longitude}");
    }
    println!("location type {:?}", stop.location_type);

    // The text lives in `StopInfo`, so scanning the stops never reads it.
    let info = consumer.stop_info(stop.index);
    if let Some(name) = info.name.as_option() {
        println!("name {}", consumer.string(name));
    }
    if let Some(code) = info.code.as_option() {
        println!("code {}", consumer.string(code));
    }

    // Stops under a station point at it.
    if let Some(parent) = stop.parent_index.as_option() {
        println!("under {}", consumer.stop_id(consumer.stop(parent).id));
    }
}
```

`consumer.stop_id(stop.id)` gives the original GTFS id back. A stop has a `location_type` of `STOP_POINT`, `STOP_AREA` (a station), `STATION_ENTRANCE`, `GENERIC_NODE` or `BOARDING_AREA`.

## Routes and trips

```rust
use flatnet_core::consumer::Consumer;

fn route_details(consumer: &Consumer<'_>) {
    let Some(route) = consumer.route_by_id("R1") else {
        return;
    };
    let short = route.short_name.as_option().map(|name| consumer.string(name));
    let long = route.long_name.as_option().map(|name| consumer.string(name));
    println!("{short:?} {long:?}, type {:?}", route.route_type);

    // The trips of a route, from the earliest start to the latest.
    for trip in consumer.iter_trips_by_route(route.index) {
        let info = consumer.trip_info(trip.index);
        let headsign = info.headsign.as_option().map(|text| consumer.string(text));
        println!("{} towards {headsign:?}", consumer.trip_id(trip.id));
    }
}
```

`trips_by_route(route)` is the same list as a `&[TripIndex]`, and `iter_trips_by_route` maps it to `&Trip`.

A `Trip` holds its `route_index`, `service_index`, `direction` and the slice of its times. Its text (headsign, short name, block) is in `TripInfo`.

### A trip's start

```rust
use flatnet_core::{consumer::Consumer, models::TripIndex};

fn start_of(consumer: &Consumer<'_>, trip: TripIndex) {
    // Its departure from the first stop, or its arrival there. `None` if it has no time there.
    match consumer.trip_start(trip) {
        Some(start) => println!("starts at {start}"),
        None => println!("no time at the first stop"),
    }
}
```

### Trips of a route that start at a time

The trips of a route are sorted by start, so finding the ones that start at a given time is a binary search. This is what matches a realtime update that names a route and a start time.

```rust
use flatnet_core::{
    consumer::Consumer,
    models::{RouteIndex, Time},
};

fn at_nine(consumer: &Consumer<'_>, route: RouteIndex) {
    // A run of trips, in index order. Several can share a start: different directions or days.
    let trips = consumer.trips_by_route_start(route, Time(9 * 3600));
    println!("{} trips leave at 09:00:00", trips.len());
}
```

## A trip, stop by stop

A trip's stops are its pattern's stops. Its times are one `Timing` per stop, in the same order. Zip them:

```rust
use flatnet_core::consumer::Consumer;

fn timetable(consumer: &Consumer<'_>, trip_id: &str) {
    let Some(trip) = consumer.trip_by_id(trip_id) else {
        return;
    };
    let pattern = consumer.trip_pattern_by_trip(trip.index);
    let stops = consumer.stop_sequence_by_trip_pattern(pattern.index);
    let timings = consumer.timings_by_trip(trip.index);

    for (&stop_index, timing) in stops.iter().zip(timings) {
        let stop = consumer.stop(stop_index);
        let name = consumer
            .stop_info(stop.index)
            .name
            .as_option()
            .map_or("(unnamed)", |slice| consumer.string(slice));
        match (timing.arrival.as_option(), timing.departure.as_option()) {
            (Some(arrival), Some(departure)) => println!("{name}: {arrival} - {departure}"),
            (Some(time), None) | (None, Some(time)) => println!("{name}: {time}"),
            (None, None) => println!("{name}: passes without a time"),
        }
    }
}
```

`Time` prints as `HH:MM:SS` and can pass 24 hours.

### What the pattern says about each stop

Boarding rules, headsigns and the distance traveled are the same for every trip of a pattern, so they are stored once, per position:

```rust
use flatnet_core::{consumer::Consumer, models::BoardingRule, models::TripPatternIndex};

fn boarding(consumer: &Consumer<'_>, pattern: TripPatternIndex) {
    let stops = consumer.stop_sequence_by_trip_pattern(pattern);
    let rules = consumer.pattern_stops_by_trip_pattern(pattern);

    for (&stop, rule) in stops.iter().zip(rules) {
        let can_board = rule.pickup != BoardingRule::NOT_AVAILABLE;
        let can_alight = rule.drop_off != BoardingRule::NOT_AVAILABLE;
        println!(
            "stop {stop:?}: sequence {}, board {can_board}, alight {can_alight}",
            rule.sequence
        );
    }
}
```

`PatternStop` also has `headsign` (an `Optional<StringSlice>`) and `distance_traveled` (an `Optional<Distance>`, meters along the trip).

### Walking a pattern

```rust
use flatnet_core::{consumer::Consumer, models::TripPatternIndex};

fn pattern(consumer: &Consumer<'_>, index: TripPatternIndex) {
    // The trips of the pattern, in the order they leave.
    for &trip in consumer.trips_in_trip_pattern(index) {
        let first = consumer.timings_by_trip(trip).first();
        println!("{:?} departs {:?}", trip, first.and_then(|t| t.departure.as_option()));
    }
}
```

Trips inside a pattern never overtake each other, so if trip `a` comes before trip `b`, then `a` is at or before `b` at every stop. A router can binary-search the trips of a pattern at a stop for the first one that leaves after a given time.

### Patterns and trips at a stop

```rust
use flatnet_core::{consumer::Consumer, models::StopIndex};

fn at_stop(consumer: &Consumer<'_>, stop: StopIndex) {
    // The patterns that visit the stop. A pattern that visits it twice in a row is listed twice.
    let patterns = consumer.trip_patterns_by_stop(stop);
    println!("{} patterns call here", patterns.len());

    // Every trip that calls here, once each.
    let trips = consumer.iter_trips_by_stop(stop).count();
    println!("{trips} trips call here");
}
```

An empty `trip_patterns_by_stop` tells you the stop has no service at all.

## Shapes

A shape is the path between two stops in a row. Each one is stored once, however many trips use it, and the compiler cuts the feed's shapes at the stops.

```rust
use flatnet_core::{consumer::Consumer, models::TripIndex};

fn path(consumer: &Consumer<'_>, trip: TripIndex) {
    // One shape for each pair of stops in a row, so one fewer than the trip has stops.
    // Empty if the trip has no shape, or the file was compiled without shapes.
    for &shape in consumer.shapes_by_trip(trip) {
        let length = consumer.shape(shape).length.0;
        let points = consumer.shape_coordinates(shape);
        println!("{} points, {length:.0} m", points.len());
    }
}
```

Join the pieces of a trip's shapes to draw its whole path.

## Transfers

```rust
use flatnet_core::{consumer::Consumer, models::StopIndex};

fn transfers(consumer: &Consumer<'_>, stop: StopIndex) {
    for transfer in consumer.outbound_transfers_by_stop(stop) {
        println!(
            "to {:?}, type {}, at least {:?} seconds",
            transfer.to_stop_index,
            transfer.transfer_type,
            transfer.min_transfer_time.as_option().map(|time| time.0),
        );
        // A transfer can apply only between two trips (an in-seat transfer).
        if let Some(trip) = transfer.from_trip_index.as_option() {
            println!("  only when arriving on trip {trip:?}");
        }
    }
}
```

`inbound_transfers_by_stop` gives the indices of the transfers that end at a stop, and `iter_inbound_transfers_by_stop` the records. `transfer_type` is the GTFS `transfer_type` value.

## Metadata

The file says where it came from:

```rust
use flatnet_core::consumer::Consumer;

fn about(consumer: &Consumer<'_>) {
    let metadata = consumer.metadata();
    println!("compiled at {} (0 means unknown)", metadata.compiled_at);

    let text = |slice: flatnet_core::models::Optional<flatnet_core::models::StringSlice>| {
        slice.as_option().map(|slice| consumer.string(slice))
    };
    println!("compiler {:?}", text(metadata.compiler_version));
    println!("source {:?}", text(metadata.source));
    println!("agency {:?}", text(metadata.agency_name));
    println!("publisher {:?}", text(metadata.publisher_name));
    println!("language {:?}", text(metadata.language));

    // Your own JSON object, from `--meta` or `Compiler::metadata_json`, as text.
    println!("{:?}", consumer.metadata_json());

    println!("time zone {:?}", consumer.timezone());
    println!("feed valid {:?} to {:?}", consumer.feed_start_date(), consumer.feed_end_date());
}
```

The fixed fields are `compiled_at`, `active_from`, `active_until`, `compiler_version`, `source`, `publisher_name`, `publisher_url`, `language`, `feed_version`, `contact_email`, `agency_name`, `agency_url` and the `settings`. `metadata_json` is text: parse it with the JSON library you already use.

### Is the file out of date?

```rust
use flatnet_core::{consumer::Consumer, models::Date};

fn stale(consumer: &Consumer<'_>, today: Date) {
    // Two integer comparisons, so ask on every launch.
    if consumer.is_stale(today) {
        eprintln!("no service runs after {:?}", consumer.service_range());
    }
}
```

`service_range()` is the first and last day any service runs. `is_stale` takes a day in the agency's time zone (`consumer.timezone()`). A file with no services is always stale. The next page says how to make a `Date`.

## Going from an id to text and back

| You have | You want | Call |
|---|---|---|
| A feed id (`&str`) | The record | `stop_by_id`, `route_by_id`, `trip_by_id`, `service_by_id` |
| A record | Its feed id | `stop_id(stop.id)`, `route_id(route.id)`, `trip_id(trip.id)`, `service_id(service.id)` |
| A `StringSlice` | Text | `consumer.string(slice)` |
| A position | The record | `stop(index)`, `route(index)`, `trip(index)`, `service(index)` |

All of these except the `*_by_id` ones panic on an index that is out of range, `NONE` included. A file that passed `Consumer::new` never hands you such an index, so a panic means your own index was made up. The `*_by_id` lookups return `None` instead, and `None` for everything if the file was compiled without ids.

## Cost, so you know what is cheap

Everything above that returns a slice or a reference is free: it indexes into the map. These read a lot and aren't for an inner loop:

- `search_by_name`, `search_by_code`, `search_by_route_name`: they read every name.
- `Consumer::new`: a pass over the whole file.
- `verify_checksum`: reads every byte.

Next: [Dates, times and service days](time-and-service-days.md).
