use std::ops::Range;

use gtfs_rt::{
    EntitySelector, FeedEntity, FeedMessage, TripDescriptor, TripUpdate, VehiclePosition,
    feed_header::Incrementality,
    trip_descriptor::ScheduleRelationship,
    trip_update::{StopTimeEvent, StopTimeUpdate},
};
use jiff::{Timestamp, civil, tz::TimeZone};

use crate::{
    consumer::Consumer,
    models::{
        Coordinate, Date, Delay, Optional, RouteType, Sentinel, StopTime, Time, Trip, TripIndex,
    },
    rt::{
        ActivePeriod, AddedStopTime, AddedTrip, Alert, ApplyReport, Error, InformedEntity,
        Realtime, Result, StopTimeStatus, TripStatus, Vehicle, alert,
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

/// Applies GTFS-realtime feeds to a `Realtime`.
///
/// Trips are matched by `trip_id`, or by route, start time and date when there is no id. Stop times
/// are matched by `stop_sequence`, or by `stop_id`.
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
    pub fn new(consumer: &'a Consumer) -> Self {
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
            Some(TimeZone::get(name).map_err(|_| Error::UnknownTimeZone(name.to_string()))?);
        Ok(self)
    }

    /// Sets the language alert texts are taken in. Without it, or when an alert has no text in
    /// that language, the first translation is used.
    #[must_use]
    pub fn with_preferred_language(mut self, language: &str) -> Self {
        self.language = Some(language.to_string());
        self
    }

    /// Applies every message to `realtime`. Later updates replace earlier ones.
    ///
    /// # Panics
    ///
    /// Panics if `realtime` was not built from the consumer this builder was created with.
    pub fn apply<I>(&self, messages: I, realtime: &mut Realtime) -> ApplyReport
    where
        I: Iterator<Item = FeedMessage>,
    {
        assert!(
            realtime.stop_time_status.len() == self.consumer.stop_times().len()
                && realtime.trip_status.len() == self.consumer.trips().len(),
            "realtime must be built from the consumer it is applied with"
        );
        let mut report = ApplyReport::default();
        for message in messages {
            report += self.apply_message(message, realtime);
        }
        realtime.check_invariants(self.consumer);
        report
    }

    pub fn build<I>(&self, messages: I) -> Realtime
    where
        I: Iterator<Item = FeedMessage>,
    {
        self.build_with_report(messages).0
    }

    pub fn build_with_report<I>(&self, messages: I) -> (Realtime, ApplyReport)
    where
        I: Iterator<Item = FeedMessage>,
    {
        let mut realtime = Realtime::new(self.consumer);
        let report = self.apply(messages, &mut realtime);
        (realtime, report)
    }

    fn apply_message(&self, message: FeedMessage, realtime: &mut Realtime) -> ApplyReport {
        let message_time = message.header.timestamp.unwrap_or(0);
        if self.replace_on_full_dataset
            && message.header.incrementality() == Incrementality::FullDataset
        {
            clear_kinds_in(&message.entity, realtime);
        }

        let mut report = ApplyReport::default();
        for entity in message.entity {
            report += self.apply_entity(entity, message_time, realtime);
        }
        report
    }

    fn apply_entity(
        &self,
        entity: FeedEntity,
        message_time: u64,
        realtime: &mut Realtime,
    ) -> ApplyReport {
        if entity.is_deleted() {
            return self.delete_entity(&entity, realtime);
        }

        let mut report = ApplyReport::default();
        if let Some(trip_update) = &entity.trip_update {
            report += self.apply_trip_update(trip_update, message_time, realtime);
        }
        if let Some(vehicle) = &entity.vehicle {
            report += self.apply_vehicle(vehicle, realtime);
        }
        if let Some(alert) = &entity.alert {
            report += self.apply_alert(&entity.id, alert, realtime);
        }
        if entity.shape.is_some() || entity.stop.is_some() || entity.trip_modifications.is_some() {
            report.unsupported += 1;
        }
        report
    }

    fn delete_entity(&self, entity: &FeedEntity, realtime: &mut Realtime) -> ApplyReport {
        if let Some(trip_update) = &entity.trip_update {
            self.delete_trip_update(trip_update, realtime);
        }
        if let Some(descriptor) = entity
            .vehicle
            .as_ref()
            .and_then(|vehicle| vehicle.trip.as_ref())
            && let Resolved::Trip(trip) = self.resolve_trip(descriptor, realtime.service_date)
        {
            realtime.trip_vehicles[trip.as_usize()] = None;
        }
        realtime.alerts.retain(|alert| *alert.id != *entity.id);

        ApplyReport {
            deleted: 1,
            ..ApplyReport::default()
        }
    }

    fn delete_trip_update(&self, trip_update: &TripUpdate, realtime: &mut Realtime) {
        let descriptor = &trip_update.trip;
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
        let date = descriptor.start_date.as_deref().and_then(Date::parse_gtfs);
        if let (Some(expected), Some(actual)) = (realtime.service_date, date)
            && expected != actual
        {
            report.wrong_service_date += 1;
            return report;
        }
        if matches!(
            descriptor.schedule_relationship(),
            ScheduleRelationship::Replacement | ScheduleRelationship::Duplicated
        ) {
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
        realtime.trip_status[trip.as_usize()] = descriptor.schedule_relationship().into();
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
        let stop_times = self.consumer.stop_times_by_trip(trip);
        let first_slot = self.consumer.trip(trip).stop_times.start as usize;

        // Updates come in trip order, so each search starts after the last match. That is what
        // tells apart a stop a trip visits twice.
        let mut from = 0;
        for stop_update in &update.stop_time_update {
            let Some(found) = self.find_stop_time(stop_times, stop_update, from) else {
                report.stop_time_updates_unmatched += 1;
                continue;
            };
            from = found + 1;
            self.apply_stop_time_update(
                stop_update,
                first_slot + found,
                &stop_times[found],
                day,
                realtime,
            );
            report.stop_time_updates += 1;
        }

        let slots = first_slot..first_slot + stop_times.len();
        if update.stop_time_update.is_empty() {
            spread_trip_delay(slots, trip, realtime);
        } else if self.cascade_delays {
            cascade_delays(slots, realtime);
        }
        report
    }

    /// Finds the stop time an update is about, by sequence if it has one and by stop otherwise.
    fn find_stop_time(
        &self,
        stop_times: &[StopTime],
        update: &StopTimeUpdate,
        from: usize,
    ) -> Option<usize> {
        if let Some(sequence) = update.stop_sequence
            && let Ok(position) = stop_times.binary_search_by_key(&sequence, |stop| stop.sequence)
        {
            return Some(position);
        }

        let stop = self.consumer.stop_by_id(update.stop_id.as_deref()?)?.index;
        let visits = |range: Range<usize>| {
            stop_times[range.clone()]
                .iter()
                .position(|stop_time| stop_time.stop_index == stop)
                .map(|position| range.start + position)
        };
        visits(from..stop_times.len()).or_else(|| visits(0..stop_times.len()))
    }

    fn apply_stop_time_update(
        &self,
        update: &StopTimeUpdate,
        slot: usize,
        stop_time: &StopTime,
        day: Option<Date>,
        realtime: &mut Realtime,
    ) {
        let status = update.schedule_relationship().into();
        realtime.stop_time_status[slot] = status;
        if matches!(status, StopTimeStatus::Skipped | StopTimeStatus::NoData) {
            return;
        }

        let arrival = self.event_delay(update.arrival.as_ref(), stop_time.arrival_time, day);
        let mut departure =
            self.event_delay(update.departure.as_ref(), stop_time.departure_time, day);
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
        event: Option<&StopTimeEvent>,
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
        let difference = match day.and_then(|day| self.service_day_start(day)) {
            // With the service day known the difference is exact, even past midnight. A timestamp
            // can be any i64, so the subtraction clamps. The result is far outside the delay range
            // and becomes `NONE` either way.
            Some(start) => actual
                .saturating_sub(start)
                .saturating_sub(i64::from(scheduled.0)),
            // Without it both sides are times of day, so wrap to the nearest day instead of being
            // a day off for a trip just either side of midnight.
            None => {
                let local = self.local_seconds(actual);
                let half_day = SECONDS_PER_DAY / 2;
                (local.rem_euclid(SECONDS_PER_DAY) - i64::from(scheduled.0) + half_day)
                    .rem_euclid(SECONDS_PER_DAY)
                    - half_day
            }
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
    fn event_time(&self, event: Option<&StopTimeEvent>, day: Option<Date>) -> Optional<Time> {
        let Some(time) = event.and_then(|event| event.time) else {
            return Optional::new(Time::NONE);
        };
        let seconds = match day.and_then(|day| self.service_day_start(day)) {
            Some(start) => time.saturating_sub(start),
            None => self.local_seconds(time).rem_euclid(SECONDS_PER_DAY),
        };
        Optional::new(Time::from(seconds))
    }

    fn resolve_trip(&self, descriptor: &TripDescriptor, day: Option<Date>) -> Resolved {
        if let Some(trip_id) = descriptor.trip_id.as_deref() {
            return self
                .consumer
                .trip_by_id(trip_id)
                .map_or(Resolved::Unmatched, |trip| Resolved::Trip(trip.index));
        }
        self.resolve_by_start_time(descriptor, day)
    }

    /// Finds the one trip of a route that starts at the time, running on the date if there is one.
    fn resolve_by_start_time(&self, descriptor: &TripDescriptor, day: Option<Date>) -> Resolved {
        let route = descriptor
            .route_id
            .as_deref()
            .and_then(|route_id| self.consumer.route_by_id(route_id));
        let start = descriptor.start_time.as_deref().and_then(Time::from_hms);
        let (Some(route), Some(start)) = (route, start) else {
            return Resolved::Unmatched;
        };

        let mut matches = self
            .consumer
            .iter_trips_by_route(route.index)
            .filter(|trip| {
                self.starts_at(trip, start)
                    && Self::runs_in_direction(trip, descriptor)
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
    fn runs_in_direction(trip: &Trip, descriptor: &TripDescriptor) -> bool {
        let Some(wanted) = descriptor.direction_id else {
            return true;
        };
        trip.direction.is_none() || u32::from(trip.direction.0) == wanted
    }

    fn starts_at(&self, trip: &Trip, start: Time) -> bool {
        self.consumer
            .stop_times_by_trip(trip.index)
            .first()
            .and_then(|first| {
                first
                    .departure_time
                    .as_option()
                    .or_else(|| first.arrival_time.as_option())
            })
            == Some(start)
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
        let is_new = matches!(
            descriptor.schedule_relationship(),
            ScheduleRelationship::Added
        );
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
            .stop_time_update
            .iter()
            .filter_map(|stop_update| {
                let stop = self.consumer.stop_by_id(stop_update.stop_id.as_deref()?)?;
                Some(AddedStopTime {
                    stop: stop.index,
                    sequence: stop_update.stop_sequence,
                    arrival_time: self.event_time(stop_update.arrival.as_ref(), day),
                    departure_time: self.event_time(stop_update.departure.as_ref(), day),
                    status: stop_update.schedule_relationship().into(),
                })
            })
            .collect()
    }

    fn apply_vehicle(&self, vehicle: &VehiclePosition, realtime: &mut Realtime) -> ApplyReport {
        let mut report = ApplyReport::default();
        let date = vehicle
            .trip
            .as_ref()
            .and_then(|descriptor| descriptor.start_date.as_deref())
            .and_then(Date::parse_gtfs);
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
            position: Coordinate::new(f64::from(position.latitude), f64::from(position.longitude)),
            current_status: vehicle.current_status(),
            occupancy_status: vehicle.occupancy_status(),
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
        });
        report.vehicles += 1;
        report
    }

    fn apply_alert(
        &self,
        id: &str,
        source: &gtfs_rt::Alert,
        realtime: &mut Realtime,
    ) -> ApplyReport {
        let language = self.language.as_deref();
        let alert = Alert {
            id: Box::from(id),
            active_periods: source
                .active_period
                .iter()
                .map(|period| ActivePeriod {
                    start: period.start,
                    end: period.end,
                })
                .collect(),
            informed: source
                .informed_entity
                .iter()
                .filter_map(|selector| self.informed_entity(selector))
                .collect(),
            cause: source.cause(),
            effect: source.effect(),
            severity: source.severity_level(),
            header: alert::translated(source.header_text.as_ref(), language),
            description: alert::translated(source.description_text.as_ref(), language),
            url: alert::translated(source.url.as_ref(), language),
        };

        realtime.alerts.retain(|existing| existing.id != alert.id);
        // An alert whose entities are all unknown to this network is about something else.
        if !source.informed_entity.is_empty() && alert.informed.is_empty() {
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
    fn informed_entity(&self, selector: &EntitySelector) -> Option<InformedEntity> {
        let route = match selector.route_id.as_deref() {
            Some(route_id) => Optional::new(self.consumer.route_by_id(route_id)?.index),
            None => Optional::default(),
        };
        let stop = match selector.stop_id.as_deref() {
            Some(stop_id) => Optional::new(self.consumer.stop_by_id(stop_id)?.index),
            None => Optional::default(),
        };
        let trip = match selector
            .trip
            .as_ref()
            .and_then(|descriptor| descriptor.trip_id.as_deref())
        {
            Some(trip_id) => Optional::new(self.consumer.trip_by_id(trip_id)?.index),
            None => Optional::default(),
        };
        let route_type = selector
            .route_type
            .and_then(|code| u16::try_from(code).ok())
            .map(RouteType::from)
            .into();

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

/// Forgets the kinds of entity a full dataset message is about, so what it leaves out disappears.
fn clear_kinds_in(entities: &[FeedEntity], realtime: &mut Realtime) {
    if entities.iter().any(|entity| entity.trip_update.is_some()) {
        realtime.clear_trip_updates();
    }
    if entities.iter().any(|entity| entity.vehicle.is_some()) {
        realtime.clear_vehicles();
    }
    if entities.iter().any(|entity| entity.alert.is_some()) {
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
