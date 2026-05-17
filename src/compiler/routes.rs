use crate::models::{
    Route, RouteIdSlice, RouteIdx, Sentinel, Slice, SliceBuilder, StringSlice, Trip, TripIdx,
    TripSlice,
};
use rayon::slice::ParallelSliceMut;
use std::collections::HashMap;

pub fn build_routes(
    raw_routes: &[gtfs_structures::Route],
    slice_builder: &mut SliceBuilder<StringSlice>,
) -> (Vec<Route>, HashMap<String, RouteIdx>) {
    let mut id_map: HashMap<String, RouteIdx> = HashMap::new();

    let routes: Vec<_> = raw_routes
        .iter()
        .enumerate()
        .map(|(i, route)| {
            let idx = RouteIdx::from(i);
            id_map.insert(route.id.clone(), idx);
            let id = RouteIdSlice::NONE;
            let long_name = route
                .long_name
                .as_ref()
                .map(|ln| slice_builder.add(ln))
                .into();
            let short_name = route
                .short_name
                .as_ref()
                .map(|sn| slice_builder.add(sn))
                .into();
            let description = route
                .desc
                .as_ref()
                .map(|desc| slice_builder.add(desc))
                .into();
            let route_type = route.route_type.into();
            Route::new(id, idx, long_name, short_name, description, route_type)
        })
        .collect();
    (routes, id_map)
}

pub fn build_route_ids(
    routes: &mut [Route],
    route_map: &HashMap<String, RouteIdx>,
) -> (Vec<RouteIdx>, String) {
    let mut id_builder = SliceBuilder::with_capacity(36 * routes.len());
    for (id, idx) in route_map {
        routes[idx.as_usize()].id = id_builder.add(id.as_str());
    }

    let route_ids = id_builder.take();

    // Build binary search friendly id lookup
    let mut route_id_lookup: Vec<_> = (0..routes.len()).map(RouteIdx::from).collect();
    route_id_lookup.par_sort_unstable_by(|a, b| {
        let id_a = &route_ids[routes[a.as_usize()].id.range()];
        let id_b = &route_ids[routes[b.as_usize()].id.range()];
        id_a.cmp(id_b)
    });

    (route_id_lookup, route_ids)
}

pub fn build_route_to_trips(trips: &[Trip], routes: &[Route]) -> (Vec<TripIdx>, Vec<TripSlice>) {
    let mut route_trip_pairs: Vec<(RouteIdx, TripIdx)> = trips
        .iter()
        .map(|trip| (trip.route_idx, trip.idx))
        .collect();

    route_trip_pairs.par_sort_unstable();

    // Swapped the variable names here:
    let mut route_to_trips = Vec::with_capacity(route_trip_pairs.len());
    let mut route_to_trips_lookup = vec![TripSlice::NONE; routes.len()];

    let mut current_route = RouteIdx::NONE;
    let mut start = 0;
    let mut count = 0;

    for (i, &(route_idx, trip_idx)) in route_trip_pairs.iter().enumerate() {
        if route_idx != current_route {
            if current_route != RouteIdx::NONE {
                route_to_trips_lookup[current_route.as_usize()] =
                    TripSlice::from_usize(start, count);
            }
            start = i;
            count = 0;
            current_route = route_idx;
        }
        route_to_trips.push(trip_idx);
        count += 1;
    }

    if current_route != RouteIdx::NONE {
        route_to_trips_lookup[current_route.as_usize()] = TripSlice::from_usize(start, count);
    }

    (route_to_trips, route_to_trips_lookup)
}
