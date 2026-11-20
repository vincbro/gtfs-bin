use flatnet_core::models::{
    Route, RouteIdSlice, RouteIndex, Sentinel, SliceBuilder, StringSlice, Trip, TripIndex,
    TripSlice,
};
use rayon::slice::ParallelSliceMut;

use crate::{network::RouteRecord, tables};

pub(super) fn build_routes(
    records: &[RouteRecord],
    slice_builder: &mut SliceBuilder<StringSlice>,
) -> Vec<Route> {
    records
        .iter()
        .enumerate()
        .map(|(position, route)| {
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
                RouteIndex::from(position),
                long_name,
                short_name,
                description,
                route.route_type,
            )
        })
        .collect()
}

pub(super) fn build_route_ids(
    routes: &mut [Route],
    records: &[RouteRecord],
) -> (Vec<RouteIndex>, String) {
    let ids: Vec<&str> = records.iter().map(|route| route.id.as_str()).collect();
    let table = tables::build_id_table::<RouteIdSlice, _>(&ids);
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
