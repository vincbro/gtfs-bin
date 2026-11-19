use std::collections::HashMap;

use crate::{
    compiler::tables,
    models::{
        RouteIndex, Sentinel, ServiceIndex, ShapeSlice, SliceBuilder, StopTimeSlice, StringSlice,
        Trip, TripIdSlice, TripIndex,
    },
};

pub fn build_trips(
    raw_trips: &[gtfs_structures::RawTrip],
    route_map: &HashMap<String, RouteIndex>,
    service_map: &HashMap<String, ServiceIndex>,
    shape_map: &HashMap<String, ShapeSlice>,
    slice_builder: &mut SliceBuilder<StringSlice>,
) -> (Vec<Trip>, HashMap<String, TripIndex>) {
    let mut id_map: HashMap<String, TripIndex> = HashMap::with_capacity(raw_trips.len());

    let trips: Vec<_> = raw_trips
        .iter()
        .filter_map(|trip| {
            route_map
                .get(&trip.route_id)
                .map(|&route_index| (trip, route_index))
        })
        .enumerate()
        .map(|(position, (trip, route_index))| {
            let index = TripIndex::from(position);
            id_map.insert(trip.id.clone(), index);

            let service_index = service_map
                .get(&trip.service_id)
                .copied()
                .unwrap_or(ServiceIndex::NONE);

            Trip {
                id: TripIdSlice::NONE,
                index,
                shape: trip
                    .shape_id
                    .as_ref()
                    .and_then(|shape_id| shape_map.get(shape_id).copied())
                    .into(),
                headsign: trip
                    .trip_headsign
                    .as_ref()
                    .map(|headsign| slice_builder.add(headsign))
                    .into(),
                route_index,
                service_index,
                stop_times: StopTimeSlice::NONE,
                short_name: trip
                    .trip_short_name
                    .as_ref()
                    .map(|name| slice_builder.add(name))
                    .into(),
            }
        })
        .collect();

    (trips, id_map)
}

pub fn build_trip_ids(
    trips: &mut [Trip],
    trip_map: &HashMap<String, TripIndex>,
) -> (Vec<TripIndex>, String) {
    let table = tables::build_id_table::<TripIdSlice, _>(trip_map, trips.len());
    for (trip, id) in trips.iter_mut().zip(table.slices) {
        trip.id = id;
    }
    (table.lookup, table.buffer)
}
