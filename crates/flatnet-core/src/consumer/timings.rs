use crate::{
    consumer::Consumer,
    models::{Slice, Timing, TripIndex},
};

impl<'a> Consumer<'a> {
    /// Returns when the trip is at each stop of its pattern, in the order of the pattern's stops.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range, which includes `TripIndex::NONE`.
    #[must_use]
    pub fn timings_by_trip(&self, index: TripIndex) -> &'a [Timing] {
        let slice = self.trip(index).stop_times;
        &self.timings[slice.range()]
    }
}
