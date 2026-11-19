use crate::{
    GTFS_BIN_VERSION,
    consumer::reader::Reader,
    models::{
        Header, Route, RouteIndex, SearchStop, Service, ServiceIndex, Shape, Stop, StopIndex,
        StopTime, Transfer, TransferIndex, TransferSlice, Trip, TripIndex, TripPattern,
        TripPatternIndex, TripPatternSlice, TripSlice,
    },
};
use memmap2::Mmap;
use std::str;

mod error;
mod reader;
mod routes;
mod sections;
mod services;
mod shapes;
mod stop_times;
mod stops;
mod strings;
mod transfers;
mod trip_patterns;
mod trips;

pub use error::{Error, Result};

/// A read-only view over a compiled GTFS file.
///
/// Every table the file holds is a slice into the memory map, so nothing is copied.
///
/// After `new` succeeds:
/// - Every id and string section is valid UTF-8.
/// - Each lookup table has one entry per item of the table it indexes.
///
/// The indices stored inside the tables are not checked. They are trusted to come from the
/// compiler, so a hand-edited file can make an accessor panic.
#[derive(Debug)]
pub struct Consumer<'a> {
    // Stops
    stops: &'a [Stop],
    stops_id_lookup: &'a [StopIndex],
    stop_ids: &'a [u8],

    // Routes
    routes: &'a [Route],
    routes_id_lookup: &'a [RouteIndex],
    route_ids: &'a [u8],

    // Trips
    trips: &'a [Trip],
    trips_id_lookup: &'a [TripIndex],
    trip_ids: &'a [u8],

    // Services
    services: &'a [Service],
    services_id_lookup: &'a [ServiceIndex],
    service_ids: &'a [u8],
    active_mask: &'a [u8],

    // Mappings
    route_to_trips: &'a [TripIndex],
    route_to_trips_lookup: &'a [TripSlice],

    stop_to_trips: &'a [TripIndex],
    stop_to_trips_lookup: &'a [TripSlice],

    search_stops: &'a [SearchStop],
    search_to_stops: &'a [StopIndex],

    // Stop times
    stop_times: &'a [StopTime],

    // Shapes
    shapes: &'a [Shape],

    // Trip patterns
    trip_patterns: &'a [TripPattern],
    trip_patterns_stop_seq: &'a [StopIndex],
    trip_patterns_trips: &'a [TripIndex],
    trip_to_trip_pattern: &'a [TripPatternIndex],
    stop_to_trip_pattern: &'a [TripPatternIndex],
    stop_to_trip_pattern_lookup: &'a [TripPatternSlice],

    // Transfer
    transfers: &'a [Transfer],
    stop_to_transfer_out: &'a [TransferSlice],
    transfers_in_indices: &'a [TransferIndex],
    stop_to_transfer_in: &'a [TransferSlice],

    strings: &'a [u8],
}

impl<'a> Consumer<'a> {
    /// Maps the sections of a compiled file.
    ///
    /// # Errors
    ///
    /// Returns an error if the header is wrong, a section is out of bounds or misaligned, a text
    /// section is not UTF-8, or a lookup table has the wrong length.
    pub fn new(mmap: &'a Mmap) -> Result<Self> {
        let header_size = size_of::<Header>();
        let header_bytes = mmap.get(..header_size).ok_or(Error::FileTooSmall)?;
        let header: &Header =
            bytemuck::try_from_bytes(header_bytes).map_err(|_| Error::InvalidSection)?;

        if header.magic != *b"GTFS" {
            return Err(Error::InvalidMagic);
        }
        if header.version != GTFS_BIN_VERSION {
            return Err(Error::UnsupportedVersion {
                expected: GTFS_BIN_VERSION,
                actual: header.version,
            });
        }

        let reader = Reader::new(mmap);

        let consumer = Self {
            stops: reader.cast_slice(header.stops)?,
            stop_ids: reader.bytes::<u8>(header.stop_ids)?,
            stops_id_lookup: reader.cast_slice(header.stops_id_lookup)?,

            routes: reader.cast_slice(header.routes)?,
            route_ids: reader.bytes::<u8>(header.route_ids)?,
            routes_id_lookup: reader.cast_slice(header.route_id_lookup)?,

            trips: reader.cast_slice(header.trips)?,
            trip_ids: reader.bytes::<u8>(header.trip_ids)?,
            trips_id_lookup: reader.cast_slice(header.trip_id_lookup)?,

            services: reader.cast_slice(header.services)?,
            service_ids: reader.bytes::<u8>(header.service_ids)?,
            services_id_lookup: reader.cast_slice(header.service_id_lookup)?,
            active_mask: reader.bytes::<u8>(header.active_mask)?,

            stop_times: reader.cast_slice(header.stop_times)?,

            shapes: reader.cast_slice(header.shapes)?,

            route_to_trips: reader.cast_slice(header.route_to_trips)?,
            route_to_trips_lookup: reader.cast_slice(header.route_to_trips_lookup)?,

            stop_to_trips: reader.cast_slice(header.stop_to_trips)?,
            stop_to_trips_lookup: reader.cast_slice(header.stop_to_trips_lookup)?,

            search_stops: reader.cast_slice(header.search_stops)?,
            search_to_stops: reader.cast_slice(header.search_to_stops)?,

            transfers: reader.cast_slice(header.transfers)?,
            stop_to_transfer_out: reader.cast_slice(header.stop_to_transfers_out)?,
            transfers_in_indices: reader.cast_slice(header.transfers_in_indices)?,
            stop_to_transfer_in: reader.cast_slice(header.stop_to_transfers_in)?,

            trip_patterns: reader.cast_slice(header.trip_patterns)?,
            trip_patterns_stop_seq: reader.cast_slice(header.trip_patterns_stop_seq)?,
            trip_patterns_trips: reader.cast_slice(header.trip_patterns_trip_seq)?,
            trip_to_trip_pattern: reader.cast_slice(header.trip_to_trip_pattern)?,
            stop_to_trip_pattern: reader.cast_slice(header.stop_to_trip_pattern)?,
            stop_to_trip_pattern_lookup: reader.cast_slice(header.stop_to_trip_pattern_lookup)?,

            strings: reader.bytes::<u8>(header.strings)?,
        };

        consumer.validate()?;
        Ok(consumer)
    }

    fn validate(&self) -> Result<()> {
        for (section, bytes) in [
            ("stop id", self.stop_ids),
            ("route id", self.route_ids),
            ("trip id", self.trip_ids),
            ("service id", self.service_ids),
            ("string", self.strings),
        ] {
            str::from_utf8(bytes).map_err(|_| Error::InvalidUtf8 { section })?;
        }

        for (table, table_len, lookup, lookup_len) in [
            (
                "stops",
                self.stops.len(),
                "stops_id_lookup",
                self.stops_id_lookup.len(),
            ),
            (
                "routes",
                self.routes.len(),
                "routes_id_lookup",
                self.routes_id_lookup.len(),
            ),
            (
                "trips",
                self.trips.len(),
                "trips_id_lookup",
                self.trips_id_lookup.len(),
            ),
            (
                "services",
                self.services.len(),
                "services_id_lookup",
                self.services_id_lookup.len(),
            ),
            (
                "routes",
                self.routes.len(),
                "route_to_trips_lookup",
                self.route_to_trips_lookup.len(),
            ),
            (
                "stops",
                self.stops.len(),
                "stop_to_trips_lookup",
                self.stop_to_trips_lookup.len(),
            ),
            (
                "stops",
                self.stops.len(),
                "stop_to_transfer_out",
                self.stop_to_transfer_out.len(),
            ),
            (
                "stops",
                self.stops.len(),
                "stop_to_transfer_in",
                self.stop_to_transfer_in.len(),
            ),
            (
                "stops",
                self.stops.len(),
                "stop_to_trip_pattern_lookup",
                self.stop_to_trip_pattern_lookup.len(),
            ),
            (
                "trips",
                self.trips.len(),
                "trip_to_trip_pattern",
                self.trip_to_trip_pattern.len(),
            ),
        ] {
            if table_len != lookup_len {
                return Err(Error::MismatchedSections { table, lookup });
            }
        }
        Ok(())
    }
}

/// Returns the text of a slice of a validated section.
///
/// # Panics
///
/// Panics if the slice cuts a multi-byte character, which a file from the compiler never does.
fn text(bytes: &[u8]) -> &str {
    str::from_utf8(bytes).expect("the compiler only writes slices that end on character boundaries")
}
