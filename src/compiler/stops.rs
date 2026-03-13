use std::collections::HashMap;

use crate::models::{
    Coordinate, Opt, Sentinel, Slice, SliceBuilder, Stop, StopIdSlice, StopIdx, StopTime,
    StringSlice, TripIdx, TripSlice,
};
use rayon::slice::ParallelSliceMut;

pub(crate) fn build_stops(
    raw_stops: &[gtfs_structures::Stop],
    slice_builder: &mut SliceBuilder<StringSlice>,
) -> Result<(Vec<Stop>, HashMap<String, StopIdx>), gtfs_structures::Error> {
    let mut id_map: HashMap<String, StopIdx> = HashMap::new();

    // Convert raw_stops to stops
    let mut stops: Vec<_> = raw_stops
        .iter()
        .enumerate()
        .map(|(i, stop)| {
            let coordinate = if let Some(lat) = stop.latitude
                && let Some(lon) = stop.longitude
            {
                Opt::new(Coordinate::new(lat, lon))
            } else {
                Opt::new(Coordinate::NONE)
            };
            let idx = StopIdx(i as u32);

            id_map.insert(stop.id.clone(), idx);

            Stop {
                coordinate,
                id: StopIdSlice::NONE,
                code: stop
                    .code
                    .as_ref()
                    .map(|code| slice_builder.add(code))
                    .into(),
                name: stop
                    .name
                    .as_ref()
                    .map(|name| slice_builder.add(name))
                    .into(),
                description: stop
                    .description
                    .as_ref()
                    .map(|desc| slice_builder.add(desc))
                    .into(),
                idx,
                parent_idx: Opt::new(StopIdx::NONE),
            }
        })
        .collect();

    // Map parent stations
    raw_stops.iter().enumerate().for_each(|(i, stop)| {
        if let Some(id) = &stop.parent_station
            && let Some(idx) = id_map.get(id).copied()
        {
            stops[i].parent_idx = Opt::new(idx);
        }
    });

    Ok((stops, id_map))
}

pub(crate) fn build_stop_ids(
    stops: &mut [Stop],
    stop_map: &HashMap<String, StopIdx>,
) -> (Vec<StopIdx>, String) {
    let mut id_builder = SliceBuilder::with_capacity(36 * stops.len());
    for (id, idx) in stop_map.iter() {
        stops[idx.to_usize()].id = id_builder.add(id.as_str());
    }

    let stop_ids = id_builder.take();

    // Build binary search friendly id lookup
    let mut stop_id_lookup: Vec<_> = (0..stops.len()).map(|i| StopIdx(i as u32)).collect();
    stop_id_lookup.par_sort_unstable_by(|a, b| {
        let id_a = &stop_ids[stops[a.to_usize()].id.range()];
        let id_b = &stop_ids[stops[b.to_usize()].id.range()];
        id_a.cmp(id_b)
    });

    (stop_id_lookup, stop_ids)
}

pub(crate) fn build_stop_to_trips(
    stops: &[Stop],
    stop_times: &[StopTime],
) -> (Vec<TripIdx>, Vec<TripSlice>) {
    let mut stop_trip_pairs: Vec<(StopIdx, TripIdx)> = stop_times
        .iter()
        .map(|st| (st.stop_idx, st.trip_idx))
        .collect();

    stop_trip_pairs.par_sort_unstable();
    stop_trip_pairs.dedup();

    // Swapped the internal variable initializations
    let mut stop_to_trips = Vec::with_capacity(stop_trip_pairs.len());
    let mut stop_to_trips_lookup = vec![TripSlice::NONE; stops.len()];

    let mut current_stop = StopIdx::NONE;
    let mut start = 0;
    let mut count = 0;

    for (i, &(stop_idx, trip_idx)) in stop_trip_pairs.iter().enumerate() {
        if stop_idx != current_stop {
            if current_stop != StopIdx::NONE {
                stop_to_trips_lookup[current_stop.to_usize()] = TripSlice { start, count };
            }
            start = i as u32;
            count = 0;
            current_stop = stop_idx;
        }
        stop_to_trips.push(trip_idx);
        count += 1;
    }

    if current_stop != StopIdx::NONE {
        stop_to_trips_lookup[current_stop.to_usize()] = TripSlice { start, count };
    }

    (stop_to_trips, stop_to_trips_lookup)
}
