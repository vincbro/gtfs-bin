use crate::{
    consumer::Consumer,
    models::{Slice, Stop, StopIdx, Trip, TripIdx, TripPattern, TripPatternIdx},
};

impl<'a> Consumer<'a> {
    #[inline]
    #[must_use]
    pub fn trip_pattern(&self, idx: TripPatternIdx) -> &'a TripPattern {
        &self.trip_patterns[idx.as_usize()]
    }

    #[inline]
    #[must_use]
    pub fn trip_pattern_by_trip(&self, trip_idx: TripIdx) -> &'a TripPattern {
        let idx = self.trip_to_trip_pattern[trip_idx.as_usize()];
        self.trip_pattern(idx)
    }

    #[inline]
    #[must_use]
    pub fn trip_patterns_by_stop(&self, stop_idx: StopIdx) -> &[TripPatternIdx] {
        let slice = self.stop_to_trip_pattern_lookup[stop_idx.as_usize()];
        &self.stop_to_trip_pattern[slice.range()]
    }

    pub fn iter_trip_patterns_by_stop(
        &self,
        stop_idx: StopIdx,
    ) -> impl Iterator<Item = &'a TripPattern> {
        self.trip_patterns_by_stop(stop_idx)
            .iter()
            .copied()
            .map(|trip_pattern_idx| self.trip_pattern(trip_pattern_idx))
    }

    #[inline]
    #[must_use]
    pub fn stop_sequence_by_trip_pattern(&self, idx: TripPatternIdx) -> &'a [StopIdx] {
        let slice = self.trip_pattern(idx).stops;
        &self.trip_patterns_stop_seq[slice.range()]
    }

    pub fn iter_stop_sequence_by_trip_pattern(
        &self,
        idx: TripPatternIdx,
    ) -> impl Iterator<Item = &'a Stop> {
        self.stop_sequence_by_trip_pattern(idx)
            .iter()
            .copied()
            .map(|idx| self.stop(idx))
    }

    #[inline]
    #[must_use]
    pub fn trips_in_trip_pattern(&self, idx: TripPatternIdx) -> &[TripIdx] {
        let slice = self.trip_pattern(idx).trips;
        &self.trip_patterns_trips[slice.range()]
    }

    pub fn iter_trips_in_trip_pattern(
        &self,
        idx: TripPatternIdx,
    ) -> impl Iterator<Item = &'a Trip> {
        self.trips_in_trip_pattern(idx)
            .iter()
            .copied()
            .map(|idx| self.trip(idx))
    }
}
