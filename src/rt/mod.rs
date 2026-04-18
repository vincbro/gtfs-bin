mod builder;

pub use builder::*;
use gtfs_rt::vehicle_position::{OccupancyStatus, VehicleStopStatus};

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

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopTimeStatus {
    #[default]
    Unchanged,
    Scheduled,
    Skipped,
    NoData,
    Unscheduled,
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
