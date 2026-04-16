mod builder;

pub use builder::*;

use crate::{
    consumer::Consumer,
    models::{Delay, Opt, Sentinel},
};

pub struct Realtime {
    stop_time_delays: Vec<Opt<Delay>>,
    active_trips: Vec<bool>,
}

impl Realtime {
    pub fn new(consumer: &Consumer) -> Self {
        Self {
            stop_time_delays: vec![Opt::new(Delay::NONE); consumer.stop_times.len()],
            active_trips: vec![true; consumer.trips.len()],
        }
    }
}
