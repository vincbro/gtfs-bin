use crate::{
    compiler::Compiler,
    models::{RouteIdx, Sentinel, ServiceIdx, Slice, SliceBuilder, StringSlice, Trip, TripIdx},
};
use rayon::slice::ParallelSliceMut;

impl Compiler {
    pub(crate) fn build_trips(
        &mut self,
        raw_trips: &[gtfs_structures::RawTrip],
        slice_builder: &mut SliceBuilder<StringSlice>,
    ) -> Result<(), gtfs_structures::Error> {
        // Recomended max length for a gtfs id is 36 characters
        let mut id_builder = SliceBuilder::with_capacity(36 * raw_trips.len());

        let trips: Vec<_> = raw_trips
            .iter()
            .enumerate()
            .filter_map(|(i, trip)| {
                let result = self.route_id_lookup.binary_search_by(|&idx| {
                    let route = &self.routes[idx.to_usize()];
                    let current_id = &self.route_ids[route.id.range()];
                    current_id.cmp(&trip.route_id)
                });
                result.ok().map(|idx| (i, trip, RouteIdx(idx as u32)))
            })
            .map(|(i, trip, route_idx)| Trip {
                id: id_builder.add(&trip.id),
                idx: TripIdx(i as u32),
                headsign: trip
                    .trip_headsign
                    .as_ref()
                    .map(|hs| slice_builder.add(hs))
                    .into(),
                route_idx,
                service_idx: ServiceIdx::NONE,
                short_name: trip
                    .trip_short_name
                    .as_ref()
                    .map(|sn| slice_builder.add(sn))
                    .into(),
            })
            .collect();

        let trip_ids = id_builder.take();

        // Build binary search friendly id lookup
        let mut trip_id_lookup: Vec<_> = (0..trips.len()).map(|i| TripIdx(i as u32)).collect();
        trip_id_lookup.par_sort_unstable_by(|a, b| {
            let id_a = &trip_ids[trips[a.to_usize()].id.range()];
            let id_b = &trip_ids[trips[b.to_usize()].id.range()];
            id_a.cmp(id_b)
        });

        self.trips = trips;
        self.trip_id_lookup = trip_id_lookup;
        self.trip_ids = trip_ids;
        Ok(())
    }
}
