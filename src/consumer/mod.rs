use crate::{
    GTFS_BIN_VERSION,
    models::{Route, RouteIdx, Stop, StopIdx, StopTime, StopTimeSlice, Trip, TripIdx, TripSlice},
};
mod header;
pub use header::*;
use memmap2::Mmap;

#[derive(Debug)]
pub struct Consumer<'a> {
    // Stops
    pub stops: &'a [Stop],
    pub stops_id_lookup: &'a [StopIdx],
    pub stop_ids: &'a [u8],

    // Routes
    pub routes: &'a [Route],
    pub route_id_lookup: &'a [RouteIdx],
    pub route_ids: &'a [u8],

    // Trips
    pub trips: &'a [Trip],
    pub trips_id_lookup: &'a [TripIdx],
    pub trip_ids: &'a [u8],

    // Stop times
    pub stop_times: &'a [StopTime],

    // Mappings
    pub route_to_trips: &'a [TripIdx],
    pub route_to_trips_lookup: &'a [TripSlice],

    pub stop_to_trips: &'a [TripIdx],
    pub stop_to_trips_lookup: &'a [TripSlice],

    pub trip_to_stop_times: &'a [StopTimeSlice],
}

impl<'a> Consumer<'a> {
    pub fn new(mmap: &'a Mmap) -> Result<Self, &'static str> {
        let header_size = std::mem::size_of::<Header>();
        if mmap.len() < header_size {
            return Err("File is too small to contain a valid header");
        }

        let header: &Header = bytemuck::from_bytes(&mmap[..header_size]);

        if header.magic != *b"GTFS" {
            return Err("Invalid magic number: Not a compiled GTFS file");
        }
        if header.version != GTFS_BIN_VERSION {
            return Err("Unsupported GTFS binary version");
        }

        let get_bytes = |section: Section, element_size: usize| -> &'a [u8] {
            let start = section.offset as usize;
            let end = start + (section.count as usize * element_size);
            &mmap[start..end]
        };

        Ok(Self {
            stops: bytemuck::cast_slice(get_bytes(header.stops, std::mem::size_of::<Stop>())),
            stop_ids: get_bytes(header.stop_ids, std::mem::size_of::<u8>()),
            stops_id_lookup: bytemuck::cast_slice(get_bytes(
                header.stop_id_lookup,
                std::mem::size_of::<StopIdx>(),
            )),

            routes: bytemuck::cast_slice(get_bytes(header.routes, std::mem::size_of::<Route>())),
            route_ids: get_bytes(header.route_ids, 1),
            route_id_lookup: bytemuck::cast_slice(get_bytes(
                header.route_id_lookup,
                std::mem::size_of::<StopIdx>(),
            )),

            trips: bytemuck::cast_slice(get_bytes(header.trips, std::mem::size_of::<Trip>())),
            trip_ids: get_bytes(header.trip_ids, 1),
            trips_id_lookup: bytemuck::cast_slice(get_bytes(
                header.trip_id_lookup,
                std::mem::size_of::<StopIdx>(),
            )),

            stop_times: bytemuck::cast_slice(get_bytes(
                header.stop_times,
                std::mem::size_of::<StopTime>(),
            )),

            route_to_trips: bytemuck::cast_slice(get_bytes(
                header.route_to_trips,
                std::mem::size_of::<TripIdx>(),
            )),
            route_to_trips_lookup: bytemuck::cast_slice(get_bytes(
                header.route_to_trips_lookup,
                std::mem::size_of::<TripSlice>(),
            )),

            stop_to_trips: bytemuck::cast_slice(get_bytes(
                header.stop_to_trips,
                std::mem::size_of::<TripIdx>(),
            )),
            stop_to_trips_lookup: bytemuck::cast_slice(get_bytes(
                header.stop_to_trips_lookup,
                std::mem::size_of::<TripSlice>(),
            )),

            trip_to_stop_times: bytemuck::cast_slice(get_bytes(
                header.stop_to_trips_lookup,
                std::mem::size_of::<StopTimeSlice>(),
            )),
        })
    }
}
