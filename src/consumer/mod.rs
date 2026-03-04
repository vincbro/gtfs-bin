use crate::{
    GTFS_BIN_VERSION,
    models::{Route, RouteIdx, Stop, StopIdx, StopTime, StopTimeSlice, Trip, TripIdx, TripSlice},
};
use bytemuck::{cast_slice, from_bytes};
use memmap2::Mmap;

mod header;
mod routes;
mod stops;
mod stoptimes;
mod trips;
pub use header::*;

#[derive(Debug)]
pub struct Consumer<'a> {
    // Stops
    pub stops: &'a [Stop],
    pub stops_id_lookup: &'a [StopIdx],
    pub stop_ids: &'a [u8],

    // Routes
    pub routes: &'a [Route],
    pub routes_id_lookup: &'a [RouteIdx],
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
    pub fn new(mmap: &'a Mmap) -> Result<Self, crate::Error> {
        let header_size = size_of::<Header>();
        if mmap.len() < header_size {
            return Err(crate::Error::FileTooSmall);
        }

        let header: &Header = from_bytes(&mmap[..header_size]);

        // Validate header
        if header.magic != *b"GTFS" {
            return Err(crate::Error::InvalidMagic);
        }
        if header.version != GTFS_BIN_VERSION {
            return Err(crate::Error::UnsupportedVersion {
                expected: GTFS_BIN_VERSION,
                actual: header.version,
            });
        }

        let get_bytes = |section: Section, element_size: usize| -> &'a [u8] {
            let start = section.offset as usize;
            let end = start + (section.count as usize * element_size);
            &mmap[start..end]
        };

        Ok(Self {
            stops: cast_slice(get_bytes(header.stops, size_of::<Stop>())),
            stop_ids: get_bytes(header.stop_ids, size_of::<u8>()),
            stops_id_lookup: cast_slice(get_bytes(header.stop_id_lookup, size_of::<StopIdx>())),

            routes: cast_slice(get_bytes(header.routes, size_of::<Route>())),
            route_ids: get_bytes(header.route_ids, size_of::<u8>()),
            routes_id_lookup: cast_slice(get_bytes(header.route_id_lookup, size_of::<StopIdx>())),

            trips: cast_slice(get_bytes(header.trips, size_of::<Trip>())),
            trip_ids: get_bytes(header.trip_ids, size_of::<u8>()),
            trips_id_lookup: cast_slice(get_bytes(header.trip_id_lookup, size_of::<StopIdx>())),

            stop_times: cast_slice(get_bytes(header.stop_times, size_of::<StopTime>())),

            route_to_trips: cast_slice(get_bytes(header.route_to_trips, size_of::<TripIdx>())),
            route_to_trips_lookup: cast_slice(get_bytes(
                header.route_to_trips_lookup,
                size_of::<TripSlice>(),
            )),

            stop_to_trips: cast_slice(get_bytes(header.stop_to_trips, size_of::<TripIdx>())),
            stop_to_trips_lookup: cast_slice(get_bytes(
                header.stop_to_trips_lookup,
                size_of::<TripSlice>(),
            )),

            trip_to_stop_times: cast_slice(get_bytes(
                header.stop_to_trips_lookup,
                size_of::<StopTimeSlice>(),
            )),
        })
    }
}
