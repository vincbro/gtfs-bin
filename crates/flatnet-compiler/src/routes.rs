use flatnet_core::models::{
    Route, RouteIdSlice, RouteIndex, Sentinel, Slice, SliceBuilder, StringSlice, Time, Timing,
    Trip, TripIndex, TripSlice,
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

/// The trips of each route, sorted by the time they start and then by index, and the start of each,
/// at the same positions.
pub(super) struct BuiltRouteTrips {
    pub route_to_trips: Vec<TripIndex>,
    pub lookup: Vec<TripSlice>,
    pub starts: Vec<Time>,
}

/// Groups the trips by route, and within a route by when they start, so a trip can be found by its
/// route and start with a binary search. A trip with no time at its first stop starts at
/// `Time::NONE` and sorts last.
pub(super) fn build_route_to_trips(
    trips: &[Trip],
    routes: &[Route],
    timings: &[Timing],
) -> BuiltRouteTrips {
    let mut keyed: Vec<(RouteIndex, Time, TripIndex)> = trips
        .iter()
        .map(|trip| (trip.route_index, start_of(trip, timings), trip.index))
        .collect();
    keyed.par_sort_unstable();

    let lookup = tables::slices_by_key(keyed.iter().map(|&(route, ..)| route), routes.len());
    BuiltRouteTrips {
        route_to_trips: keyed.iter().map(|&(.., trip)| trip).collect(),
        starts: keyed.iter().map(|&(_, start, _)| start).collect(),
        lookup,
    }
}

/// When the trip leaves its first stop, or arrives there if it has no departure.
fn start_of(trip: &Trip, timings: &[Timing]) -> Time {
    timings[trip.stop_times.range()]
        .first()
        .and_then(|first| {
            first
                .departure
                .as_option()
                .or_else(|| first.arrival.as_option())
        })
        .unwrap_or(Time::NONE)
}
