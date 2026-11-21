# Realtime

A compiled file is a timetable and never changes. Live data sits next to it: a `Realtime` holds the delays, cancellations, skipped stops, vehicle positions and alerts that feeds report right now, and a `RealtimeBuilder` applies feeds to it.

```
 compiled file  ─────────────►  Consumer  ◄─── reads ───┐
                                                        │
 feed bytes ─► decode ─► Feed ─► RealtimeBuilder ─► Realtime ─► your router
```

Reading a live time is an array index, so a router can use delays in its inner loop.

## What you need

- A file compiled **with ids** (the default). Realtime matches trips and stops by the ids the source uses, and a file compiled `--without ids` matches nothing.
- The `realtime` feature of `flatnet-core` for the builder, and `gtfs-rt` to decode GTFS-realtime. Both are on by default.

flatnet **applies** feeds. Fetching them, with the URL and the API key, is your code: use any HTTP client.

## The shortest path: GTFS-realtime

```rust
use flatnet_core::{
    consumer::Consumer,
    rt::{self, Realtime, RealtimeBuilder},
};

fn apply(consumer: &Consumer<'_>, feed_bytes: &[u8]) -> Result<Realtime, rt::Error> {
    // The bytes a publisher serves, already decompressed.
    let message = rt::decode_feed(feed_bytes)?;

    let builder = RealtimeBuilder::new(consumer);
    let (realtime, report) = builder.build_with_report([message].into_iter());

    println!("{report:?}");
    Ok(realtime)
}
```

`build` makes a fresh `Realtime` from the messages. `apply` adds messages to one you already have. `decode_feed` means you need no protobuf crate of your own.

### Reading a delayed time

```rust
use flatnet_core::{
    consumer::Consumer,
    models::{StopTimeIndex, Time, TripIndex},
    rt::Realtime,
};

fn live_departures(consumer: &Consumer<'_>, realtime: &Realtime, trip: TripIndex) -> Vec<Option<Time>> {
    let first = consumer.trip(trip).stop_times.start as usize;
    consumer
        .timings_by_trip(trip)
        .iter()
        .enumerate()
        .map(|(position, timing)| {
            // The slot of this stop time in the file's `timings` table.
            let slot = StopTimeIndex::from(first + position);
            // The scheduled time plus the delay, or `None` if skipped or canceled.
            realtime.departure_time(trip, slot, *timing)
        })
        .collect()
}
```

`arrival_time` and `departure_time` take the trip, the slot and the scheduled `Timing` you already read from the consumer. They return `None` when the stop has no scheduled time there, the stop is skipped, or the whole trip is canceled.

Other getters, each a slice indexed by trip or stop time:

| Call | Gives |
|---|---|
| `is_trip_canceled(trip)` | Canceled or deleted. |
| `is_stop_time_skipped(trip, slot)` | The vehicle skips that stop, or the trip is canceled. |
| `trip_status()` / `trip_delay()` | One entry per trip. `TripStatus` is `Unchanged`, `Scheduled`, `Added`, `Canceled`, `Deleted` or `Unscheduled`. |
| `stop_time_status()` / `stop_time_arrival_delays()` / `stop_time_departure_delays()` / `stop_time_occupancy()` | One entry per stop time. |
| `trip_vehicles()` | One `Option<Vehicle>` per trip: position, bearing, speed, occupancy, current stop. |
| `added_trips()` | Trips the feed added that the static data doesn't have. |
| `alerts()` | Every alert. |

A delay is `Optional<Delay>`. `Delay` is seconds as an `i16`, so about plus or minus nine hours. A difference outside that range becomes "no delay".

## One state per service day

```rust
use flatnet_core::{consumer::Consumer, models::Date, rt::Realtime};

fn states(consumer: &Consumer<'_>) {
    // Accepts updates for any day.
    let any_day = Realtime::new(consumer);

    // Accepts updates for 2026-03-15 only. Updates that name another day are ignored.
    let today = Realtime::for_service_date(consumer, Date::parse_gtfs("20260315").expect("a date"));
    assert_eq!(today.service_date(), Date::parse_gtfs("20260315"));
    drop(any_day);
}
```

A trip that runs past midnight belongs to the day it started on. Keep one `Realtime` per service day, and a late-night trip can't be mixed up with the first trips of the next day. See [Dates, times and service days](time-and-service-days.md#trips-that-started-yesterday).

## Options

```rust
use flatnet_core::{consumer::Consumer, rt::RealtimeBuilder};

fn builder<'a>(consumer: &'a Consumer<'a>) -> Result<RealtimeBuilder<'a>, flatnet_core::rt::Error> {
    RealtimeBuilder::new(consumer)
        // A stop time without an update takes the delay of the stop before it. On by default.
        .with_cascading_delays(true)
        // A feed that says it is a full dataset replaces what an earlier one set. On by default.
        .with_replace_on_full_dataset(true)
        // Alert texts in this language, else the first translation.
        .with_preferred_language("sv")
        // The zone feed timestamps are read in. By default, the zone stored in the file.
        .with_time_zone("Europe/Stockholm")
}
```

Time zones matter because stop times are local times of day and feeds give absolute timestamps. The builder uses the agency zone stored in the file, which follows daylight saving time, so a delay computed from a timestamp stays exact on the days the clocks change. `with_utc_offset_seconds(3600)` sets a fixed offset instead, for a file with no zone.

`with_replace_on_full_dataset(false)` is for several feeds that each hold part of the same kind of entity, such as one feed of trip updates per region.

## What is applied

- **Trip updates**, matched by `trip_id`, or by route, start time, date and direction when there is no id. A start time that fits two trips is reported as ambiguous and applied to neither.
- **Stop time updates**, matched by `stop_sequence` or by `stop_id`. Delays come from `delay`, or from `time` against the schedule. `Skipped` and `NoData` follow the GTFS-realtime spec. A stop a trip visits twice is told apart by order.
- **Trip status**: canceled, deleted, added and unscheduled, and a trip-level delay, which is spread over the stops when the update has no stop times.
- **Added trips** that aren't in the static data, kept in `added_trips()`.
- **Vehicle positions** with the current stop, bearing, speed and occupancy.
- **Alerts**, resolved to routes, stops and trips, and `active_alerts(time)`.
- **Stale and deleted data**: an older timestamp is ignored, `is_deleted` removes an entity, and a full dataset replaces the kinds of entity it holds.

Each trip update **replaces** what that trip had before. It does not add to it.

What is not applied: shapes and stops sent in a feed, trip modifications, and replaced or duplicated trips. They are counted as `unsupported`. See [TODO.md](../TODO.md).

## The report

An entity that is not applied is not an error, because feeds routinely mention trips the static data doesn't have. `ApplyReport` counts what happened, so you can tell when a feed isn't matching:

```rust
use flatnet_core::rt::ApplyReport;

fn health(report: &ApplyReport) {
    let seen = report.trip_updates + report.trip_updates_unmatched;
    if seen > 0 && report.trip_updates * 2 < seen {
        eprintln!("most trip updates match nothing: wrong feed for this file?");
    }
    if report.wrong_service_date > 0 {
        eprintln!("{} updates were for another service day", report.wrong_service_date);
    }
}
```

| Field | Counts |
|---|---|
| `trip_updates` | Applied to a trip. |
| `trip_updates_unmatched` | For a trip the file doesn't have. |
| `trip_updates_ambiguous` | Matched more than one trip, so none was chosen. |
| `trip_updates_stale` | Older than an update already applied. |
| `wrong_service_date` | For a day other than the `Realtime`'s. |
| `stop_time_updates` / `stop_time_updates_unmatched` | Applied / matched no stop time of their trip. |
| `vehicles` / `vehicles_unmatched` | Positions applied / with no trip found. |
| `added_trips`, `alerts`, `deleted` | Added trips, alerts stored, entities removed. |
| `unsupported` | Entities of a kind that isn't applied. |

On SL's feed, 456 of 457 trip updates match.

## Sharing between threads

A `Realtime` is plain data and is `Send + Sync`. To update it while many threads read it, put it in a `SharedRealtime`:

```rust
use std::sync::Arc;

use flatnet_core::{
    consumer::Consumer,
    rt::{self, Realtime, RealtimeBuilder, SharedRealtime},
};

fn refresh(consumer: &Consumer<'_>, shared: &SharedRealtime, bytes: &[u8]) -> Result<(), rt::Error> {
    let message = rt::decode_feed(bytes)?;
    let builder = RealtimeBuilder::new(consumer);

    // Works on a copy and swaps it in whole: a reader never sees a feed half applied.
    let report = shared.update(|realtime| builder.apply([message].into_iter(), realtime));
    println!("{report:?}");
    Ok(())
}

fn read(shared: &SharedRealtime) {
    // Keep reading this `Arc` for as long as you like: it does not change under you.
    let realtime: Arc<Realtime> = shared.current();
    println!("{} trips tracked", realtime.trip_status().len());
}
```

Updates run one at a time. Readers take `current()` and go on using the old state until the swap.

## Applying feeds from another source

The builder knows nothing of GTFS-realtime. It applies a `Feed`, a list of source-neutral `Update`s that name trips and stops by the source's ids. GTFS-realtime is one decoder (`rt::to_feed`). For another source, such as a vendor API or SIRI, make `Feed` values yourself:

```rust
use flatnet_core::{
    consumer::Consumer,
    models::Delay,
    rt::{
        CallUpdate, Event, Feed, Realtime, RealtimeBuilder, StopTimeStatus, TripRef, TripStatus,
        TripUpdate, Update,
    },
};

fn late_by_two_minutes(consumer: &Consumer<'_>) -> Option<Delay> {
    let late = Event {
        delay: Some(120), // seconds later than scheduled
        time: None,       // or a POSIX time, to be compared with the schedule
    };
    let feed = Feed {
        updates: vec![Update::Trip(TripUpdate {
            trip: TripRef {
                trip_id: Some("trip4".to_owned()),
                ..TripRef::default()
            },
            status: TripStatus::Scheduled,
            unsupported: false,
            delay: None,
            timestamp: None,
            calls: vec![CallUpdate {
                sequence: None,
                stop_id: Some("stop9".to_owned()),
                arrival: Some(late),
                departure: Some(late),
                status: StopTimeStatus::Scheduled,
                occupancy: None,
            }],
        })],
        ..Feed::default()
    };

    let mut realtime = Realtime::new(consumer);
    let report = RealtimeBuilder::new(consumer).apply_feeds([feed], &mut realtime);
    assert_eq!(report.trip_updates, 1);

    let trip = consumer.trip_by_id("trip4")?;
    let slot = trip.stop_times.start as usize + 1;
    realtime.stop_time_arrival_delays()[slot].as_option()
}
```

| Type | What it is |
|---|---|
| `Feed` | One message: a `timestamp`, `replaces` (the kinds it replaces in full), and `updates`. |
| `Update::Trip(TripUpdate)` | A trip, its status, and a `CallUpdate` per stop. |
| `Update::Vehicle(VehicleUpdate)` | A vehicle's position and state. |
| `Update::Alert(AlertUpdate)` | An alert and what it informs. |
| `Update::Remove(Removal)` | Take back an alert, a trip update or a vehicle. |
| `Update::Unsupported` | Something the source sent that this crate can't apply. It is counted. |

An `Event` has a `delay` or a `time`. When both are set, `delay` wins. Feeds with dates can name them in `TripRef::start_date`; a date past year 9999 is ignored and doesn't fail.

## Alerts

```rust
use flatnet_core::{consumer::Consumer, models::StopIndex, rt::Realtime};

fn alerts(consumer: &Consumer<'_>, realtime: &Realtime, stop: StopIndex, now: u64) {
    // `now` is a POSIX time. An alert with no active period is always active.
    for alert in realtime.alerts_for_stop(stop, now) {
        println!("{}: {}", alert.header, alert.description);
    }

    // Or by route or trip.
    for route in consumer.routes() {
        let count = realtime.alerts_for_route(route.index, now).count();
        if count > 0 {
            println!("route {:?} has {count} alerts", route.index);
        }
    }
}
```

An alert whose entities are all unknown to this network is dropped, because it is about something else. An id the file doesn't have makes the whole entity useless, because dropping just that id would widen what the alert applies to.

## Clearing

`realtime.clear()` forgets everything. `clear_trip_updates()`, `clear_vehicles()` and `clear_alerts()` forget one kind. A feed that says it is a full dataset does this for the kinds it holds, so you rarely call them yourself.

Next: [Recipes](recipes.md).
