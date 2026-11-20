use std::collections::HashMap;

use gtfs_bin_core::models::{
    Route, RouteIdSlice, RouteIndex, Sentinel, SliceBuilder, StringSlice, Trip, TripIndex,
    TripSlice,
};
use rayon::slice::ParallelSliceMut;

use crate::{records::RawRoute, tables};

pub(super) fn build_routes(
    raw_routes: &[RawRoute],
    slice_builder: &mut SliceBuilder<StringSlice>,
) -> (Vec<Route>, HashMap<String, RouteIndex>) {
    let mut id_map: HashMap<String, RouteIndex> = HashMap::with_capacity(raw_routes.len());

    let routes: Vec<_> = raw_routes
        .iter()
        .enumerate()
        .map(|(position, route)| {
            let index = RouteIndex::from(position);
            id_map.insert(route.id.clone(), index);
            let long_name = route
                .long_name
                .as_ref()
                .map(|name| slice_builder.add(name))
                .into();
            let short_name = route
                .short_name
                .as_ref()
                .map(|name| slice_builder.add(name))
                .into();
            let description = route
                .description
                .as_ref()
                .map(|description| slice_builder.add(description))
                .into();
            Route::new(
                RouteIdSlice::NONE,
                index,
                long_name,
                short_name,
                description,
                route.route_type,
            )
        })
        .collect();
    (routes, id_map)
}

pub(super) fn build_route_ids(
    routes: &mut [Route],
    route_map: &HashMap<String, RouteIndex>,
) -> (Vec<RouteIndex>, String) {
    let table = tables::build_id_table::<RouteIdSlice, _>(route_map, routes.len());
    for (route, id) in routes.iter_mut().zip(table.slices) {
        route.id = id;
    }
    (table.lookup, table.buffer)
}

pub(super) fn build_route_to_trips(
    trips: &[Trip],
    routes: &[Route],
) -> (Vec<TripIndex>, Vec<TripSlice>) {
    let mut route_trip_pairs: Vec<(RouteIndex, TripIndex)> = trips
        .iter()
        .map(|trip| (trip.route_index, trip.index))
        .collect();
    route_trip_pairs.par_sort_unstable();

    let route_to_trips = route_trip_pairs.iter().map(|&(_, trip)| trip).collect();
    let lookup = tables::slices_by_key(
        route_trip_pairs.iter().map(|&(route, _)| route),
        routes.len(),
    );
    (route_to_trips, lookup)
}
