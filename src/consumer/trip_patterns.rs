use crate::{
    consumer::Consumer,
    models::{Slice, Stop, Trip, TripIdx, TripPattern, TripPatternIdx},
};

impl<'a> Consumer<'a> {
    #[inline(always)]
    pub fn trip_pattern(&self, idx: TripPatternIdx) -> &'a TripPattern {
        &self.trip_patterns[idx.to_usize()]
    }

    pub fn trip_pattern_by_trip(&self, trip_idx: TripIdx) -> &'a TripPattern {
        let idx = self.trip_to_trip_pattern[trip_idx.to_usize()];
        self.trip_pattern(idx)
    }

    pub fn stop_sequence_in_trip_pattern(
        &self,
        idx: TripPatternIdx,
    ) -> impl Iterator<Item = &'a Stop> {
        let slice = self.trip_pattern(idx).stops;
        self.trip_patterns_stop_seq[slice.range()]
            .iter()
            .copied()
            .map(|idx| self.stop(idx))
    }

    pub fn trips_in_trip_pattern(&self, idx: TripPatternIdx) -> impl Iterator<Item = &'a Trip> {
        let slice = self.trip_pattern(idx).stops;
        self.trip_patterns_trips[slice.range()]
            .iter()
            .copied()
            .map(|idx| self.trip(idx))
    }
}
