use crate::{
    consumer::Consumer,
    models::{Slice, Stop, StopIdx, Trip, TripIdx, TripPattern, TripPatternIdx},
};

impl<'a> Consumer<'a> {
    #[inline(always)]
    pub fn trip_pattern(&self, idx: TripPatternIdx) -> &'a TripPattern {
        &self.trip_patterns[idx.as_usize()]
    }

    pub fn trip_pattern_by_trip(&self, trip_idx: TripIdx) -> &'a TripPattern {
        let idx = self.trip_to_trip_pattern[trip_idx.as_usize()];
        self.trip_pattern(idx)
    }

    pub fn iter_trip_patterns_by_stop(
        &self,
        stop_idx: StopIdx,
    ) -> impl Iterator<Item = &'a TripPattern> {
        let slice = self.stop_to_trip_pattern_lookup[stop_idx.as_usize()];
        self.stop_to_trip_pattern[slice.range()]
            .iter()
            .copied()
            .map(|trip_pattern_idx| self.trip_pattern(trip_pattern_idx))
    }

    pub fn iter_stop_sequence_in_trip_pattern(
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
        let slice = self.trip_pattern(idx).trips;
        self.trip_patterns_trips[slice.range()]
            .iter()
            .copied()
            .map(|idx| self.trip(idx))
    }
}
