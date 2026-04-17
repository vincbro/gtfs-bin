mod builder;

pub use builder::*;

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

#[derive(Default, Debug, Clone, Copy)]
pub struct Vehicle {
    pub position: Coordinate,
}

pub struct Realtime {
    pub stop_time_departure_delays: Vec<Opt<Delay>>,
    pub stop_time_arrival_delays: Vec<Opt<Delay>>,
    pub trip_status: Vec<TripStatus>,
    pub trip_vehicles: Vec<Option<Vehicle>>,
}

impl Realtime {
    pub fn new(consumer: &Consumer) -> Self {
        Self {
            stop_time_departure_delays: vec![Opt::new(Delay::NONE); consumer.stop_times.len()],
            stop_time_arrival_delays: vec![Opt::new(Delay::NONE); consumer.stop_times.len()],
            trip_status: vec![TripStatus::Unchanged; consumer.trips.len()],
            trip_vehicles: vec![None; consumer.trips.len()],
        }
    }
}
