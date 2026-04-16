use crate::{consumer::Consumer, rt::Realtime};

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
                if let Some(trip_update) = entity.trip_update {
                    if let Some(trip) = self.consumer.trip_by_id(trip_update.trip.trip_id()) {
                        for stop_time_update in trip_update.stop_time_update.into_iter() {}
                    }
                }

                if let Some(vehicle) = entity.vehicle {}

                if let Some(alert) = entity.alert {}

                if let Some(trip_modifications) = entity.trip_modifications {}
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
