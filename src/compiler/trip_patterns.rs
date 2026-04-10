use std::collections::HashMap;

use crate::models::{
    Sentinel, Slice, Stop, StopIdx, StopSlice, StopTime, Trip, TripIdx, TripPattern,
    TripPatternIdx, TripPatternSlice, TripSlice,
};

pub struct BuildTripPatternsResult(
    pub Vec<TripPattern>,
    pub Vec<StopIdx>,
    pub Vec<TripIdx>,
    pub Vec<TripPatternIdx>,
    pub Vec<TripPatternIdx>,
    pub Vec<TripPatternSlice>,
);

pub fn build_trip_patterns(
    trips: &[Trip],
    stops: &[Stop],
    stop_times: &[StopTime],
) -> BuildTripPatternsResult {
    let mut stop_seq_to_trips: HashMap<Vec<StopIdx>, Vec<TripIdx>> =
        HashMap::with_capacity(trips.len());
    for trip in trips.iter() {
        let stop_times = &stop_times[trip.stop_times.range()];
        let stop_seq = stop_times.iter().map(|st| st.stop_idx).collect();
        stop_seq_to_trips
            .entry(stop_seq)
            .or_default()
            .push(trip.idx);
    }

    let mut trip_patterns: Vec<TripPattern> = Vec::with_capacity(stop_seq_to_trips.len());
    let mut stop_sequences: Vec<StopIdx> = Vec::new();
    let mut trips_in_sequences: Vec<TripIdx> = Vec::new();
    let mut trip_to_trip_pattern: Vec<TripPatternIdx> = vec![TripPatternIdx::NONE; trips.len()];
    let mut stop_to_trip_pattern_map: HashMap<StopIdx, Vec<TripPatternIdx>> = HashMap::new();

    for (i, (stop_seq, trips_in_seq)) in stop_seq_to_trips.iter().enumerate() {
        let idx = TripPatternIdx(i as u32);
        trips_in_seq.iter().for_each(|trip_idx| {
            trip_to_trip_pattern[trip_idx.as_usize()] = idx;
        });
        trip_patterns.push(TripPattern {
            stops: StopSlice {
                start: stop_sequences.len() as u32,
                count: stop_seq.len() as u32,
            },
            trips: TripSlice {
                start: trips_in_sequences.len() as u32,
                count: trips_in_seq.len() as u32,
            },
            idx,
        });

        stop_seq.iter().copied().for_each(|stop_idx| {
            stop_to_trip_pattern_map
                .entry(stop_idx)
                .or_default()
                .push(idx)
        });

        stop_sequences.extend_from_slice(stop_seq);
        trips_in_sequences.extend_from_slice(trips_in_seq);
    }

    let mut stop_to_trip_pattern_lookup = vec![TripPatternSlice::NONE; stops.len()];
    let mut stop_to_trip_pattern: Vec<TripPatternIdx> = Vec::new();

    for (stop_idx, trip_patterns) in stop_to_trip_pattern_map.into_iter() {
        let slice = TripPatternSlice {
            start: stop_to_trip_pattern.len() as u32,
            count: trip_patterns.len() as u32,
        };

        stop_to_trip_pattern.extend_from_slice(&trip_patterns);
        stop_to_trip_pattern_lookup[stop_idx.as_usize()] = slice;
    }

    BuildTripPatternsResult(
        trip_patterns,
        stop_sequences,
        trips_in_sequences,
        trip_to_trip_pattern,
        stop_to_trip_pattern,
        stop_to_trip_pattern_lookup,
    )
}
