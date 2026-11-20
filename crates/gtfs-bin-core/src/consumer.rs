use std::str;

pub use memmap2::Advice;
use memmap2::Mmap;

use crate::{
    GTFS_BIN_VERSION,
    consumer::reader::Reader,
    models::{
        self, Date, Header, Route, RouteIndex, SearchStop, Sentinel, Service, ServiceIndex, Shape,
        Stop, StopCell, StopIndex, StopTime, Transfer, TransferIndex, TransferSlice, Trip,
        TripIndex, TripPattern, TripPatternIndex, TripPatternSlice, TripSlice,
    },
};

mod error;
mod grid;
mod reader;
mod routes;
mod search;
mod sections;
mod services;
mod shapes;
mod stop_times;
mod stops;
mod strings;
mod transfers;
mod trip_patterns;
mod trips;
mod validate;

pub use error::{Error, Result};

/// A read-only view over a compiled GTFS file.
///
/// Every table the file holds is a slice into the memory map, so nothing is copied.
///
/// After `new` succeeds, for any bytes it was given:
/// - Every id and string section is valid UTF-8, and every slice into one ends on a character.
/// - Each lookup table has one entry per item of the table it indexes.
/// - Every index and slice stored in a table points inside the table it names.
/// - Every item's own `index` is its position.
///
/// So the accessors can only panic on an index the caller made up.
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

    timezone: &'a [u8],
    stop_grid: &'a [StopCell],
    grid_stops: &'a [StopIndex],
    feed_start_date: Date,
    feed_end_date: Date,

    /// The whole file, for `verify_checksum`.
    bytes: &'a [u8],
    expected_checksum: u64,
}

impl<'a> Consumer<'a> {
    /// Maps the sections of a compiled file.
    ///
    /// # Errors
    ///
    /// Returns an error if the header is wrong, a section is out of bounds or misaligned, a text
    /// section is not UTF-8, or an index or slice inside a table points outside its target.
    pub fn new(mmap: &'a Mmap) -> Result<Self> {
        let consumer = Self::open(mmap)?;
        consumer.validate()?;
        Ok(consumer)
    }

    /// Maps the sections of a compiled file without checking what is inside them.
    ///
    /// This still checks the header, the length of the file and that every section lies inside it.
    /// What it skips is the check that the indices inside the tables point at real items, so a
    /// damaged file can make an accessor panic. It can never cause undefined behavior. Use it for a
    /// file this program wrote itself, and `new` for anything else.
    ///
    /// # Errors
    ///
    /// Returns an error if the header is wrong or a section is out of bounds or misaligned.
    pub fn new_unvalidated(mmap: &'a Mmap) -> Result<Self> {
        Self::open(mmap)
    }

    /// Checks that the file is the one the compiler wrote, byte for byte, by comparing a hash of
    /// everything after the header. It reads the whole file, so it costs about as much as copying
    /// it, and `new` doesn't do it.
    ///
    /// # Errors
    ///
    /// Returns an error if any byte after the header has changed.
    pub fn verify_checksum(&self) -> Result<()> {
        let body = &self.bytes[size_of::<Header>()..];
        if models::checksum(body) == self.expected_checksum {
            Ok(())
        } else {
            Err(Error::ChecksumMismatch)
        }
    }

    fn open(mmap: &'a Mmap) -> Result<Self> {
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

        if header.file_length != mmap.len() as u64 {
            return Err(Error::LengthMismatch {
                expected: header.file_length,
                actual: mmap.len() as u64,
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

            timezone: reader.bytes::<u8>(header.timezone)?,
            stop_grid: reader.cast_slice(header.stop_grid)?,
            grid_stops: reader.cast_slice(header.grid_stops)?,
            feed_start_date: header.feed_start_date,
            feed_end_date: header.feed_end_date,

            bytes: mmap,
            expected_checksum: header.checksum,
        };
        Ok(consumer)
    }

    /// Returns the agency's time zone, such as `Europe/Stockholm`, if the feed has one.
    #[must_use]
    pub fn timezone(&self) -> Option<&'a str> {
        let timezone = text_of(self.timezone);
        (!timezone.is_empty()).then_some(timezone)
    }

    /// Returns when the feed says it starts being valid, from `feed_info.txt`.
    #[must_use]
    pub fn feed_start_date(&self) -> Option<Date> {
        self.feed_start_date.as_option()
    }

    /// Returns when the feed says it stops being valid, from `feed_info.txt`.
    #[must_use]
    pub fn feed_end_date(&self) -> Option<Date> {
        self.feed_end_date.as_option()
    }

    /// Returns the first and last day any service runs, or `None` if there are no services. Unlike
    /// the feed's own dates, it can't be left out of the feed.
    #[must_use]
    pub fn service_range(&self) -> Option<(Date, Date)> {
        let first = self
            .services
            .iter()
            .map(|service| service.start_date)
            .min()?;
        let last = self.services.iter().map(|service| service.end_date).max()?;
        Some((first, last))
    }
}

/// Opens the file for the operating system to read ahead or not, as a router that jumps around
/// the tables wants.
///
/// # Errors
///
/// Returns an error if the operating system refuses the advice.
#[cfg(unix)]
pub fn advise(mmap: &Mmap, advice: Advice) -> Result<()> {
    Ok(mmap.advise(advice)?)
}

/// Returns a whole validated text section.
fn text_of(bytes: &[u8]) -> &str {
    str::from_utf8(bytes).expect("a text section is checked to be utf-8 when the file is opened")
}

/// Returns the text of a slice of a validated section.
///
/// # Panics
///
/// Panics if the slice cuts a multi-byte character, which a file from the compiler never does.
fn text(bytes: &[u8]) -> &str {
    str::from_utf8(bytes).expect("the compiler only writes slices that end on character boundaries")
}
