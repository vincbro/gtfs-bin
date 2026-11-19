use gtfs_rt::{
    trip_descriptor,
    trip_update::stop_time_update,
    vehicle_position::{OccupancyStatus, VehicleStopStatus},
};

use crate::{
    consumer::Consumer,
    models::{Coordinate, Delay, Optional, Sentinel},
};

mod builder;

pub use builder::RealtimeBuilder;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TripStatus {
    #[default]
    Unchanged,
    Added,
    Canceled,
    Deleted,
    Unscheduled,
}

impl From<trip_descriptor::ScheduleRelationship> for TripStatus {
    fn from(value: trip_descriptor::ScheduleRelationship) -> Self {
        match value {
            trip_descriptor::ScheduleRelationship::Added => Self::Added,
            trip_descriptor::ScheduleRelationship::Unscheduled => Self::Unscheduled,
            trip_descriptor::ScheduleRelationship::Canceled => Self::Canceled,
            trip_descriptor::ScheduleRelationship::Deleted => Self::Deleted,
            _ => Self::Unchanged,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopTimeStatus {
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
}

/// The live state of every stop time and trip of one `Consumer`, indexed the same way.
///
/// After every public call:
/// - The `stop_time_*` tables have one entry per stop time of the consumer it was built from.
/// - The `trip_*` tables have one entry per trip of that consumer.
///
/// Nothing resizes the tables, so a `Realtime` can only be applied with the consumer it came from.
#[derive(Debug)]
pub struct Realtime {
    stop_time_departure_delays: Box<[Optional<Delay>]>,
    stop_time_arrival_delays: Box<[Optional<Delay>]>,
    stop_time_status: Box<[StopTimeStatus]>,
    trip_status: Box<[TripStatus]>,
    trip_delay: Box<[Optional<Delay>]>,
    trip_vehicles: Box<[Option<Vehicle>]>,
}

impl Realtime {
    /// Creates a state with nothing updated, sized for `consumer`.
    #[must_use]
    pub fn new(consumer: &Consumer) -> Self {
        let stop_time_count = consumer.stop_times().len();
        let trip_count = consumer.trips().len();
        Self {
            stop_time_departure_delays: vec![Optional::new(Delay::NONE); stop_time_count]
                .into_boxed_slice(),
            stop_time_arrival_delays: vec![Optional::new(Delay::NONE); stop_time_count]
                .into_boxed_slice(),
            stop_time_status: vec![StopTimeStatus::Unchanged; stop_time_count].into_boxed_slice(),
            trip_status: vec![TripStatus::Unchanged; trip_count].into_boxed_slice(),
            trip_vehicles: vec![None; trip_count].into_boxed_slice(),
            trip_delay: vec![Optional::new(Delay::NONE); trip_count].into_boxed_slice(),
        }
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
}
