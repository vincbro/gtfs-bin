# Recipes

Worked examples that combine the pieces. Each is a complete function you can paste into your project. They assume `consumer` is an open [`Consumer`](reading.md#opening).

- [A departure board](#a-departure-board)
- [A live departure board](#a-live-departure-board)
- [Every stop a route serves](#every-stop-a-route-serves)
- [Stops you can reach on foot](#stops-you-can-reach-on-foot)
- [A search box that is also a map](#a-search-box-that-is-also-a-map)
- [Per-stop data for a router](#per-stop-data-for-a-router)
- [Checking a file before you ship it](#checking-a-file-before-you-ship-it)
- [Refreshing realtime on a timer](#refreshing-realtime-on-a-timer)

## A departure board

The next departures from a stop on a given day, with the line and the headsign.

```rust
use flatnet_core::{
    consumer::Consumer,
    models::{BoardingRule, Date, StopIndex, Time, TripIndex},
};

struct Departure {
    trip: TripIndex,
    time: Time,
}

fn departures(
    consumer: &Consumer<'_>,
    stop: StopIndex,
    date: Date,
    after: Time,
    limit: usize,
) -> Vec<Departure> {
    let mut board = Vec::new();

    // A pattern that visits the stop twice in a row is listed twice.
    let mut patterns = consumer.trip_patterns_by_stop(stop).to_vec();
    patterns.dedup();

    for pattern in patterns {
        let stops = consumer.stop_sequence_by_trip_pattern(pattern);
        let rules = consumer.pattern_stops_by_trip_pattern(pattern);

        // Where in the pattern the stop is, which is also the column in every trip's times.
        for position in (0..stops.len()).filter(|&position| stops[position] == stop) {
            if rules[position].pickup == BoardingRule::NOT_AVAILABLE {
                continue;
            }
            for &trip in consumer.trips_in_trip_pattern(pattern) {
                let Some(time) = consumer.timings_by_trip(trip)[position].departure.as_option()
                else {
                    continue;
                };
                let runs = consumer.is_service_active(consumer.trip(trip).service_index, date);
                if time >= after && runs {
                    board.push(Departure { trip, time });
                }
            }
        }
    }

    board.sort_by_key(|departure| departure.time);
    board.truncate(limit);
    board
}

fn print_board(consumer: &Consumer<'_>, stop: StopIndex, date: Date) {
    for departure in departures(consumer, stop, date, Time(8 * 3600), 10) {
        let trip = consumer.trip(departure.trip);
        let line = consumer
            .route(trip.route_index)
            .short_name
            .as_option()
            .map_or("?", |name| consumer.string(name));
        let towards = consumer
            .trip_info(departure.trip)
            .headsign
            .as_option()
            .map_or("", |text| consumer.string(text));
        println!("{}  {line:>4}  {towards}", departure.time);
    }
}
```

Notes:

- Trips inside a pattern never overtake each other, so for a long board you can binary-search each pattern's trips for the first one at or after `after`, instead of testing all of them.
- A trip past midnight from yesterday is not on this board. See [trips that started yesterday](time-and-service-days.md#trips-that-started-yesterday).

## A live departure board

The same board with realtime applied. Canceled trips and skipped stops drop out, and the times move.

```rust
use flatnet_core::{
    consumer::Consumer,
    models::{Date, StopIndex, StopTimeIndex, Time, TripIndex},
    rt::Realtime,
};

fn live_departures(
    consumer: &Consumer<'_>,
    realtime: &Realtime,
    stop: StopIndex,
    date: Date,
    after: Time,
) -> Vec<(TripIndex, Time)> {
    let mut board = Vec::new();
    let mut patterns = consumer.trip_patterns_by_stop(stop).to_vec();
    patterns.dedup();

    for pattern in patterns {
        let stops = consumer.stop_sequence_by_trip_pattern(pattern);
        for position in (0..stops.len()).filter(|&position| stops[position] == stop) {
            for &trip in consumer.trips_in_trip_pattern(pattern) {
                if !consumer.is_service_active(consumer.trip(trip).service_index, date) {
                    continue;
                }
                let timing = consumer.timings_by_trip(trip)[position];
                let slot = StopTimeIndex::from(consumer.trip(trip).stop_times.start as usize + position);
                // `None` if the stop is skipped, the trip is canceled or it has no time here.
                if let Some(time) = realtime.departure_time(trip, slot, timing)
                    && time >= after
                {
                    board.push((trip, time));
                }
            }
        }
    }
    board.sort_by_key(|&(_, time)| time);
    board
}
```

A delayed trip can move out of order with the next one, so test every trip against the live time. The binary search shortcut works on scheduled times only.

Trips the feed added are not in the static tables. They are in `realtime.added_trips()`, and `realtime.added_trips_at_stop(stop)` gives those that call at a stop.

## Every stop a route serves

```rust
use flatnet_core::{consumer::Consumer, models::{RouteIndex, StopIndex}};

fn stops_of(consumer: &Consumer<'_>, route: RouteIndex) -> Vec<StopIndex> {
    let mut stops: Vec<StopIndex> = Vec::new();
    for &trip in consumer.trips_by_route(route) {
        let pattern = consumer.trip_pattern_by_trip(trip).index;
        stops.extend_from_slice(consumer.stop_sequence_by_trip_pattern(pattern));
    }
    stops.sort_unstable();
    stops.dedup();
    stops
}
```

This visits every trip. To visit each pattern once, collect the pattern indices first (`consumer.trip_to_trip_pattern()[trip.as_usize()]`), sort and dedup them, and read each pattern's stops once.

## Stops you can reach on foot

From a position, find the stops nearby, then widen to the places they belong to and the places those connect to:

```rust
use flatnet_core::{consumer::Consumer, models::StopIndex};

fn walkable(consumer: &Consumer<'_>, latitude: f64, longitude: f64) -> Vec<(StopIndex, f64)> {
    let mut near = Vec::new();
    consumer.stops_near(latitude, longitude, 300.0, &mut near);

    let mut reach: Vec<(StopIndex, f64)> = Vec::new();
    for &(stop, meters) in &near {
        // The stop itself, and the other stops of its place, which cost nothing extra.
        let place = consumer.place_of(stop);
        for &boardable in consumer.stops_by_place(place) {
            reach.push((boardable, f64::from(meters)));
        }
        // The other places in reach of this one.
        for footpath in consumer.footpaths_by_stop(stop) {
            for &boardable in consumer.stops_by_place(footpath.to_place_index) {
                reach.push((boardable, f64::from(meters) + f64::from(footpath.meters.0)));
            }
        }
    }

    // Keep the shortest walk to each stop.
    reach.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    reach.dedup_by_key(|entry| entry.0);
    reach.sort_by(|a, b| a.1.total_cmp(&b.1));
    reach
}
```

## A search box that is also a map

A search box finds places by name; a map shows the stops in view. Both fill a buffer you keep.

```rust
use flatnet_core::{
    consumer::Consumer,
    models::{Coordinate, SearchIndex, StopIndex},
};

struct SearchState {
    names: Vec<SearchIndex>,
    on_screen: Vec<StopIndex>,
}

impl SearchState {
    fn new() -> Self {
        Self { names: Vec::new(), on_screen: Vec::new() }
    }

    /// Called on every key press.
    fn typed(&mut self, consumer: &Consumer<'_>, text: &str) -> Vec<String> {
        consumer.search_by_name(text, &mut self.names);
        self.names
            .iter()
            .take(8)
            .map(|&index| consumer.string(consumer.search(index).name).to_owned())
            .collect()
    }

    /// Called when the map moves. Returns each stop's position.
    fn moved(&mut self, consumer: &Consumer<'_>, view: [f64; 4]) -> Vec<Coordinate> {
        let [south, west, north, east] = view;
        consumer.stops_in_box(south, west, north, east, &mut self.on_screen);
        self.on_screen
            .iter()
            .filter_map(|&stop| consumer.stop(stop).coordinate.as_option())
            .collect()
    }
}
```

## Per-stop data for a router

A router keeps arrays with one entry per stop, indexed by position. The indices make that a `Vec`, not a hash map:

```rust
use flatnet_core::{consumer::Consumer, models::{StopIndex, Time}};

struct Labels {
    arrival: Vec<Time>,
}

impl Labels {
    fn new(consumer: &Consumer<'_>) -> Self {
        Self { arrival: vec![Time(u32::MAX - 1); consumer.stops().len()] }
    }

    fn improve(&mut self, stop: StopIndex, time: Time) -> bool {
        let slot = &mut self.arrival[stop.as_usize()];
        if time < *slot {
            *slot = time;
            true
        } else {
            false
        }
    }
}
```

Where to put the work:

- Keep one `Vec` per kind of scratch space and `clear()` it between queries, as the search functions do.
- Prefer `trips_in_trip_pattern` and `timings_by_trip` in the inner loop. They return slices, and the data is contiguous: the times of a pattern's trips are next to each other in the file.
- Tell the OS you are jumping around with [`consumer::advise`](reading.md#telling-the-os-how-you-will-read).

## Checking a file before you ship it

```rust
use flatnet_core::{consumer::Consumer, models::Date};

fn check(consumer: &Consumer<'_>, today: Date) -> Result<(), String> {
    // 1. Not corrupted in transit.
    consumer.verify_checksum().map_err(|error| error.to_string())?;

    // 2. Not out of date.
    if consumer.is_stale(today) {
        return Err(format!("no service after {:?}", consumer.service_range()));
    }

    // 3. Compiled the way you expect.
    let settings = consumer.settings();
    if settings.footpath_max_meters < 400 {
        return Err("footpaths reach too short".to_owned());
    }
    Ok(())
}
```

## Refreshing realtime on a timer

```rust
use std::{sync::Arc, thread, time::Duration};

use flatnet_core::{
    consumer::Consumer,
    rt::{self, Realtime, RealtimeBuilder, SharedRealtime},
};

fn fetch() -> Vec<u8> {
    // Your HTTP client and API key go here.
    Vec::new()
}

fn run(consumer: &Consumer<'_>) {
    let shared = Arc::new(SharedRealtime::new(Realtime::new(consumer)));
    let builder = RealtimeBuilder::new(consumer);

    thread::scope(|scope| {
        // The updater: every 15 seconds, apply a new feed.
        scope.spawn(|| loop {
            if let Ok(message) = rt::decode_feed(&fetch()) {
                shared.update(|realtime| builder.apply([message].into_iter(), realtime));
            }
            thread::sleep(Duration::from_secs(15));
        });

        // Any number of readers.
        scope.spawn(|| {
            let realtime = shared.current();
            println!("{} trips tracked", realtime.trip_status().len());
        });
    });
}
```

`Consumer` and `RealtimeBuilder` are shared by reference between the threads of a scope.
