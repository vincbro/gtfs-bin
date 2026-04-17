use gtfs_rt::trip_descriptor::ScheduleRelationship;

use crate::{
    consumer::Consumer,
    models::{Coordinate, Delay, Opt},
    rt::{Realtime, TripStatus, Vehicle},
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
                        ScheduleRelationship::Scheduled => {
                            realtime.trip_status[trip.idx.as_usize()] = TripStatus::Unchanged
                        }
                        ScheduleRelationship::Added => {
                            realtime.trip_status[trip.idx.as_usize()] = TripStatus::Added
                        }
                        ScheduleRelationship::Unscheduled => {
                            realtime.trip_status[trip.idx.as_usize()] = TripStatus::Unscheduled
                        }
                        ScheduleRelationship::Canceled => {
                            realtime.trip_status[trip.idx.as_usize()] = TripStatus::Cancled
                        }
                        ScheduleRelationship::Deleted => {
                            realtime.trip_status[trip.idx.as_usize()] = TripStatus::Deleted
                        }
                        _ => (),
                    }
                    let stop_times = self.consumer.stop_times_by_trip(trip.idx);
                    for stop_time_update in trip_update.stop_time_update.into_iter() {
                        if let Ok(idx) = stop_times.binary_search_by_key(
                            &stop_time_update.stop_sequence.unwrap_or(u32::MAX),
                            |st| st.sequence,
                        ) {
                            let idx = trip.stop_times.start as usize + idx;
                            if let Some(departure) = stop_time_update.departure {
                                realtime.stop_time_departure_delays[idx] =
                                    Opt::new(Delay(departure.delay() as i16))
                            }

                            if let Some(arrival) = stop_time_update.arrival {
                                realtime.stop_time_arrival_delays[idx] =
                                    Opt::new(Delay(arrival.delay() as i16))
                            }
                        }
                    }
                }

                if let Some(vehicle) = entity.vehicle
                    && let Some(position) = vehicle.position
                    && let Some(trip_descriptor) = vehicle.trip
                    && let Some(trip) = self.consumer.trip_by_id(trip_descriptor.trip_id())
                {
                    realtime.trip_vehicles[trip.idx.as_usize()] = Some(Vehicle {
                        position: Coordinate::new(
                            position.latitude as f64,
                            position.longitude as f64,
                        ),
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
