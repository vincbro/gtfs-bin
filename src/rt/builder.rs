use gtfs_rt::{
    FeedMessage, TripUpdate, VehiclePosition,
    trip_update::{StopTimeEvent, StopTimeUpdate},
};

use crate::{
    consumer::Consumer,
    models::{Coordinate, Delay, Optional, Sentinel, StopTime, Time},
    rt::{Realtime, StopTimeStatus, Vehicle},
};

const SECONDS_PER_DAY: i64 = 86_400;

/// Applies GTFS-realtime feeds to a `Realtime`.
pub struct RealtimeBuilder<'a> {
    consumer: &'a Consumer<'a>,
    cascade_delays: bool,
    utc_offset_seconds: i32,
}

impl<'a> RealtimeBuilder<'a> {
    #[must_use]
    pub const fn new(consumer: &'a Consumer) -> Self {
        Self {
            consumer,
            cascade_delays: true,
            utc_offset_seconds: 0,
        }
    }

    /// Sets whether a stop time without an update of its own takes the delay of the nearest
    /// updated stop time before it. This is on by default.
    #[must_use]
    pub const fn with_cascading_delays(mut self, cascade_delays: bool) -> Self {
        self.cascade_delays = cascade_delays;
        self
    }

    /// Sets the agency's offset from UTC, in seconds.
    ///
    /// Stop times are local times of day, but feeds give absolute timestamps. A delay computed from
    /// a timestamp needs the offset to compare them. It is 0 (UTC) by default.
    #[must_use]
    pub const fn with_utc_offset_seconds(mut self, utc_offset_seconds: i32) -> Self {
        self.utc_offset_seconds = utc_offset_seconds;
        self
    }

    /// Applies every message to `realtime`. Later updates replace earlier ones.
    ///
    /// # Panics
    ///
    /// Panics if `realtime` was not built from the consumer this builder was created with.
    pub fn apply<I>(self, messages: I, realtime: &mut Realtime)
    where
        I: Iterator<Item = FeedMessage>,
    {
        assert!(
            realtime.stop_time_status.len() == self.consumer.stop_times().len()
                && realtime.trip_status.len() == self.consumer.trips().len(),
            "realtime must be built from the consumer it is applied with"
        );
        for message in messages {
            self.apply_message(message, realtime);
        }
    }

    pub fn build<I>(self, messages: I) -> Realtime
    where
        I: Iterator<Item = FeedMessage>,
    {
        let mut realtime = Realtime::new(self.consumer);
        self.apply(messages, &mut realtime);
        realtime
    }

    fn apply_message(&self, message: FeedMessage, realtime: &mut Realtime) {
        for entity in message.entity {
            if let Some(trip_update) = entity.trip_update {
                self.apply_trip_update(&trip_update, realtime);
            }
            if let Some(vehicle) = entity.vehicle {
                self.apply_vehicle(vehicle, realtime);
            }
        }
    }

    fn apply_trip_update(&self, trip_update: &TripUpdate, realtime: &mut Realtime) {
        let Some(trip) = self.consumer.trip_by_id(trip_update.trip.trip_id()) else {
            return;
        };
        let trip_slot = trip.index.as_usize();
        realtime.trip_status[trip_slot] = trip_update.trip.schedule_relationship().into();
        realtime.trip_delay[trip_slot] = Optional::new(Delay::from(trip_update.delay()));

        let stop_times = self.consumer.stop_times_by_trip(trip.index);
        let first_slot = trip.stop_times.start as usize;
        for update in &trip_update.stop_time_update {
            let sequence = update.stop_sequence.unwrap_or(u32::MAX);
            let Ok(offset) =
                stop_times.binary_search_by_key(&sequence, |stop_time| stop_time.sequence)
            else {
                continue;
            };
            self.apply_stop_time_update(update, first_slot + offset, stop_times[offset], realtime);
        }

        if self.cascade_delays {
            cascade_delays(first_slot..first_slot + stop_times.len(), realtime);
        }
    }

    fn apply_stop_time_update(
        &self,
        update: &StopTimeUpdate,
        slot: usize,
        stop_time: StopTime,
        realtime: &mut Realtime,
    ) {
        realtime.stop_time_status[slot] = update.schedule_relationship().into();
        realtime.stop_time_departure_delays[slot] =
            self.event_delay(update.departure.as_ref(), stop_time.departure_time);
        realtime.stop_time_arrival_delays[slot] =
            self.event_delay(update.arrival.as_ref(), stop_time.arrival_time);
    }

    /// Returns the delay an arrival or departure event reports, from its `delay` or else from how
    /// far its `time` is from the scheduled time.
    fn event_delay(
        &self,
        event: Option<&StopTimeEvent>,
        scheduled: Optional<Time>,
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

        let local_time_of_day =
            (actual + i64::from(self.utc_offset_seconds)).rem_euclid(SECONDS_PER_DAY);
        let difference = local_time_of_day - i64::from(scheduled.0);
        // Both sides are times of day, so a trip just either side of midnight wraps to the nearest
        // day instead of being a day off.
        let half_day = SECONDS_PER_DAY / 2;
        Optional::new(Delay::from(
            (difference + half_day).rem_euclid(SECONDS_PER_DAY) - half_day,
        ))
    }

    fn apply_vehicle(&self, vehicle: VehiclePosition, realtime: &mut Realtime) {
        let occupancy_status = vehicle.occupancy_status();
        let current_status = vehicle.current_status();
        let (Some(position), Some(descriptor)) = (vehicle.position, vehicle.trip) else {
            return;
        };
        let Some(trip) = self.consumer.trip_by_id(descriptor.trip_id()) else {
            return;
        };

        realtime.trip_vehicles[trip.index.as_usize()] = Some(Vehicle {
            occupancy_status,
            occupancy_percentage: vehicle.occupancy_percentage,
            position: Coordinate::new(f64::from(position.latitude), f64::from(position.longitude)),
            current_stop_sequence: vehicle.current_stop_sequence,
            current_status,
        });
    }
}

/// Gives each stop time without an update the delay of the nearest updated one before it.
fn cascade_delays(slots: std::ops::Range<usize>, realtime: &mut Realtime) {
    let mut arrival = Optional::new(Delay::NONE);
    let mut departure = Optional::new(Delay::NONE);
    for slot in slots {
        let has_update = realtime.stop_time_status[slot] != StopTimeStatus::Unchanged;
        carry_delay(
            &mut realtime.stop_time_arrival_delays[slot],
            &mut arrival,
            has_update,
        );
        carry_delay(
            &mut realtime.stop_time_departure_delays[slot],
            &mut departure,
            has_update,
        );
    }
}

/// An updated stop time passes its delay on, if it has one. A stop time without an update takes
/// the delay that was passed on.
fn carry_delay(delay: &mut Optional<Delay>, carried: &mut Optional<Delay>, has_update: bool) {
    if !has_update {
        *delay = *carried;
    } else if delay.is_some() {
        *carried = *delay;
    }
}
