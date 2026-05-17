use std::collections::HashMap;

use crate::models::{
    Coordinate, LocationType, Opt, RouteType, SearchSlice, SearchStop, Sentinel, Slice,
    SliceBuilder, Stop, StopIdSlice, StopIdx, StopTime, StringSlice, TripIdx, TripSlice,
};
use rayon::slice::ParallelSliceMut;

pub fn build_stops(
    raw_stops: &[gtfs_structures::Stop],
    slice_builder: &mut SliceBuilder<StringSlice>,
) -> (Vec<Stop>, HashMap<String, StopIdx>) {
    let mut id_map: HashMap<String, StopIdx> = HashMap::new();

    // Convert raw_stops to stops
    let mut stops: Vec<_> = raw_stops
        .iter()
        .enumerate()
        .map(|(i, stop)| {
            let coordinate = if let (Some(lat), Some(lon)) = (stop.latitude, stop.longitude) {
                Opt::new(Coordinate::new(lat, lon))
            } else {
                Opt::new(Coordinate::NONE)
            };
            let idx = StopIdx::from(i);

            id_map.insert(stop.id.clone(), idx);

            let code = stop
                .code
                .as_ref()
                .map(|code| slice_builder.add(code))
                .into();

            let name = stop
                .name
                .as_ref()
                .map(|name| slice_builder.add(name))
                .into();

            let description = stop
                .description
                .as_ref()
                .map(|desc| slice_builder.add(desc))
                .into();

            let location_type = stop.location_type.into();

            Stop::new(
                StopIdSlice::NONE,
                idx,
                Opt::new(StopIdx::NONE),
                name,
                description,
                code,
                coordinate,
                location_type,
            )
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

    (stops, id_map)
}

pub fn build_stop_ids(
    stops: &mut [Stop],
    stop_map: &HashMap<String, StopIdx>,
) -> (Vec<StopIdx>, String) {
    let mut id_builder = SliceBuilder::with_capacity(36 * stops.len());
    for (id, idx) in stop_map {
        stops[idx.as_usize()].id = id_builder.add(id.as_str());
    }

    let stop_ids = id_builder.take();

    // Build binary search friendly id lookup
    let mut stop_id_lookup: Vec<_> = (0..stops.len()).map(StopIdx::from).collect();
    stop_id_lookup.par_sort_unstable_by(|a, b| {
        let id_a = &stop_ids[stops[a.as_usize()].id.range()];
        let id_b = &stop_ids[stops[b.as_usize()].id.range()];
        id_a.cmp(id_b)
    });

    (stop_id_lookup, stop_ids)
}

pub fn build_stop_to_trips(
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
                stop_to_trips_lookup[current_stop.as_usize()] = TripSlice::from_usize(start, count);
            }
            start = i;
            count = 0;
            current_stop = stop_idx;
        }
        stop_to_trips.push(trip_idx);
        count += 1;
    }

    if current_stop != StopIdx::NONE {
        stop_to_trips_lookup[current_stop.as_usize()] = TripSlice::from_usize(start, count);
    }

    (stop_to_trips, stop_to_trips_lookup)
}

pub fn build_stop_search(stops: &[Stop]) -> (Vec<SearchStop>, Vec<StopIdx>) {
    let mut grouped_stops: HashMap<StringSlice, Vec<StopIdx>> = HashMap::with_capacity(stops.len());

    for stop in stops {
        if let Some(name) = stop.name.as_option()
            && (stop.location_type == LocationType::STOP_AREA
                || (stop.location_type == LocationType::GENERIC_NODE && stop.parent_idx.is_none()))
        {
            grouped_stops.entry(name).or_default().push(stop.idx);
        } else if let Some(parent) = stop.parent_idx.as_option() {
            let parent = &stops[parent.as_usize()];
            if let Some(name) = parent.name.as_option() {
                grouped_stops.entry(name).or_default().push(stop.idx);
            }
        }
    }

    let mut search_stops: Vec<SearchStop> = Vec::with_capacity(grouped_stops.len());
    let mut search_to_stops: Vec<StopIdx> = Vec::new();

    for (i, (name, group)) in grouped_stops.into_iter().enumerate() {
        let start = search_to_stops.len();
        let count = group.len();
        search_to_stops.extend_from_slice(&group);
        search_stops.push(SearchStop::new(
            i.into(),
            name,
            SearchSlice::from_usize(start, count),
            RouteType::NONE,
        ));
    }

    (search_stops, search_to_stops)
}
