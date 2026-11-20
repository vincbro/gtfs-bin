use flatnet_core::models::{
    Optional, RouteIndex, Sentinel, ServiceIndex, SliceBuilder, StopTimeSlice, StringSlice, Trip,
    TripIdSlice, TripIndex,
};

use crate::{network::TripRecord, tables};

pub(super) fn build_trips(
    records: &[TripRecord],
    slice_builder: &mut SliceBuilder<StringSlice>,
) -> Vec<Trip> {
    records
        .iter()
        .enumerate()
        .map(|(position, trip)| Trip {
            id: TripIdSlice::NONE,
            index: TripIndex::from(position),
            // Set once the trip's stops are known, because it is cut at them.
            shape: Optional::default(),
            headsign: trip
                .headsign
                .as_ref()
                .map(|headsign| slice_builder.add(headsign))
                .into(),
            route_index: RouteIndex::from(trip.route),
            service_index: trip.service.map_or(ServiceIndex::NONE, ServiceIndex::from),
            stop_times: StopTimeSlice::NONE,
            block: trip
                .block
                .as_ref()
                .map(|block| slice_builder.add(block))
                .into(),
            direction: trip.direction,
            padding: [0; 3],
            short_name: trip
                .short_name
                .as_ref()
                .map(|name| slice_builder.add(name))
                .into(),
        })
        .collect()
}

pub(super) fn build_trip_ids(
    trips: &mut [Trip],
    records: &[TripRecord],
) -> (Vec<TripIndex>, String) {
    let ids: Vec<&str> = records.iter().map(|trip| trip.id.as_str()).collect();
    let table = tables::build_id_table::<TripIdSlice, _>(&ids);
    for (trip, id) in trips.iter_mut().zip(table.slices) {
        trip.id = id;
    }
    (table.lookup, table.buffer)
}
