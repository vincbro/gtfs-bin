use std::ops::Range;

use jiff::{Timestamp, civil, tz::TimeZone};

use crate::{
    consumer::Consumer,
    models::{
        Coordinate, Date, Delay, Optional, RouteType, Sentinel, Time, Timing, Trip, TripIndex,
        TripPatternIndex,
    },
    rt::{
        AddedStopTime, AddedTrip, Alert, AlertUpdate, ApplyReport, CallUpdate, Error, Event, Feed,
        InformedEntity, Kinds, Realtime, Removal, Result, Selector, StopTimeStatus, TripRef,
        TripStatus, TripUpdate, Update, Vehicle, VehicleUpdate, alert,
    },
};

const SECONDS_PER_DAY: i64 = 86_400;

/// The outcome of looking for the trip a descriptor names.
enum Resolved {
    Trip(TripIndex),
    Unmatched,
    /// The descriptor fits more than one trip, so none is chosen.
    Ambiguous,
}

/// Applies a realtime source's feeds to a `Realtime`, knowing nothing of the wire format they came
/// in. With the `gtfs-rt` feature, `apply` and `build` take GTFS-realtime messages.
///
/// Trips are matched by `trip_id`, or by route, start time and date when there is no id. Stop times
/// are matched by `stop_sequence`, or by `stop_id`.
#[derive(Debug)]
pub struct RealtimeBuilder<'a> {
    consumer: &'a Consumer<'a>,
    cascade_delays: bool,
    replace_on_full_dataset: bool,
    utc_offset_seconds: i32,
    time_zone: Option<TimeZone>,
    language: Option<String>,
}

impl<'a> RealtimeBuilder<'a> {
    #[must_use]
    pub fn new(consumer: &'a Consumer<'_>) -> Self {
        Self {
            consumer,
            cascade_delays: true,
            replace_on_full_dataset: true,
            utc_offset_seconds: 0,
            // The feed says which zone its times are in, so use it unless told otherwise.
            time_zone: consumer
                .timezone()
                .and_then(|name| TimeZone::get(name).ok()),
            language: None,
        }
    }

    /// Sets whether a stop time without an update of its own takes the delay of the nearest
    /// updated stop time before it. This is on by default.
    #[must_use]
    pub const fn with_cascading_delays(mut self, cascade_delays: bool) -> Self {
        self.cascade_delays = cascade_delays;
        self
    }

    /// Sets whether a feed that says it is a full dataset replaces what an earlier feed set.
    ///
    /// Only the kinds of entity the feed contains are replaced, so a feed of vehicle positions
    /// leaves trip updates alone. This is on by default, as the GTFS-realtime spec says a feed is
    /// a full dataset unless it states otherwise. Turn it off when several feeds each hold part of
    /// the same kind of entity.
    #[must_use]
    pub const fn with_replace_on_full_dataset(mut self, replace_on_full_dataset: bool) -> Self {
        self.replace_on_full_dataset = replace_on_full_dataset;
        self
    }

    /// Sets a fixed offset from UTC, in seconds, in place of the agency's time zone.
    ///
    /// Stop times are local times of day, but feeds give absolute timestamps, so a delay computed
    /// from a timestamp needs to know the zone. By default the builder uses the zone stored in the
    /// file, which follows daylight saving time. A fixed offset is for a file that has none.
    #[must_use]
    pub fn with_utc_offset_seconds(mut self, utc_offset_seconds: i32) -> Self {
        self.utc_offset_seconds = utc_offset_seconds;
        self.time_zone = None;
        self
    }

    /// Sets the time zone by name, such as `Europe/Stockholm`, in place of the one in the file.
    ///
    /// # Errors
    ///
    /// Returns an error if the name isn't a time zone the system knows.
    pub fn with_time_zone(mut self, name: &str) -> Result<Self> {
        self.time_zone =
            Some(TimeZone::get(name).map_err(|_| Error::UnknownTimeZone(name.to_owned()))?);
        Ok(self)
    }

    /// Sets the language alert texts are taken in. Without it, or when an alert has no text in
    /// that language, the first translation is used.
    #[must_use]
    pub fn with_preferred_language(mut self, language: &str) -> Self {
        self.language = Some(language.to_owned());
        self
    }

    /// Applies every feed to `realtime`. Later updates replace earlier ones.
    ///
    /// # Panics
    ///
    /// Panics if `realtime` was not built from the consumer this builder was created with.
    pub fn apply_feeds<I>(&self, feeds: I, realtime: &mut Realtime) -> ApplyReport
    where
        I: IntoIterator<Item = Feed>,
    {
        assert!(
            realtime.stop_time_status.len() == self.consumer.timings().len()
                && realtime.trip_status.len() == self.consumer.trips().len(),
            "realtime must be built from the consumer it is applied with"
        );
        let mut report = ApplyReport::default();
        for feed in feeds {
            report += self.apply_feed(&feed, realtime);
        }
        realtime.check_invariants(self.consumer);
        report
    }

    pub fn build_feeds<I>(&self, feeds: I) -> Realtime
    where
        I: IntoIterator<Item = Feed>,
    {
        self.build_feeds_with_report(feeds).0
    }

    pub fn build_feeds_with_report<I>(&self, feeds: I) -> (Realtime, ApplyReport)
    where
        I: IntoIterator<Item = Feed>,
    {
        let mut realtime = Realtime::new(self.consumer);
        let report = self.apply_feeds(feeds, &mut realtime);
        (realtime, report)
    }

    fn apply_feed(&self, feed: &Feed, realtime: &mut Realtime) -> ApplyReport {
        if self.replace_on_full_dataset {
            clear_kinds(feed.replaces, realtime);
        }

        let mut report = ApplyReport::default();
        for update in &feed.updates {
            report += match update {
                Update::Trip(update) => self.apply_trip_update(update, feed.timestamp, realtime),
                Update::Vehicle(update) => self.apply_vehicle(update, realtime),
                Update::Alert(update) => self.apply_alert(update, realtime),
                Update::Remove(removal) => self.remove(removal, realtime),
                Update::Unsupported => ApplyReport {
                    unsupported: 1,
                    ..ApplyReport::default()
                },
            };
        }
        report
    }

    fn remove(&self, removal: &Removal, realtime: &mut Realtime) -> ApplyReport {
        if let Some(trip) = &removal.trip {
            self.delete_trip_update(trip, realtime);
        }
        if let Some(descriptor) = &removal.vehicle_trip
            && let Resolved::Trip(trip) = self.resolve_trip(descriptor, realtime.service_date)
        {
            realtime.trip_vehicles[trip.as_usize()] = None;
        }
        realtime.alerts.retain(|alert| *alert.id != *removal.id);

        ApplyReport {
            deleted: 1,
            ..ApplyReport::default()
        }
    }

    fn delete_trip_update(&self, descriptor: &TripRef, realtime: &mut Realtime) {
        if let Some(trip_id) = &descriptor.trip_id {
            realtime.added_trips.retain(|added| *added.id != **trip_id);
        }
        if let Resolved::Trip(trip) = self.resolve_trip(descriptor, realtime.service_date) {
            realtime.reset_trip(self.consumer, trip);
            realtime.trip_timestamps[trip.as_usize()] = 0;
        }
    }

    fn apply_trip_update(
        &self,
        update: &TripUpdate,
        message_time: u64,
        realtime: &mut Realtime,
    ) -> ApplyReport {
        let mut report = ApplyReport::default();
        let descriptor = &update.trip;
        let date = descriptor.start_date;
        if let (Some(expected), Some(actual)) = (realtime.service_date, date)
            && expected != actual
        {
            report.wrong_service_date += 1;
            return report;
        }
        if update.unsupported {
            report.unsupported += 1;
            return report;
        }

        let day = date.or(realtime.service_date);
        let timestamp = update.timestamp.unwrap_or(message_time);
        let trip = match self.resolve_trip(descriptor, day) {
            Resolved::Trip(trip) => trip,
            Resolved::Ambiguous => {
                report.trip_updates_ambiguous += 1;
                return report;
            }
            Resolved::Unmatched => {
                return self.apply_unmatched_trip(update, day, timestamp, realtime);
            }
        };

        let stored = realtime.trip_timestamps[trip.as_usize()];
        if timestamp != 0 && stored != 0 && timestamp < stored {
            report.trip_updates_stale += 1;
            return report;
        }

        realtime.reset_trip(self.consumer, trip);
        realtime.trip_timestamps[trip.as_usize()] = timestamp;
        realtime.trip_status[trip.as_usize()] = update.status;
        realtime.trip_delay[trip.as_usize()] =
            update.delay.map_or(Optional::new(Delay::NONE), |delay| {
                Optional::new(Delay::from(delay))
            });
        report.trip_updates += 1;

        if !matches!(
            realtime.trip_status[trip.as_usize()],
            TripStatus::Canceled | TripStatus::Deleted
        ) {
            report += self.apply_stop_time_updates(update, trip, day, realtime);
        }
        report
    }

    fn apply_stop_time_updates(
        &self,
        update: &TripUpdate,
        trip: TripIndex,
        day: Option<Date>,
        realtime: &mut Realtime,
    ) -> ApplyReport {
        let mut report = ApplyReport::default();
        let timings = self.consumer.timings_by_trip(trip);
        let pattern = self.consumer.trip_pattern_by_trip(trip).index;
        let first_slot = self.consumer.trip(trip).stop_times.start as usize;

        // Updates come in trip order, so each search starts after the last match. That is what
        // tells apart a stop a trip visits twice.
        let mut from = 0;
        for stop_update in &update.calls {
            let Some(found) = self.find_stop_time(pattern, stop_update, from) else {
                report.stop_time_updates_unmatched += 1;
                continue;
            };
            from = found + 1;
            self.apply_stop_time_update(
                stop_update,
                first_slot + found,
                timings[found],
                day,
                realtime,
            );
            report.stop_time_updates += 1;
        }

        let slots = first_slot..first_slot + timings.len();
        if update.calls.is_empty() {
            spread_trip_delay(slots, trip, realtime);
        } else if self.cascade_delays {
            cascade_delays(slots, realtime);
        }
        report
    }

    /// Finds the stop time an update is about, by sequence if it has one and by stop otherwise.
    fn find_stop_time(
        &self,
        pattern: TripPatternIndex,
        update: &CallUpdate,
        from: usize,
    ) -> Option<usize> {
        let pattern_stops = self.consumer.pattern_stops_by_trip_pattern(pattern);
        if let Some(sequence) = update.sequence
            && let Ok(position) =
                pattern_stops.binary_search_by_key(&sequence, |stop| stop.sequence)
        {
            return Some(position);
        }

        let stop = self.consumer.stop_by_id(update.stop_id.as_deref()?)?.index;
        let stops = self.consumer.stop_sequence_by_trip_pattern(pattern);
        let visits = |range: Range<usize>| {
            stops[range.clone()]
                .iter()
                .position(|&visited| visited == stop)
                .map(|position| range.start + position)
        };
        visits(from..stops.len()).or_else(|| visits(0..stops.len()))
    }

    fn apply_stop_time_update(
        &self,
        update: &CallUpdate,
        slot: usize,
        timing: Timing,
        day: Option<Date>,
        realtime: &mut Realtime,
    ) {
        let status = update.status;
        realtime.stop_time_status[slot] = status;
        realtime.stop_time_occupancy[slot] = update.occupancy;
        if matches!(status, StopTimeStatus::Skipped | StopTimeStatus::NoData) {
            return;
        }

        let arrival = self.event_delay(update.arrival, timing.arrival, day);
        let mut departure = self.event_delay(update.departure, timing.departure, day);
        // A vehicle that arrives late leaves late, unless the feed says otherwise.
        if update.departure.is_none() {
            departure = arrival;
        }
        realtime.stop_time_arrival_delays[slot] = arrival;
        realtime.stop_time_departure_delays[slot] = departure;
    }

    /// Returns the delay an arrival or departure event reports, from its `delay` or else from how
    /// far its `time` is from the scheduled time.
    fn event_delay(
        &self,
        event: Option<Event>,
        scheduled: Optional<Time>,
        day: Option<Date>,
    ) -> Optional<Delay> {
        let none = Optional::new(Delay::NONE);
        let Some(event) = event else {
            return none;
        };
        if let Some(delay) = event.delay {
            return Optional::new(Delay::from(delay));
        }
        let (Some(actual), Some(scheduled)) = (event.time, scheduled.as_option()) else {
            return none;
        };
        let difference = if let Some(start) = day.and_then(|day| self.service_day_start(day)) {
            // With the service day known the difference is exact, even past midnight. A timestamp
            // can be any i64, so the subtraction clamps. The result is far outside the delay range
            // and becomes `NONE` either way.
            actual
                .saturating_sub(start)
                .saturating_sub(i64::from(scheduled.0))
        } else {
            // Without it both sides are times of day, so wrap to the nearest day instead of being
            // a day off for a trip just either side of midnight.
            let local = self.local_seconds(actual);
            let half_day = SECONDS_PER_DAY / 2;
            (local.rem_euclid(SECONDS_PER_DAY) - i64::from(scheduled.0) + half_day)
                .rem_euclid(SECONDS_PER_DAY)
                - half_day
        };
        Optional::new(Delay::from(difference))
    }

    /// Returns the POSIX time the service day starts at, which the GTFS spec defines as noon on
    /// the date minus 12 hours. That is midnight except on the days the clocks change.
    fn service_day_start(&self, day: Date) -> Option<i64> {
        let Some(zone) = &self.time_zone else {
            return Some(i64::from(day.0) * SECONDS_PER_DAY - i64::from(self.utc_offset_seconds));
        };
        let (year, month, date) = day.to_year_month_day()?;
        let noon = civil::date(
            i16::try_from(year).ok()?,
            i8::try_from(month).ok()?,
            i8::try_from(date).ok()?,
        )
        .at(12, 0, 0, 0)
        .to_zoned(zone.clone())
        .ok()?;
        Some(noon.timestamp().as_second() - SECONDS_PER_DAY / 2)
    }

    /// Returns the seconds the local clock is ahead of UTC at a POSIX time.
    fn offset_at(&self, timestamp: i64) -> i64 {
        let Some(zone) = &self.time_zone else {
            return i64::from(self.utc_offset_seconds);
        };
        Timestamp::from_second(timestamp).map_or(0, |timestamp| {
            i64::from(zone.to_offset(timestamp).seconds())
        })
    }

    fn local_seconds(&self, timestamp: i64) -> i64 {
        timestamp.saturating_add(self.offset_at(timestamp))
    }

    /// Returns an event's time as seconds since the start of the service day.
    fn event_time(&self, event: Option<Event>, day: Option<Date>) -> Optional<Time> {
        let Some(time) = event.and_then(|event| event.time) else {
            return Optional::new(Time::NONE);
        };
        let seconds = match day.and_then(|day| self.service_day_start(day)) {
            Some(start) => time.saturating_sub(start),
            None => self.local_seconds(time).rem_euclid(SECONDS_PER_DAY),
        };
        Optional::new(Time::from(seconds))
    }

    fn resolve_trip(&self, descriptor: &TripRef, day: Option<Date>) -> Resolved {
        if let Some(trip_id) = descriptor.trip_id.as_deref() {
            return self
                .consumer
                .trip_by_id(trip_id)
                .map_or(Resolved::Unmatched, |trip| Resolved::Trip(trip.index));
        }
        self.resolve_by_start_time(descriptor, day)
    }

    /// Finds the one trip of a route that starts at the time, running on the date if there is one.
    fn resolve_by_start_time(&self, descriptor: &TripRef, day: Option<Date>) -> Resolved {
        let route = descriptor
            .route_id
            .as_deref()
            .and_then(|route_id| self.consumer.route_by_id(route_id));
        let start = descriptor.start_time;
        let (Some(route), Some(start)) = (route, start) else {
            return Resolved::Unmatched;
        };

        let mut matches = self
            .consumer
            .trips_by_route_start(route.index, start)
            .iter()
            .map(|&index| self.consumer.trip(index))
            .filter(|trip| {
                Self::runs_in_direction(trip, descriptor)
                    && day
                        .is_none_or(|day| self.consumer.is_service_active(trip.service_index, day))
            });
        match (matches.next(), matches.next()) {
            (Some(trip), None) => Resolved::Trip(trip.index),
            (Some(_), Some(_)) => Resolved::Ambiguous,
            _ => Resolved::Unmatched,
        }
    }

    /// A trip with no direction fits any, as does a descriptor with none.
    fn runs_in_direction(trip: &Trip, descriptor: &TripRef) -> bool {
        let Some(wanted) = descriptor.direction else {
            return true;
        };
        trip.direction.is_none() || u32::from(trip.direction.0) == wanted
    }

    /// Stores a trip the static data doesn't have, if the feed says it is a new one.
    fn apply_unmatched_trip(
        &self,
        update: &TripUpdate,
        day: Option<Date>,
        timestamp: u64,
        realtime: &mut Realtime,
    ) -> ApplyReport {
        let mut report = ApplyReport::default();
        let descriptor = &update.trip;
        let is_new = update.status == TripStatus::Added;
        let (true, Some(trip_id)) = (is_new, descriptor.trip_id.as_deref()) else {
            report.trip_updates_unmatched += 1;
            return report;
        };

        let stored = realtime
            .added_trips
            .iter()
            .find(|added| *added.id == *trip_id)
            .map_or(0, |added| added.timestamp);
        if timestamp != 0 && stored != 0 && timestamp < stored {
            report.trip_updates_stale += 1;
            return report;
        }

        let added = AddedTrip {
            id: Box::from(trip_id),
            route: descriptor
                .route_id
                .as_deref()
                .and_then(|route_id| self.consumer.route_by_id(route_id))
                .map(|route| route.index)
                .into(),
            start_date: day,
            stop_times: self.added_stop_times(update, day),
            timestamp,
        };
        realtime
            .added_trips
            .retain(|existing| existing.id != added.id);
        realtime.added_trips.push(added);
        report.added_trips += 1;
        report
    }

    fn added_stop_times(&self, update: &TripUpdate, day: Option<Date>) -> Box<[AddedStopTime]> {
        update
            .calls
            .iter()
            .filter_map(|stop_update| {
                let stop = self.consumer.stop_by_id(stop_update.stop_id.as_deref()?)?;
                Some(AddedStopTime {
                    stop: stop.index,
                    sequence: stop_update.sequence,
                    arrival_time: self.event_time(stop_update.arrival, day),
                    departure_time: self.event_time(stop_update.departure, day),
                    status: stop_update.status,
                })
            })
            .collect()
    }

    fn apply_vehicle(&self, vehicle: &VehicleUpdate, realtime: &mut Realtime) -> ApplyReport {
        let mut report = ApplyReport::default();
        let date = vehicle
            .trip
            .as_ref()
            .and_then(|descriptor| descriptor.start_date);
        if let (Some(expected), Some(actual)) = (realtime.service_date, date)
            && expected != actual
        {
            report.wrong_service_date += 1;
            return report;
        }

        let resolved = vehicle
            .trip
            .as_ref()
            .map(|descriptor| self.resolve_trip(descriptor, date.or(realtime.service_date)));
        let (Some(Resolved::Trip(trip)), Some(position)) = (resolved, vehicle.position.as_ref())
        else {
            report.vehicles_unmatched += 1;
            return report;
        };

        let slot = &mut realtime.trip_vehicles[trip.as_usize()];
        let is_older =
            |newer: &Vehicle| newer.timestamp > vehicle.timestamp && vehicle.timestamp.is_some();
        if slot.as_ref().is_some_and(is_older) {
            return report;
        }
        *slot = Some(Vehicle {
            position: Coordinate::new(position.latitude, position.longitude),
            current_status: vehicle.status,
            occupancy_status: vehicle.occupancy,
            occupancy_percentage: vehicle.occupancy_percentage,
            current_stop_sequence: vehicle.current_stop_sequence,
            current_stop: vehicle
                .stop_id
                .as_deref()
                .and_then(|stop_id| self.consumer.stop_by_id(stop_id))
                .map(|stop| stop.index)
                .into(),
            bearing: position.bearing,
            speed: position.speed,
            timestamp: vehicle.timestamp,
            description: vehicle.description.clone().map(Box::new),
        });
        report.vehicles += 1;
        report
    }

    fn apply_alert(&self, source: &AlertUpdate, realtime: &mut Realtime) -> ApplyReport {
        let language = self.language.as_deref();
        let alert = Alert {
            id: Box::from(source.id.as_str()),
            active_periods: source.active_periods.iter().copied().collect(),
            informed: source
                .selectors
                .iter()
                .filter_map(|selector| self.informed_entity(selector))
                .collect(),
            cause: source.cause,
            effect: source.effect,
            severity: source.severity,
            header: alert::translated(source.header.as_ref(), language),
            description: alert::translated(source.description.as_ref(), language),
            url: alert::translated(source.url.as_ref(), language),
        };

        realtime.alerts.retain(|existing| existing.id != alert.id);
        // An alert whose entities are all unknown to this network is about something else.
        if !source.selectors.is_empty() && alert.informed.is_empty() {
            return ApplyReport::default();
        }
        realtime.alerts.push(alert);
        ApplyReport {
            alerts: 1,
            ..ApplyReport::default()
        }
    }

    /// Resolves what an alert is about. An id the static data doesn't have makes the whole entity
    /// useless, because dropping just that id would widen what the alert applies to.
    fn informed_entity(&self, selector: &Selector) -> Option<InformedEntity> {
        let route = match selector.route_id.as_deref() {
            Some(route_id) => Optional::new(self.consumer.route_by_id(route_id)?.index),
            None => Optional::default(),
        };
        let stop = match selector.stop_id.as_deref() {
            Some(stop_id) => Optional::new(self.consumer.stop_by_id(stop_id)?.index),
            None => Optional::default(),
        };
        let trip = match selector.trip_id.as_deref() {
            Some(trip_id) => Optional::new(self.consumer.trip_by_id(trip_id)?.index),
            None => Optional::default(),
        };
        let route_type = selector.route_type.map(RouteType::from).into();

        let entity = InformedEntity {
            agency_id: selector.agency_id.clone(),
            route,
            route_type,
            trip,
            stop,
        };
        let is_empty = entity == InformedEntity::default();
        (!is_empty).then_some(entity)
    }
}

/// Forgets the kinds of update a full dataset is about, so what it leaves out disappears.
fn clear_kinds(kinds: Kinds, realtime: &mut Realtime) {
    if kinds.trip_updates {
        realtime.clear_trip_updates();
    }
    if kinds.vehicles {
        realtime.clear_vehicles();
    }
    if kinds.alerts {
        realtime.clear_alerts();
    }
}

/// Gives every stop time of a trip the trip's own delay, for a trip update that has no stop times.
fn spread_trip_delay(slots: Range<usize>, trip: TripIndex, realtime: &mut Realtime) {
    let delay = realtime.trip_delay[trip.as_usize()];
    if delay.is_none() {
        return;
    }
    realtime.stop_time_arrival_delays[slots.clone()].fill(delay);
    realtime.stop_time_departure_delays[slots].fill(delay);
}

/// Gives each stop time without an update the delay of the nearest updated one before it.
///
/// A skipped stop passes the delay on, and a stop with no data ends it, as the spec says.
fn cascade_delays(slots: Range<usize>, realtime: &mut Realtime) {
    let mut arrival = Optional::new(Delay::NONE);
    let mut departure = Optional::new(Delay::NONE);
    for slot in slots {
        match realtime.stop_time_status[slot] {
            StopTimeStatus::Skipped => {}
            StopTimeStatus::NoData => {
                arrival = Optional::new(Delay::NONE);
                departure = Optional::new(Delay::NONE);
            }
            StopTimeStatus::Unchanged => {
                realtime.stop_time_arrival_delays[slot] = arrival;
                realtime.stop_time_departure_delays[slot] = departure;
            }
            StopTimeStatus::Scheduled | StopTimeStatus::Unscheduled => {
                carry(realtime.stop_time_arrival_delays[slot], &mut arrival);
                carry(realtime.stop_time_departure_delays[slot], &mut departure);
            }
        }
    }
}

fn carry(delay: Optional<Delay>, carried: &mut Optional<Delay>) {
    if delay.is_some() {
        *carried = delay;
    }
}
