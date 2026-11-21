# Dates, times and service days

A timetable depends on a day, and a trip's time is not a clock time. This page explains what flatnet stores and how to ask "does this trip run today, and when".

## `Date`

A `Date` is a number of days since 1970-01-01. It is a plain `u32` wrapper, so dates compare and subtract.

```rust
use flatnet_core::models::Date;

fn dates() {
    // The GTFS form: exactly eight digits.
    let date = Date::parse_gtfs("20260315").expect("a real day");

    // From parts. `None` for an impossible day, a year before 1970, or one after 9999.
    assert_eq!(Date::from_year_month_day(2026, 3, 15), Some(date));

    // Back to parts, and printing.
    assert_eq!(date.to_year_month_day(), Some((2026, 3, 15)));
    assert_eq!(date.to_string(), "2026-03-15");

    // Monday is 0, Sunday is 6.
    assert_eq!(date.day_of_week(), 6);

    // The next day.
    let tomorrow = Date(date.0 + 1);
    assert_eq!(tomorrow.to_string(), "2026-03-16");
}
```

The range is 1970-01-01 to 9999-12-31. `to_year_month_day` returns `None` for the sentinel `Date::NONE` (`u32::MAX`) and for anything past year 9999.

## `Time`

A `Time` is **seconds since the start of the service day**, not since midnight.

```rust
use flatnet_core::models::Time;

fn times() {
    let nine = Time::from_hms("09:00:00").expect("a time");
    assert_eq!(nine, Time(9 * 3600));
    assert_eq!(nine.to_string(), "09:00:00");

    // A trip that runs after midnight on its service day is past 24:00:00.
    let late = Time::from_hms("25:10:00").expect("a time");
    assert_eq!(late.0, 25 * 3600 + 10 * 60);
    assert_eq!(late.to_string(), "25:10:00");

    // Ordering is by seconds.
    assert!(nine < late);
}
```

Why "service day": a bus that leaves at 00:30 on Saturday belongs to Friday's timetable if it is the last run of Friday's service. Its time is then `24:30:00` and its day is Friday. Every time in the file works this way, and so does every service check below.

A `Time` can be `Time::NONE`. You rarely meet it directly: absent times are wrapped in `Optional<Time>` and you read them with `.as_option()`.

### Adding to a time

`Time` adds `Duration` (a span) and `Delay` (a signed span, from realtime), and subtracting two times gives a `Duration`:

```rust
use flatnet_core::models::{Delay, Duration, Time};

fn arithmetic() {
    let departure = Time(9 * 3600);

    let later = departure + Duration(300);
    assert_eq!(later, Time(9 * 3600 + 300));

    let delayed = departure + Delay(-60);
    assert_eq!(delayed, Time(9 * 3600 - 60));

    let wait = later - departure;
    assert_eq!(wait, Duration(300));
}
```

These saturate, so a delay can't wrap a time around. `Duration` is seconds as a `u32`. `Delay` is seconds as an `i16`, so about plus or minus nine hours.

## Services

A **service** is a pattern of days: "weekdays in spring", "every day except Christmas". Each trip points at one with `trip.service_index`. The compiler expands each service to one bit per day between its first and last day, so asking costs one array read.

```rust
use flatnet_core::{
    consumer::Consumer,
    models::{Date, TripIndex},
};

fn runs_on(consumer: &Consumer<'_>, trip: TripIndex, date: Date) -> bool {
    consumer.is_service_active(consumer.trip(trip).service_index, date)
}
```

`is_service_active(service, date)` is false when:

- the date is before the service's first day or after its last,
- the service doesn't run that day (a weekly pattern, or removed by an exception),
- the trip has no service (`ServiceIndex::NONE`).

A service's first and last day come from its weekly pattern and its exceptions together: an added day outside the weekly range extends the service.

```rust
use flatnet_core::consumer::Consumer;

fn service_days(consumer: &Consumer<'_>) {
    let Some(service) = consumer.service_by_id("weekdays") else {
        return;
    };
    println!("runs between {} and {}", service.start_date, service.end_date);
}
```

### The whole file's range

```rust
use flatnet_core::consumer::Consumer;

fn range(consumer: &Consumer<'_>) {
    if let Some((first, last)) = consumer.service_range() {
        println!("service from {first} to {last}");
    }
}
```

## Which trips run today

Combine the two. For a stop, today's trips are the ones whose service is active:

```rust
use flatnet_core::{
    consumer::Consumer,
    models::{Date, StopIndex},
};

fn running_today(consumer: &Consumer<'_>, stop: StopIndex, today: Date) -> usize {
    consumer
        .iter_trips_by_stop(stop)
        .filter(|trip| consumer.is_service_active(trip.service_index, today))
        .count()
}
```

### Trips that started yesterday

A trip with times past 24:00:00 is still running after midnight, but its service day is the day before. If you want everything that is moving at 00:30 on Saturday, you need two checks:

- trips of **Saturday's** services with a time at or after `00:30:00`, and
- trips of **Friday's** services with a time at or after `24:30:00`.

```rust
use flatnet_core::{
    consumer::Consumer,
    models::{Date, Time, TripIndex},
};

/// Whether the trip departs the stop's position exactly at `seconds_into_day` on `date`, counting
/// the trips of the day before that run past midnight.
fn is_there(
    consumer: &Consumer<'_>,
    trip: TripIndex,
    position: usize,
    date: Date,
    seconds_into_day: u32,
) -> bool {
    let timing = consumer.timings_by_trip(trip)[position];
    let Some(time) = timing.departure.as_option() else {
        return false;
    };
    let service = consumer.trip(trip).service_index;
    let same_day = time == Time(seconds_into_day) && consumer.is_service_active(service, date);
    let day_before = Date(date.0 - 1);
    let from_yesterday = time == Time(seconds_into_day + 24 * 3600)
        && consumer.is_service_active(service, day_before);
    same_day || from_yesterday
}
```

[Realtime](realtime.md) has the same rule: one `Realtime` per service day, so a trip that runs past midnight can't mix up two days.

## Time zones

Times in the file are local times of the agency. The file stores the agency's zone name (`consumer.timezone()`, such as `Europe/Stockholm`), and the realtime builder uses it to turn a POSIX timestamp from a feed into a delay, including on the two days a year the clocks change. Nothing else in the file needs a zone: a timetable is "09:00 on this date", wherever you are.

To get "today" as a `Date` in the agency's zone, use your date library with `consumer.timezone()` and convert with `Date::from_year_month_day`.

Next: [Finding things](finding-things.md).
