use std::collections::HashMap;

use crate::models::{
    Sentinel, Slice, StopIdx, StopSlice, StopTime, Trip, TripIdx, TripPattern, TripPatternIdx,
    TripSlice,
};

pub fn build_trip_patterns(
    trips: &[Trip],
    stop_times: &[StopTime],
) -> (
    Vec<TripPattern>,
    Vec<StopIdx>,
    Vec<TripIdx>,
    Vec<TripPatternIdx>,
) {
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
    for (i, (stop_seq, trips_in_seq)) in stop_seq_to_trips.iter().enumerate() {
        let idx = TripPatternIdx(i as u32);
        trips_in_seq.iter().for_each(|trip_idx| {
            trip_to_trip_pattern[trip_idx.to_usize()] = idx;
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
            idx: TripPatternIdx(i as u32),
        });
        stop_sequences.extend_from_slice(stop_seq);
        trips_in_sequences.extend_from_slice(trips_in_seq);
    }
    (
        trip_patterns,
        stop_sequences,
        trips_in_sequences,
        trip_to_trip_pattern,
    )
}
