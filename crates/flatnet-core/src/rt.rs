use gtfs_rt::{
    trip_descriptor,
    trip_update::stop_time_update,
    vehicle_position::{OccupancyStatus, VehicleStopStatus},
};

use crate::{
    consumer::Consumer,
    models::{
        Coordinate, Date, Delay, Optional, RouteIndex, Sentinel, StopIndex, StopTimeIndex, Time,
        Timing, TripIndex,
    },
};

mod added;
mod alert;
mod builder;
mod decode;
mod error;
mod report;

pub use added::{AddedStopTime, AddedTrip};
pub use alert::{ActivePeriod, Alert, InformedEntity};
pub use builder::RealtimeBuilder;
pub use decode::decode_feed;
pub use error::{Error, Result};
pub use report::ApplyReport;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TripStatus {
    /// No update has been applied.
    #[default]
    Unchanged,
    /// An update confirms the trip runs, with or without delays.
    Scheduled,
    Added,
    Canceled,
    Deleted,
    Unscheduled,
}

impl From<trip_descriptor::ScheduleRelationship> for TripStatus {
    fn from(value: trip_descriptor::ScheduleRelationship) -> Self {
        match value {
            trip_descriptor::ScheduleRelationship::Scheduled => Self::Scheduled,
            trip_descriptor::ScheduleRelationship::Added => Self::Added,
            trip_descriptor::ScheduleRelationship::Unscheduled => Self::Unscheduled,
            trip_descriptor::ScheduleRelationship::Canceled => Self::Canceled,
            trip_descriptor::ScheduleRelationship::Deleted => Self::Deleted,
            // Replaced and duplicated trips aren't supported, so they leave the trip as it was.
            _ => Self::Unchanged,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopTimeStatus {
    /// No update has been applied.
    #[default]
    Unchanged,
    Scheduled,
    Skipped,
    NoData,
    Unscheduled,
}

impl From<stop_time_update::ScheduleRelationship> for StopTimeStatus {
    fn from(value: stop_time_update::ScheduleRelationship) -> Self {
        match value {
            stop_time_update::ScheduleRelationship::Scheduled => Self::Scheduled,
            stop_time_update::ScheduleRelationship::Skipped => Self::Skipped,
            stop_time_update::ScheduleRelationship::NoData => Self::NoData,
            stop_time_update::ScheduleRelationship::Unscheduled => Self::Unscheduled,
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Vehicle {
    pub position: Coordinate,
    pub current_status: VehicleStopStatus,
    pub occupancy_status: OccupancyStatus,
    pub occupancy_percentage: Option<u32>,
    pub current_stop_sequence: Option<u32>,
    /// The stop the vehicle is at or heading to, if the feed names one the static data has.
    pub current_stop: Optional<StopIndex>,
    /// Degrees clockwise from north.
    pub bearing: Option<f32>,
    /// Meters per second.
    pub speed: Option<f32>,
    /// When the position was measured, as a POSIX time.
    pub timestamp: Option<u64>,
}

/// The live state of every stop time and trip of one `Consumer`, indexed the same way, plus the
/// alerts and added trips that have no place in the static tables.
///
/// After every public call:
/// - The `stop_time_*` tables have one entry per stop time of the consumer it was built from.
/// - The `trip_*` tables have one entry per trip of that consumer.
///
/// Nothing resizes the tables, so a `Realtime` can only be applied with the consumer it came from.
///
/// A `Realtime` describes one service day when it is made with `for_service_date`, and an update
/// for another day is ignored. Keep one per day to follow trips that run past midnight.
#[derive(Debug)]
pub struct Realtime {
    service_date: Option<Date>,
    stop_time_departure_delays: Box<[Optional<Delay>]>,
    stop_time_arrival_delays: Box<[Optional<Delay>]>,
    stop_time_status: Box<[StopTimeStatus]>,
    trip_status: Box<[TripStatus]>,
    trip_delay: Box<[Optional<Delay>]>,
    trip_vehicles: Box<[Option<Vehicle>]>,
    /// The POSIX time of the update each trip's state comes from, or 0.
    trip_timestamps: Box<[u64]>,
    added_trips: Vec<AddedTrip>,
    alerts: Vec<Alert>,
}

impl Realtime {
    /// Creates a state with nothing updated, sized for `consumer`, that accepts updates for any
    /// service date.
    #[must_use]
    pub fn new(consumer: &Consumer<'_>) -> Self {
        let stop_time_count = consumer.timings().len();
        let trip_count = consumer.trips().len();
        Self {
            service_date: None,
            stop_time_departure_delays: vec![Optional::new(Delay::NONE); stop_time_count]
                .into_boxed_slice(),
            stop_time_arrival_delays: vec![Optional::new(Delay::NONE); stop_time_count]
                .into_boxed_slice(),
            stop_time_status: vec![StopTimeStatus::Unchanged; stop_time_count].into_boxed_slice(),
            trip_status: vec![TripStatus::Unchanged; trip_count].into_boxed_slice(),
            trip_vehicles: vec![None; trip_count].into_boxed_slice(),
            trip_delay: vec![Optional::new(Delay::NONE); trip_count].into_boxed_slice(),
            trip_timestamps: vec![0; trip_count].into_boxed_slice(),
            added_trips: Vec::new(),
            alerts: Vec::new(),
        }
    }

    /// Creates a state for one service day. Trip updates that name another day are ignored.
    #[must_use]
    pub fn for_service_date(consumer: &Consumer<'_>, service_date: Date) -> Self {
        Self {
            service_date: Some(service_date),
            ..Self::new(consumer)
        }
    }

    #[must_use]
    pub const fn service_date(&self) -> Option<Date> {
        self.service_date
    }

    #[must_use]
    pub fn stop_time_departure_delays(&self) -> &[Optional<Delay>] {
        &self.stop_time_departure_delays
    }

    #[must_use]
    pub fn stop_time_arrival_delays(&self) -> &[Optional<Delay>] {
        &self.stop_time_arrival_delays
    }

    #[must_use]
    pub fn stop_time_status(&self) -> &[StopTimeStatus] {
        &self.stop_time_status
    }

    #[must_use]
    pub fn trip_status(&self) -> &[TripStatus] {
        &self.trip_status
    }

    #[must_use]
    pub fn trip_delay(&self) -> &[Optional<Delay>] {
        &self.trip_delay
    }

    #[must_use]
    pub fn trip_vehicles(&self) -> &[Option<Vehicle>] {
        &self.trip_vehicles
    }

    #[must_use]
    pub fn added_trips(&self) -> &[AddedTrip] {
        &self.added_trips
    }

    #[must_use]
    pub fn alerts(&self) -> &[Alert] {
        &self.alerts
    }

    /// Returns the alerts that apply at a POSIX time.
    pub fn active_alerts(&self, time: u64) -> impl Iterator<Item = &Alert> {
        self.alerts
            .iter()
            .filter(move |alert| alert.is_active_at(time))
    }

    /// Returns the alerts about a stop that apply at a POSIX time.
    ///
    /// It looks at every alert, which is cheap because a feed has a few hundred at most.
    pub fn alerts_for_stop(&self, stop: StopIndex, time: u64) -> impl Iterator<Item = &Alert> {
        self.active_alerts(time)
            .filter(move |alert| alert.affects_stop(stop))
    }

    /// Returns the alerts about a route that apply at a POSIX time.
    pub fn alerts_for_route(&self, route: RouteIndex, time: u64) -> impl Iterator<Item = &Alert> {
        self.active_alerts(time)
            .filter(move |alert| alert.affects_route(route))
    }

    /// Returns the alerts about a trip that apply at a POSIX time.
    pub fn alerts_for_trip(&self, trip: TripIndex, time: u64) -> impl Iterator<Item = &Alert> {
        self.active_alerts(time)
            .filter(move |alert| alert.affects_trip(trip))
    }

    /// Returns whether the trip was canceled or deleted.
    ///
    /// # Panics
    ///
    /// Panics if `trip` is out of range.
    #[must_use]
    pub fn is_trip_canceled(&self, trip: TripIndex) -> bool {
        matches!(
            self.trip_status[trip.as_usize()],
            TripStatus::Canceled | TripStatus::Deleted
        )
    }

    /// Returns whether the vehicle skips the stop, or the whole trip is canceled.
    ///
    /// `index` is the position of the stop time in the file's timings, which is the trip's
    /// `stop_times` slice start plus the stop's position in the trip's pattern.
    ///
    /// # Panics
    ///
    /// Panics if `trip` or `index` is not from the consumer this was built from.
    #[must_use]
    pub fn is_stop_time_skipped(&self, trip: TripIndex, index: StopTimeIndex) -> bool {
        self.is_trip_canceled(trip)
            || self.stop_time_status[index.as_usize()] == StopTimeStatus::Skipped
    }

    /// Returns the arrival time with the delay applied, or `None` if the stop has no scheduled
    /// arrival or is skipped.
    ///
    /// # Panics
    ///
    /// Panics if `trip` or `index` is not from the consumer this was built from.
    #[must_use]
    pub fn arrival_time(
        &self,
        trip: TripIndex,
        index: StopTimeIndex,
        timing: Timing,
    ) -> Option<Time> {
        let delay = self.stop_time_arrival_delays[index.as_usize()];
        self.delayed(trip, index, timing.arrival, delay)
    }

    /// Returns the departure time with the delay applied, or `None` if the stop has no scheduled
    /// departure or is skipped.
    ///
    /// # Panics
    ///
    /// Panics if `trip` or `index` is not from the consumer this was built from.
    #[must_use]
    pub fn departure_time(
        &self,
        trip: TripIndex,
        index: StopTimeIndex,
        timing: Timing,
    ) -> Option<Time> {
        let delay = self.stop_time_departure_delays[index.as_usize()];
        self.delayed(trip, index, timing.departure, delay)
    }

    fn delayed(
        &self,
        trip: TripIndex,
        index: StopTimeIndex,
        scheduled: Optional<Time>,
        delay: Optional<Delay>,
    ) -> Option<Time> {
        if self.is_stop_time_skipped(trip, index) {
            return None;
        }
        let scheduled = scheduled.as_option()?;
        Some(
            delay
                .as_option()
                .map_or(scheduled, |delay| scheduled + delay),
        )
    }

    /// Forgets everything: trip updates, vehicles, added trips and alerts.
    pub fn clear(&mut self) {
        self.clear_trip_updates();
        self.clear_vehicles();
        self.alerts.clear();
    }

    /// Forgets the trip updates and the added trips, keeping vehicles and alerts.
    pub fn clear_trip_updates(&mut self) {
        self.stop_time_departure_delays
            .fill(Optional::new(Delay::NONE));
        self.stop_time_arrival_delays
            .fill(Optional::new(Delay::NONE));
        self.stop_time_status.fill(StopTimeStatus::Unchanged);
        self.trip_status.fill(TripStatus::Unchanged);
        self.trip_delay.fill(Optional::new(Delay::NONE));
        self.trip_timestamps.fill(0);
        self.added_trips.clear();
    }

    pub fn clear_vehicles(&mut self) {
        self.trip_vehicles.fill(None);
    }

    pub fn clear_alerts(&mut self) {
        self.alerts.clear();
    }

    /// Checks what must hold after every call. It does nothing in release builds.
    ///
    /// # Panics
    ///
    /// Panics if the state doesn't match the consumer, or if an id is stored twice.
    pub(crate) fn check_invariants(&self, consumer: &Consumer<'_>) {
        if !cfg!(debug_assertions) {
            return;
        }
        let stop_times = consumer.timings().len();
        let trips = consumer.trips().len();
        assert!(
            self.stop_time_departure_delays.len() == stop_times
                && self.stop_time_arrival_delays.len() == stop_times
                && self.stop_time_status.len() == stop_times,
            "every stop time has one slot of each kind"
        );
        assert!(
            self.trip_status.len() == trips
                && self.trip_delay.len() == trips
                && self.trip_vehicles.len() == trips
                && self.trip_timestamps.len() == trips,
            "every trip has one slot of each kind"
        );
        let unique = |ids: Vec<&str>| {
            let mut sorted = ids.clone();
            sorted.sort_unstable();
            sorted.dedup();
            sorted.len() == ids.len()
        };
        assert!(
            unique(self.alerts.iter().map(|alert| &*alert.id).collect()),
            "an alert id is stored once, because a later alert replaces it"
        );
        assert!(
            unique(self.added_trips.iter().map(|trip| &*trip.id).collect()),
            "an added trip id is stored once, because a later update replaces it"
        );
    }

    /// Resets one trip to its static schedule.
    fn reset_trip(&mut self, consumer: &Consumer<'_>, trip: TripIndex) {
        let range = consumer.trip(trip).stop_times;
        let stop_times = range.start as usize..(range.start + range.count) as usize;
        self.stop_time_departure_delays[stop_times.clone()].fill(Optional::new(Delay::NONE));
        self.stop_time_arrival_delays[stop_times.clone()].fill(Optional::new(Delay::NONE));
        self.stop_time_status[stop_times].fill(StopTimeStatus::Unchanged);
        self.trip_status[trip.as_usize()] = TripStatus::Unchanged;
        self.trip_delay[trip.as_usize()] = Optional::new(Delay::NONE);
    }
}
