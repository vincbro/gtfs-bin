use crate::{
    consumer::Consumer,
    models::{PatternStop, Slice, Stop, StopIndex, Trip, TripIndex, TripPattern, TripPatternIndex},
};

impl<'a> Consumer<'a> {
    /// # Panics
    ///
    /// Panics if `index` is out of range, which includes `TripPatternIndex::NONE`.
    #[must_use]
    pub fn trip_pattern(&self, index: TripPatternIndex) -> &'a TripPattern {
        &self.trip_patterns[index.as_usize()]
    }

    /// # Panics
    ///
    /// Panics if `trip_index` is out of range.
    #[must_use]
    pub fn trip_pattern_by_trip(&self, trip_index: TripIndex) -> &'a TripPattern {
        let index = self.trip_to_trip_pattern[trip_index.as_usize()];
        self.trip_pattern(index)
    }

    /// # Panics
    ///
    /// Panics if `stop_index` is out of range.
    #[must_use]
    pub fn trip_patterns_by_stop(&self, stop_index: StopIndex) -> &'a [TripPatternIndex] {
        let slice = self.stop_to_trip_pattern_lookup[stop_index.as_usize()];
        &self.stop_to_trip_pattern[slice.range()]
    }

    /// # Panics
    ///
    /// Panics if `stop_index` is out of range.
    pub fn iter_trip_patterns_by_stop(
        &self,
        stop_index: StopIndex,
    ) -> impl Iterator<Item = &'a TripPattern> {
        self.trip_patterns_by_stop(stop_index)
            .iter()
            .map(|&pattern_index| self.trip_pattern(pattern_index))
    }

    /// # Panics
    ///
    /// Panics if `index` is out of range.
    #[must_use]
    pub fn stop_sequence_by_trip_pattern(&self, index: TripPatternIndex) -> &'a [StopIndex] {
        let slice = self.trip_pattern(index).stops;
        &self.trip_patterns_stop_seq[slice.range()]
    }

    /// Returns what the pattern says at each of its stops, at the positions of
    /// `stop_sequence_by_trip_pattern`.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range.
    #[must_use]
    pub fn pattern_stops_by_trip_pattern(&self, index: TripPatternIndex) -> &'a [PatternStop] {
        let slice = self.trip_pattern(index).stops;
        &self.pattern_stops[slice.range()]
    }

    /// # Panics
    ///
    /// Panics if `index` is out of range.
    pub fn iter_stop_sequence_by_trip_pattern(
        &self,
        index: TripPatternIndex,
    ) -> impl Iterator<Item = &'a Stop> {
        self.stop_sequence_by_trip_pattern(index)
            .iter()
            .map(|&stop_index| self.stop(stop_index))
    }

    /// # Panics
    ///
    /// Panics if `index` is out of range.
    #[must_use]
    pub fn trips_in_trip_pattern(&self, index: TripPatternIndex) -> &'a [TripIndex] {
        let slice = self.trip_pattern(index).trips;
        &self.trip_patterns_trips[slice.range()]
    }

    /// # Panics
    ///
    /// Panics if `index` is out of range.
    pub fn iter_trips_in_trip_pattern(
        &self,
        index: TripPatternIndex,
    ) -> impl Iterator<Item = &'a Trip> {
        self.trips_in_trip_pattern(index)
            .iter()
            .map(|&trip_index| self.trip(trip_index))
    }
}
