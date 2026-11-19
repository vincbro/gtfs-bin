use crate::{
    compiler::tables,
    models::{
        Distance, Sentinel, SliceBuilder, StopIndex, StopTime, StopTimeIndex, StopTimeSlice,
        StringSlice, Time, Trip, TripIndex,
    },
};
use rayon::slice::ParallelSliceMut;
use std::collections::HashMap;

pub fn build_stop_times(
    raw_stop_times: &[gtfs_structures::RawStopTime],
    trips: &mut [Trip],
    trip_map: &HashMap<String, TripIndex>,
    stop_map: &HashMap<String, StopIndex>,
    slice_builder: &mut SliceBuilder<StringSlice>,
) -> Vec<StopTime> {
    let mut stop_times: Vec<_> = raw_stop_times
        .iter()
        .filter_map(|stop_time| {
            let trip_index = trip_map.get(&stop_time.trip_id).copied()?;
            let stop_index = stop_map.get(&stop_time.stop_id).copied()?;
            Some((stop_time, trip_index, stop_index))
        })
        .map(|(stop_time, trip_index, stop_index)| StopTime {
            index: StopTimeIndex::NONE,
            headsign: stop_time
                .stop_headsign
                .as_ref()
                .map(|headsign| slice_builder.add(headsign))
                .into(),
            stop_index,
            trip_index,
            sequence: stop_time.stop_sequence,
            arrival_time: stop_time.arrival_time.map(Time).into(),
            departure_time: stop_time.departure_time.map(Time).into(),
            distance_traveled: stop_time.shape_dist_traveled.map(Distance).into(),
        })
        .collect();

    stop_times.par_sort_unstable_by(|a, b| {
        a.trip_index
            .cmp(&b.trip_index)
            .then(a.sequence.cmp(&b.sequence))
    });

    for (position, stop_time) in stop_times.iter_mut().enumerate() {
        stop_time.index = StopTimeIndex::from(position);
    }

    let slices: Vec<StopTimeSlice> = tables::slices_by_key(
        stop_times.iter().map(|stop_time| stop_time.trip_index),
        trips.len(),
    );
    for (trip, slice) in trips.iter_mut().zip(slices) {
        trip.stop_times = slice;
    }

    stop_times
}
