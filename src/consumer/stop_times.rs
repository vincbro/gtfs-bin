use crate::{
    consumer::Consumer,
    models::{Slice, StopTime, TripIdx},
};

impl<'a> Consumer<'a> {
    #[inline]
    pub fn trip_stop_times(&self, idx: TripIdx) -> &'a [StopTime] {
        let slice = self.trip(idx).stop_times;
        &self.stop_times[slice.range()]
    }
}
