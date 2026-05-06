mod builder;

pub use builder::*;
use gtfs_rt::{
    trip_descriptor,
    trip_update::stop_time_update,
    vehicle_position::{OccupancyStatus, VehicleStopStatus},
};

use crate::{
    consumer::Consumer,
    models::{Coordinate, Delay, Opt, Sentinel},
};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TripStatus {
    #[default]
    Unchanged,
    Added,
    Cancled,
    Deleted,
    Unscheduled,
}

impl From<trip_descriptor::ScheduleRelationship> for TripStatus {
    fn from(value: trip_descriptor::ScheduleRelationship) -> Self {
        match value {
            trip_descriptor::ScheduleRelationship::Added => Self::Added,
            trip_descriptor::ScheduleRelationship::Unscheduled => Self::Unscheduled,
            trip_descriptor::ScheduleRelationship::Canceled => Self::Cancled,
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

pub struct Realtime {
    pub stop_time_departure_delays: Vec<Opt<Delay>>,
    pub stop_time_arrival_delays: Vec<Opt<Delay>>,
    pub stop_time_status: Vec<StopTimeStatus>,
    pub trip_status: Vec<TripStatus>,
    pub trip_delay: Vec<Opt<Delay>>,
    pub trip_vehicles: Vec<Option<Vehicle>>,
}

impl Realtime {
    #[must_use]
    pub fn new(consumer: &Consumer) -> Self {
        Self {
            stop_time_departure_delays: vec![Opt::new(Delay::NONE); consumer.stop_times.len()],
            stop_time_arrival_delays: vec![Opt::new(Delay::NONE); consumer.stop_times.len()],
            stop_time_status: vec![StopTimeStatus::Unchanged; consumer.stop_times.len()],
            trip_status: vec![TripStatus::Unchanged; consumer.trips.len()],
            trip_vehicles: vec![None; consumer.trips.len()],
            trip_delay: vec![Opt::new(Delay::NONE); consumer.trips.len()],
        }
    }
}
