use crate::{
    consumer::Consumer,
    models::{Coordinate, Delay, Opt, Sentinel, Time},
    rt::{Realtime, Vehicle},
};

pub struct RealtimeBuilder<'a> {
    consumer: &'a Consumer<'a>,
    cascade_delay: bool,
}

impl<'a> RealtimeBuilder<'a> {
    #[must_use]
    pub const fn new(consumer: &'a Consumer) -> Self {
        Self {
            consumer,
            cascade_delay: true,
        }
    }

    #[must_use]
    pub const fn with_cascading_delays(mut self, value: bool) -> Self {
        self.cascade_delay = value;
        self
    }

    pub fn apply<I>(self, messages: I, realtime: &mut Realtime)
    where
        I: Iterator<Item = gtfs_rt::FeedMessage>,
    {
        for message in messages {
            for entity in message.entity {
                if let Some(trip_update) = entity.trip_update
                    && let Some(trip) = self.consumer.trip_by_id(trip_update.trip.trip_id())
                {
                    realtime.trip_status[trip.idx.as_usize()] =
                        trip_update.trip.schedule_relationship().into();

                    realtime.trip_delay[trip.idx.as_usize()] =
                        Opt::new(Delay::from(trip_update.delay()));

                    let stop_times = self.consumer.stop_times_by_trip(trip.idx);
                    for stop_time_update in trip_update.stop_time_update {
                        if let Ok(idx) = stop_times.binary_search_by_key(
                            &stop_time_update.stop_sequence.unwrap_or(u32::MAX),
                            |st| st.sequence,
                        ) {
                            let idx = trip.stop_times.start as usize + idx;
                            let stop_time = self.consumer.stop_times[idx];
                            realtime.stop_time_status[idx] =
                                stop_time_update.schedule_relationship().into();

                            let departure_delay = stop_time_update.departure.map_or_else(
                                || Opt::new(Delay::NONE),
                                |departure| {
                                    departure.delay.map_or_else(
                                        || {
                                            if let Some(new_time) = departure.time
                                                && let Some(time) =
                                                    stop_time.departure_time.as_option()
                                            {
                                                let new_time = Time::from(new_time % 86_400);
                                                let delay =
                                                    i64::from(new_time.0) - i64::from(time.0);
                                                Opt::new(Delay::from(delay))
                                            } else {
                                                Opt::new(Delay::NONE)
                                            }
                                        },
                                        |delay| Opt::new(Delay::from(delay)),
                                    )
                                },
                            );
                            realtime.stop_time_departure_delays[idx] = departure_delay;

                            let arrival_delay = stop_time_update.arrival.map_or_else(
                                || Opt::new(Delay::NONE),
                                |arrival| {
                                    arrival.delay.map_or_else(
                                        || {
                                            if let Some(new_time) = arrival.time
                                                && let Some(time) =
                                                    stop_time.arrival_time.as_option()
                                            {
                                                let new_time = Time::from(new_time % 86_400);
                                                let delay =
                                                    i64::from(new_time.0) - i64::from(time.0);
                                                Opt::new(Delay::from(delay))
                                            } else {
                                                Opt::new(Delay::NONE)
                                            }
                                        },
                                        |delay| Opt::new(Delay::from(delay)),
                                    )
                                },
                            );
                            realtime.stop_time_arrival_delays[idx] = arrival_delay;
                        }
                    }
                }

                if let Some(vehicle) = entity.vehicle
                    && let Some(position) = vehicle.position.clone()
                    && let Some(trip_descriptor) = vehicle.trip.clone()
                    && let Some(trip) = self.consumer.trip_by_id(trip_descriptor.trip_id())
                {
                    realtime.trip_vehicles[trip.idx.as_usize()] = Some(Vehicle {
                        occupancy_status: vehicle.occupancy_status(),
                        occupancy_percentage: vehicle.occupancy_percentage,
                        position: Coordinate::new(
                            f64::from(position.latitude),
                            f64::from(position.longitude),
                        ),
                        current_stop_sequence: vehicle.current_stop_sequence,
                        current_status: vehicle.current_status(),
                    });
                }

                // if let Some(trip_modifications) = entity.trip_modifications {}
            }
        }
    }

    pub fn build<I>(self, messages: I) -> Realtime
    where
        I: Iterator<Item = gtfs_rt::FeedMessage>,
    {
        let mut realtime = Realtime::new(self.consumer);
        self.apply(messages, &mut realtime);
        realtime
    }
}
