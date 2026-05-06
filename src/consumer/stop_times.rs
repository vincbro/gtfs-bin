use crate::{
    consumer::Consumer,
    models::{Slice, StopTime, TripIdx},
};

impl<'a> Consumer<'a> {
    #[inline]
    #[must_use]
    pub fn stop_times_by_trip(&self, idx: TripIdx) -> &'a [StopTime] {
        let slice = self.trip(idx).stop_times;
        &self.stop_times[slice.range()]
    }
}
