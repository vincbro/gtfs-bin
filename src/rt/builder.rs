use gtfs_rt::{trip_descriptor, trip_update::stop_time_update};

use crate::{
    consumer::Consumer,
    models::{Coordinate, Delay, Opt, Sentinel, Time},
    rt::{Realtime, StopTimeStatus, TripStatus, Vehicle},
};

pub struct RealtimeBuilder<'a> {
    consumer: &'a Consumer<'a>,
    cascade_delay: bool,
}

impl<'a> RealtimeBuilder<'a> {
    pub fn new(consumer: &'a Consumer) -> Self {
        Self {
            consumer,
            cascade_delay: true,
        }
    }

    pub fn with_cascading_delays(mut self, value: bool) -> Self {
        self.cascade_delay = value;
        self
    }

    pub fn apply<I>(self, messages: I, realtime: &mut Realtime)
    where
        I: Iterator<Item = gtfs_rt::FeedMessage>,
    {
        for message in messages {
            for entity in message.entity.into_iter() {
                if let Some(trip_update) = entity.trip_update
                    && let Some(trip) = self.consumer.trip_by_id(trip_update.trip.trip_id())
                {
                    match trip_update.trip.schedule_relationship() {
                        trip_descriptor::ScheduleRelationship::Scheduled => {
                            realtime.trip_status[trip.idx.as_usize()] = TripStatus::Unchanged
                        }
                        trip_descriptor::ScheduleRelationship::Added => {
                            realtime.trip_status[trip.idx.as_usize()] = TripStatus::Added
                        }
                        trip_descriptor::ScheduleRelationship::Unscheduled => {
                            realtime.trip_status[trip.idx.as_usize()] = TripStatus::Unscheduled
                        }
                        trip_descriptor::ScheduleRelationship::Canceled => {
                            realtime.trip_status[trip.idx.as_usize()] = TripStatus::Cancled
                        }
                        trip_descriptor::ScheduleRelationship::Deleted => {
                            realtime.trip_status[trip.idx.as_usize()] = TripStatus::Deleted
                        }
                        _ => (),
                    }
                    realtime.trip_delay[trip.idx.as_usize()] =
                        Opt::new(Delay(trip_update.delay() as i16));

                    let stop_times = self.consumer.stop_times_by_trip(trip.idx);
                    for stop_time_update in trip_update.stop_time_update.into_iter() {
                        if let Ok(idx) = stop_times.binary_search_by_key(
                            &stop_time_update.stop_sequence.unwrap_or(u32::MAX),
                            |st| st.sequence,
                        ) {
                            let idx = trip.stop_times.start as usize + idx;
                            let stop_time = self.consumer.stop_times[idx];
                            match stop_time_update.schedule_relationship() {
                                stop_time_update::ScheduleRelationship::Scheduled => {
                                    realtime.stop_time_status[idx] = StopTimeStatus::Scheduled
                                }
                                stop_time_update::ScheduleRelationship::Skipped => {
                                    realtime.stop_time_status[idx] = StopTimeStatus::Skipped
                                }
                                stop_time_update::ScheduleRelationship::NoData => {
                                    realtime.stop_time_status[idx] = StopTimeStatus::NoData
                                }
                                stop_time_update::ScheduleRelationship::Unscheduled => {
                                    realtime.stop_time_status[idx] = StopTimeStatus::Unscheduled
                                }
                            }

                            let departure_delay =
                                if let Some(departure) = stop_time_update.departure {
                                    if let Some(delay) = departure.delay {
                                        Opt::new(Delay(delay as i16))
                                    } else if let Some(new_time) = departure.time
                                        && let Some(time) = stop_time.departure_time.get()
                                    {
                                        let new_time = Time((new_time % 86_400) as u32);
                                        let delay = new_time.0 as i64 - time.0 as i64;
                                        Opt::new(Delay(delay as i16))
                                    } else {
                                        Opt::new(Delay::NONE)
                                    }
                                } else {
                                    Opt::new(Delay::NONE)
                                };
                            realtime.stop_time_departure_delays[idx] = departure_delay;

                            let arrival_delay = if let Some(arrival) = stop_time_update.arrival {
                                if let Some(delay) = arrival.delay {
                                    Opt::new(Delay(delay as i16))
                                } else if let Some(new_time) = arrival.time
                                    && let Some(time) = stop_time.arrival_time.get()
                                {
                                    let new_time = Time((new_time % 86_400) as u32);
                                    let delay = new_time.0 as i64 - time.0 as i64;
                                    Opt::new(Delay(delay as i16))
                                } else {
                                    Opt::new(Delay::NONE)
                                }
                            } else {
                                Opt::new(Delay::NONE)
                            };
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
                            position.latitude as f64,
                            position.longitude as f64,
                        ),
                        current_stop_sequence: vehicle.current_stop_sequence,
                        current_status: vehicle.current_status(),
                    })
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
