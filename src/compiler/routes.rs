use crate::{
    compiler::Compiler,
    models::{Route, RouteIdx, Sentinel, Slice, SliceBuilder, StringSlice, TripIdx, TripSlice},
};
use rayon::slice::ParallelSliceMut;

impl Compiler {
    pub(crate) fn build_routes(
        &mut self,
        raw_routes: &[gtfs_structures::Route],
        slice_builder: &mut SliceBuilder<StringSlice>,
    ) -> Result<(), gtfs_structures::Error> {
        // Recomended max length for a gtfs id is 36 characters
        let mut id_builder = SliceBuilder::with_capacity(36 * raw_routes.len());

        let routes: Vec<_> = raw_routes
            .iter()
            .enumerate()
            .map(|(i, route)| Route {
                id: id_builder.add(&route.id),
                long_name: route
                    .long_name
                    .as_ref()
                    .map(|ln| slice_builder.add(ln))
                    .into(),
                short_name: route
                    .short_name
                    .as_ref()
                    .map(|sn| slice_builder.add(sn))
                    .into(),
                description: route
                    .desc
                    .as_ref()
                    .map(|desc| slice_builder.add(desc))
                    .into(),
                idx: RouteIdx(i as u32),
                rtype: 0,
            })
            .collect();

        let route_ids = id_builder.take();

        // Build binary search friendly id lookup
        let mut route_id_lookup: Vec<_> = (0..routes.len()).map(|i| RouteIdx(i as u32)).collect();
        route_id_lookup.par_sort_unstable_by(|a, b| {
            let id_a = &route_ids[routes[a.to_usize()].id.range()];
            let id_b = &route_ids[routes[b.to_usize()].id.range()];
            id_a.cmp(id_b)
        });

        self.routes = routes;
        self.route_id_lookup = route_id_lookup;
        self.route_ids = route_ids;

        Ok(())
    }

    pub(crate) fn build_route_to_trips(&mut self) {
        let mut route_trip_pairs: Vec<(RouteIdx, TripIdx)> = self
            .trips
            .iter()
            .map(|trip| (trip.route_idx, trip.idx))
            .collect();

        route_trip_pairs.par_sort_unstable_by_key(|&(route_idx, _)| route_idx);

        let mut route_to_trips = vec![TripSlice::NONE; self.routes.len()];
        let mut route_to_trips_lookup = Vec::with_capacity(route_trip_pairs.len());

        let mut current_route = RouteIdx::NONE;
        let mut start = 0;
        let mut count = 0;

        for (i, &(route_idx, trip_idx)) in route_trip_pairs.iter().enumerate() {
            if route_idx != current_route {
                if current_route != RouteIdx::NONE {
                    route_to_trips[current_route.to_usize()] = TripSlice { start, count };
                }
                start = i as u32;
                count = 0;
                current_route = route_idx;
            }
            route_to_trips_lookup.push(trip_idx);
            count += 1;
        }

        if current_route != RouteIdx::NONE {
            route_to_trips[current_route.to_usize()] = TripSlice { start, count };
        }

        self.route_to_trips = route_to_trips;
        self.route_to_trips_lookup = route_to_trips_lookup;
    }
}
