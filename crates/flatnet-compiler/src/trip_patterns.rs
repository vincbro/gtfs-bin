use std::collections::BTreeMap;

use flatnet_core::models::{
    Sentinel, Slice, Stop, StopIndex, StopSlice, StopTime, Trip, TripIndex, TripPattern,
    TripPatternIndex, TripPatternSlice, TripSlice,
};

pub(super) struct TripPatterns {
    pub patterns: Vec<TripPattern>,
    /// The stops of every pattern, back to back.
    pub stop_sequences: Vec<StopIndex>,
    /// The trips of every pattern, back to back.
    pub trips_in_patterns: Vec<TripIndex>,
    /// By trip: the pattern it follows.
    pub trip_to_pattern: Vec<TripPatternIndex>,
    /// The patterns that visit each stop, back to back.
    pub stop_to_pattern: Vec<TripPatternIndex>,
    /// By stop: the run of `stop_to_pattern` that visits it.
    pub stop_to_pattern_lookup: Vec<TripPatternSlice>,
}

pub(super) fn build_trip_patterns(
    trips: &[Trip],
    stops: &[Stop],
    stop_times: &[StopTime],
) -> TripPatterns {
    // Ordered maps so the same input always gives the same bytes.
    let mut trips_by_stop_sequence: BTreeMap<Vec<StopIndex>, Vec<TripIndex>> = BTreeMap::new();
    for trip in trips {
        let stop_sequence = stop_times[trip.stop_times.range()]
            .iter()
            .map(|stop_time| stop_time.stop_index)
            .collect();
        trips_by_stop_sequence
            .entry(stop_sequence)
            .or_default()
            .push(trip.index);
    }

    let mut patterns: Vec<TripPattern> = Vec::with_capacity(trips_by_stop_sequence.len());
    let mut stop_sequences: Vec<StopIndex> = Vec::with_capacity(stop_times.len());
    let mut trips_in_patterns: Vec<TripIndex> = Vec::with_capacity(trips.len());
    let mut trip_to_pattern = vec![TripPatternIndex::NONE; trips.len()];
    let mut patterns_by_stop: BTreeMap<StopIndex, Vec<TripPatternIndex>> = BTreeMap::new();

    for (position, (stop_sequence, pattern_trips)) in trips_by_stop_sequence.iter().enumerate() {
        let index = TripPatternIndex::from(position);
        for trip_index in pattern_trips {
            trip_to_pattern[trip_index.as_usize()] = index;
        }
        patterns.push(TripPattern {
            stops: StopSlice::from_usize(stop_sequences.len(), stop_sequence.len()),
            trips: TripSlice::from_usize(trips_in_patterns.len(), pattern_trips.len()),
            index,
        });

        for &stop_index in stop_sequence {
            patterns_by_stop.entry(stop_index).or_default().push(index);
        }

        stop_sequences.extend_from_slice(stop_sequence);
        trips_in_patterns.extend_from_slice(pattern_trips);
    }

    let mut stop_to_pattern_lookup = vec![TripPatternSlice::NONE; stops.len()];
    let mut stop_to_pattern: Vec<TripPatternIndex> = Vec::with_capacity(stop_sequences.len());

    for (stop_index, stop_patterns) in patterns_by_stop {
        stop_to_pattern_lookup[stop_index.as_usize()] =
            TripPatternSlice::from_usize(stop_to_pattern.len(), stop_patterns.len());
        stop_to_pattern.extend_from_slice(&stop_patterns);
    }

    TripPatterns {
        patterns,
        stop_sequences,
        trips_in_patterns,
        trip_to_pattern,
        stop_to_pattern,
        stop_to_pattern_lookup,
    }
}
