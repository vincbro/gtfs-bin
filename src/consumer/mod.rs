use crate::{
    GTFS_BIN_VERSION,
    consumer::reader::Reader,
    models::{
        Header, Route, RouteIdx, Service, ServiceIdx, Stop, StopIdx, StopTime, StopTimeSlice,
        Transfer, TransferIdx, TransferSlice, Trip, TripIdx, TripSlice,
    },
};
use bytemuck::from_bytes;
use memmap2::Mmap;

mod reader;
mod routes;
mod stops;
mod stoptimes;
mod transfers;
mod trips;

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

    // Services
    pub services: &'a [Service],
    pub services_id_lookup: &'a [ServiceIdx],
    pub service_ids: &'a [u8],
    pub active_mask: &'a [u8],

    // Mappings
    pub route_to_trips: &'a [TripIdx],
    pub route_to_trips_lookup: &'a [TripSlice],

    pub stop_to_trips: &'a [TripIdx],
    pub stop_to_trips_lookup: &'a [TripSlice],

    // Stop times
    pub stop_times: &'a [StopTime],
    pub trip_to_stop_times: &'a [StopTimeSlice],

    // Transfer
    pub transfers: &'a [Transfer],
    pub stop_to_transfer_out: &'a [TransferSlice],
    pub transfers_in_indencies: &'a [TransferIdx],
    pub stop_to_transfer_in: &'a [TransferSlice],
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

        let reader = Reader::new(mmap);

        Ok(Self {
            stops: reader.cast_slice(header.stops),
            stop_ids: reader.get_bytes::<u8>(header.stop_ids),
            stops_id_lookup: reader.cast_slice(header.stops_id_lookup),

            routes: reader.cast_slice(header.routes),
            route_ids: reader.get_bytes::<u8>(header.route_ids),
            routes_id_lookup: reader.cast_slice(header.route_id_lookup),

            trips: reader.cast_slice(header.trips),
            trip_ids: reader.get_bytes::<u8>(header.trip_ids),
            trips_id_lookup: reader.cast_slice(header.trip_id_lookup),

            services: reader.cast_slice(header.services),
            service_ids: reader.get_bytes::<u8>(header.service_ids),
            services_id_lookup: reader.cast_slice(header.service_id_lookup),
            active_mask: reader.get_bytes::<u8>(header.active_mask),

            stop_times: reader.cast_slice(header.stop_times),

            route_to_trips: reader.cast_slice(header.route_to_trips),
            route_to_trips_lookup: reader.cast_slice(header.route_to_trips_lookup),

            stop_to_trips: reader.cast_slice(header.stop_to_trips),
            stop_to_trips_lookup: reader.cast_slice(header.stop_to_trips_lookup),

            trip_to_stop_times: reader.cast_slice(header.trip_to_stop_times),

            transfers: reader.cast_slice(header.transfers),
            stop_to_transfer_out: reader.cast_slice(header.stop_to_transfers_out),
            transfers_in_indencies: reader.cast_slice(header.transfers_in_indencies),
            stop_to_transfer_in: reader.cast_slice(header.stop_to_transfers_in),
        })
    }
}
