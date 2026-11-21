# Finding things

Four ways to find a stop or a route: by position, in a box, by name, and by walking. Each one fills a buffer you pass in, so a loop that asks again and again allocates nothing after the first call.

Examples assume `consumer` is an open [`Consumer`](reading.md#opening).

## Stops near a point

```rust
use flatnet_core::consumer::Consumer;

fn near(consumer: &Consumer<'_>) {
    let mut found = Vec::new();
    consumer.stops_near(59.3303, 18.0586, 250.0, &mut found);

    // Nearest first, with the distance in meters. Equal distances go by stop index.
    for &(stop, meters) in &found {
        let name = consumer
            .stop_info(stop)
            .name
            .as_option()
            .map_or("(unnamed)", |slice| consumer.string(slice));
        println!("{meters:5.0} m  {name}");
    }
}
```

`stops_near(latitude, longitude, radius_meters, &mut found)` replaces what was in `found`. The distance is the great-circle distance on a sphere of radius 6,371,008.8 m, the same as `Coordinate::distance_meters`.

- The result has every stop within the radius, and no other. A property test compares it with a scan of all stops, anywhere on the globe and for radii up to f64's maximum.
- Stops with no position are never found.
- A latitude, longitude or radius that isn't a finite number finds nothing. A negative radius counts as zero.
- The circle wraps over the 180th meridian and covers every longitude at a pole.

Keep one `Vec` and reuse it:

```rust
use flatnet_core::{consumer::Consumer, models::StopIndex};

fn many_queries(consumer: &Consumer<'_>, points: &[(f64, f64)]) -> usize {
    let mut found: Vec<(StopIndex, f32)> = Vec::new();
    let mut total = 0;
    for &(latitude, longitude) in points {
        consumer.stops_near(latitude, longitude, 400.0, &mut found);
        total += found.len();
    }
    total
}
```

### How it works

Stops are binned into a grid of 0.005 degree cells (about 550 m of latitude) that the compiler writes sorted. A query looks only at the cells the circle touches, with a binary search for each row, then measures the stops in them. A query costs the number of stops in nearby cells, not the number of stops in the network.

## Stops in a box

```rust
use flatnet_core::consumer::Consumer;

fn in_box(consumer: &Consumer<'_>) {
    let mut found = Vec::new();
    // south, west, north, east
    consumer.stops_in_box(59.32, 18.04, 59.34, 18.08, &mut found);
    println!("{} stops on screen", found.len());
}
```

This is for a map: the stops in the view. The edges are inside the box, and the result is in stop order.

- A bound that isn't a finite number, a south edge north of the north edge, or a west edge east of the east edge finds nothing.
- A box does not wrap. To search across the 180th meridian, make two boxes.

## Searching by name

```rust
use flatnet_core::consumer::Consumer;

fn by_name(consumer: &Consumer<'_>) {
    let mut found = Vec::new();
    // Case and accents are ignored: "sodermalmstorg" finds "Södermalmstorg".
    consumer.search_by_name("centralen", &mut found);

    for &index in &found {
        let place = consumer.search(index);
        println!(
            "{} ({} stops)",
            consumer.string(place.name),
            consumer.stops_by_search(index).len()
        );
    }
}
```

`search_by_name` finds **named places**. A place is a name, and all the stops that share it, so "T-Centralen" is one result with several stops. The result is a list of `SearchIndex`; `stops_by_search(index)` gives the stops, and `iter_stops_by_search(index)` the records.

Results come best first, then by index:

1. a name equal to the query,
2. a name that starts with it,
3. a name with a word that starts with it,
4. a name that contains it.

An empty query, or a file without the `SEARCH` part, finds nothing. Folding covers the Latin letters with accents that European place names use (`å ä ö é ü ñ ł ß æ œ` and more), and combining marks. Other scripts are only lowercased.

### By stop code

```rust
use flatnet_core::consumer::Consumer;

fn by_code(consumer: &Consumer<'_>) {
    let mut found = Vec::new();
    consumer.search_by_code("12", &mut found);

    // The code "12" first, then codes that start with it ("123"), each in stop order.
    for &stop in &found {
        println!("{}", consumer.stop_id(consumer.stop(stop).id));
    }
}
```

### By route name

```rust
use flatnet_core::consumer::Consumer;

fn by_route(consumer: &Consumer<'_>) {
    let mut found = Vec::new();
    // Matches the short name or the long name, ranked by the better of the two.
    consumer.search_by_route_name("4", &mut found);
    for &route in &found {
        println!("{}", consumer.route_id(consumer.route(route).id));
    }
}
```

These three read every name, so they suit a search box and not a loop. Typo-tolerant search is not there yet.

## Places and footpaths

A passenger can walk between nearby stops. flatnet stores those walks, with one idea to keep them small: a **place**.

- A place is the top of a stop's parent chain. A station with three platforms under it is **one place** with three stops. A stop with no parent is a place of one stop.
- Walking between the stops of one place is a given, so it has no footpath.
- A **footpath** is a walk from one place to another: the straight-line distance in meters between the nearest stops of the two places.

On SL's feed, this took 99,390 stop-to-stop walks down to 19,434 place-to-place walks.

```rust
use flatnet_core::{consumer::Consumer, models::StopIndex};

fn walks(consumer: &Consumer<'_>, stop: StopIndex) {
    // The place the stop belongs to, named by its root stop.
    let place = consumer.place_of(stop);

    // The stops a passenger can board at in a place.
    let here = consumer.stops_by_place(place);
    println!("{} stops in this place", here.len());

    // The walks from the stop's place, nearest first.
    for footpath in consumer.footpaths_by_stop(stop) {
        let there = consumer.stops_by_place(footpath.to_place_index);
        println!(
            "{:.0} m to place {:?} with {} stops",
            footpath.meters.0,
            footpath.to_place_index,
            there.len()
        );
    }
}
```

| Call | Gives |
|---|---|
| `place_of(stop)` | The place a stop belongs to, as a `StopIndex`. |
| `stops_by_place(place)` | The stops you can board at, in index order. Empty for a stop that isn't a place. |
| `footpaths_by_place(place)` | The walks that start at a place, nearest first. |
| `footpaths_by_stop(stop)` | `footpaths_by_place(place_of(stop))`. |

Stations, entrances and generic nodes are not among a place's stops, because nobody boards there.

### The file gives distances, you give the speed

A footpath is meters in a straight line. It has no walking speed and no detour, so one file serves a runner and a wheelchair user. Turn it into a time in your router:

```rust
use flatnet_core::{consumer::Consumer, models::{Duration, StopIndex}};

fn walking_seconds(consumer: &Consumer<'_>, stop: StopIndex) -> Vec<(StopIndex, Duration)> {
    let speed_meters_per_second = 1.2;
    // Nobody walks in a straight line: a factor of 1.3 is a common choice.
    let detour = 1.3;

    consumer
        .footpaths_by_stop(stop)
        .iter()
        .map(|footpath| {
            let seconds = f64::from(footpath.meters.0) * detour / speed_meters_per_second;
            (footpath.to_place_index, Duration(seconds.ceil() as u32))
        })
        .collect()
}
```

For two positions the file has no footpath for, use `Coordinate::distance_meters`:

```rust
use flatnet_core::models::Coordinate;

fn straight_line() -> f64 {
    let from = Coordinate::new(59.3303, 18.0586);
    let to = Coordinate::new(59.3326, 18.0649);
    from.distance_meters(to)
}
```

### What the settings mean

How far footpaths reach and how many each place keeps are chosen when you compile (`--footpath-meters`, `--footpath-max`) and stored in the file:

```rust
use flatnet_core::consumer::Consumer;

fn limits(consumer: &Consumer<'_>) {
    let settings = consumer.settings();
    println!("{} m, {} per place", settings.footpath_max_meters, settings.footpath_max_per_place);
}
```

If you need a walk longer than the file's reach, it isn't there: compile again with a bigger reach, or measure with `Coordinate::distance_meters`. A reach is capped at 50,000 m.

Footpaths are not always symmetric. A place keeps only its nearest `footpath_max_per_place`, so place A can list place B while B, which has many nearer neighbors, doesn't list A.

### Without the part

A file compiled without `FOOTPATHS` has no places: `stops_by_place` and `footpaths_by_place` are empty for every stop. `place_of` still works, because it follows the parent chain in the stops table.

Next: [Realtime](realtime.md).
