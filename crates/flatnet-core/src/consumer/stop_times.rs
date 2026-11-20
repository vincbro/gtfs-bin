use crate::{
    consumer::Consumer,
    models::{Slice, StopTime, TripIndex},
};

impl<'a> Consumer<'a> {
    /// # Panics
    ///
    /// Panics if `index` is out of range, which includes `TripIndex::NONE`.
    #[must_use]
    pub fn stop_times_by_trip(&self, index: TripIndex) -> &'a [StopTime] {
        let slice = self.trip(index).stop_times;
        &self.stop_times[slice.range()]
    }
}
