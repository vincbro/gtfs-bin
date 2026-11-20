use flatnet_core::models::{
    Optional, RouteIndex, Sentinel, ServiceIndex, SliceBuilder, StopTimeSlice, StringSlice, Trip,
    TripIdSlice, TripIndex, TripInfo,
};

use crate::{network::TripRecord, tables};

pub(super) fn build_trips(
    records: &[TripRecord],
    slice_builder: &mut SliceBuilder<StringSlice>,
    text: bool,
) -> (Vec<Trip>, Vec<TripInfo>) {
    records
        .iter()
        .enumerate()
        .map(|(position, trip)| {
            let row = Trip {
                id: TripIdSlice::NONE,
                index: TripIndex::from(position),
                // Set once the trip's stops are known, because it is cut at them.
                shape: Optional::default(),
                route_index: RouteIndex::from(trip.route),
                service_index: trip.service.map_or(ServiceIndex::NONE, ServiceIndex::from),
                stop_times: StopTimeSlice::NONE,
                direction: trip.direction,
                padding: [0; 3],
            };
            let info = TripInfo {
                headsign: trip
                    .headsign
                    .as_ref()
                    .filter(|_| text)
                    .map(|headsign| slice_builder.add(headsign))
                    .into(),
                short_name: trip
                    .short_name
                    .as_ref()
                    .filter(|_| text)
                    .map(|name| slice_builder.add(name))
                    .into(),
                block: trip
                    .block
                    .as_ref()
                    .filter(|_| text)
                    .map(|block| slice_builder.add(block))
                    .into(),
            };
            (row, info)
        })
        .unzip()
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
