use std::collections::HashMap;

use rayon::slice::ParallelSliceMut;

use crate::models::{
    RouteIdx, Sentinel, ServiceIdx, ShapeSlice, Slice, SliceBuilder, StopTimeSlice, StringSlice,
    Trip, TripIdSlice, TripIdx,
};

pub(crate) fn build_trips(
    raw_trips: &[gtfs_structures::RawTrip],
    route_map: &HashMap<String, RouteIdx>,
    service_map: &HashMap<String, ServiceIdx>,
    shape_map: &HashMap<String, ShapeSlice>,
    slice_builder: &mut SliceBuilder<StringSlice>,
) -> Result<(Vec<Trip>, HashMap<String, TripIdx>), gtfs_structures::Error> {
    let mut id_map: HashMap<String, TripIdx> = HashMap::new();

    let trips: Vec<_> = raw_trips
        .iter()
        .filter_map(|trip| {
            route_map
                .get(&trip.route_id)
                .copied()
                .map(|idx| (trip, idx))
        })
        .enumerate()
        .map(|(i, (trip, route_idx))| {
            let idx = TripIdx(i as u32);
            id_map.insert(trip.id.clone(), idx);

            let service_idx = service_map
                .get(&trip.service_id)
                .copied()
                .unwrap_or(ServiceIdx::NONE);

            Trip {
                id: TripIdSlice::NONE,
                idx,
                shape: trip
                    .shape_id
                    .as_ref()
                    .and_then(|shape_id| shape_map.get(shape_id).copied())
                    .into(),
                headsign: trip
                    .trip_headsign
                    .as_ref()
                    .map(|hs| slice_builder.add(hs))
                    .into(),
                route_idx,
                service_idx,
                stop_times: StopTimeSlice::NONE,
                short_name: trip
                    .trip_short_name
                    .as_ref()
                    .map(|sn| slice_builder.add(sn))
                    .into(),
            }
        })
        .collect();

    Ok((trips, id_map))
}

pub(crate) fn build_trip_ids(
    trips: &mut [Trip],
    trip_map: &HashMap<String, TripIdx>,
) -> (Vec<TripIdx>, String) {
    let mut id_builder = SliceBuilder::with_capacity(36 * trips.len());
    for (id, idx) in trip_map.iter() {
        trips[idx.to_usize()].id = id_builder.add(id.as_str());
    }

    let trip_ids = id_builder.take();

    // Build binary search friendly id lookup
    let mut trip_id_lookup: Vec<_> = (0..trips.len()).map(|i| TripIdx(i as u32)).collect();
    trip_id_lookup.par_sort_unstable_by(|a, b| {
        let id_a = &trip_ids[trips[a.to_usize()].id.range()];
        let id_b = &trip_ids[trips[b.to_usize()].id.range()];
        id_a.cmp(id_b)
    });

    (trip_id_lookup, trip_ids)
}
