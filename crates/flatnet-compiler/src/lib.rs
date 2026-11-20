//! Compiles a transit network into the one file `flatnet-core` maps and reads.
//!
//! A reader turns a source into a [`Network`], and [`Compiler`] turns a `Network` into the file:
//! it turns every id into an index, groups the trips into patterns, builds the lookups and the
//! indexes, and writes the tables in the layout `docs/FORMAT.md` describes. The same network always
//! compiles to the same bytes.
//!
//! The `gtfs` feature, on by default, adds [`gtfs::Reader`], which reads a GTFS feed.

#[cfg(feature = "gtfs")]
use std::path::Path;

use flatnet_core::{
    FLATNET_VERSION,
    models::{self, Date, Header, Sentinel, SliceBuilder, StringSlice},
};
use metadata::MetadataInput;
use services::BuiltServices;
use shapes::BuiltShapes;
use transfers::BuiltTransfers;
use trip_patterns::TripPatterns;
use writer::BinaryWriter;

mod error;
mod grid;
mod metadata;
mod network;
mod routes;
mod services;
mod shapes;
mod stop_times;
mod stops;
mod tables;
mod transfers;
mod trip_patterns;
mod trips;
mod writer;

#[cfg(feature = "gtfs")]
pub mod gtfs;

pub use error::{Error, Result};
pub use network::{
    Call, Network, NetworkInfo, RouteRecord, ServiceRecord, StopRecord, TransferRecord, TripRecord,
    Weekly,
};

/// Compiles a [`Network`] into the binary format `Consumer` reads.
///
/// The same network always compiles to the same bytes.
#[derive(Debug, Default)]
pub struct Compiler {
    metadata: MetadataInput,
}

impl Compiler {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records when the file is compiled, in seconds since the unix epoch.
    ///
    /// The compiler never reads the clock itself, because the same feed has to give the same
    /// bytes. Without this the file says it was compiled at 0, which means it doesn't know.
    #[must_use]
    pub const fn compiled_at(mut self, unix_seconds: u64) -> Self {
        self.metadata.compiled_at = unix_seconds;
        self
    }

    /// Records the name of the feed, such as `sl.zip`, in the file.
    ///
    /// The compiler doesn't take it from the path, because the same feed has to give the same bytes
    /// whether it is a folder or a zip, and wherever it is.
    #[must_use]
    pub fn source(mut self, name: String) -> Self {
        self.metadata.source = Some(name);
        self
    }

    /// Puts a JSON object in the file, for `Consumer::metadata_json` to give back.
    ///
    /// It is checked when compiling, and written compactly with its keys sorted. Entries from
    /// `metadata_entry` are added to it, and replace a key with the same name.
    #[must_use]
    pub fn metadata_json(mut self, json: String) -> Self {
        self.metadata.json = Some(json);
        self
    }

    /// Adds one key and its text value to the JSON metadata.
    #[must_use]
    pub fn metadata_entry(mut self, key: String, value: String) -> Self {
        self.metadata.entries.insert(key, value);
        self
    }

    /// Compiles the network into the file.
    ///
    /// # Errors
    ///
    /// Returns an error if the network has two things of one kind with the same id, a reference
    /// that points at nothing, a service with no days or one that runs for too long, or metadata
    /// that is not a JSON object.
    #[allow(
        clippy::too_many_lines,
        reason = "one pass that builds each table and then lists every section in file order"
    )]
    pub fn compile(&self, mut network: Network) -> Result<Vec<u8>> {
        network.check()?;
        let mut slice_builder = SliceBuilder::new();

        let (mut stops, stop_infos) = stops::build_stops(&network.stops, &mut slice_builder);
        let mut routes = routes::build_routes(&network.routes, &mut slice_builder);
        let BuiltServices {
            mut services,
            active_mask,
        } = services::build_services(&network.services)?;
        let (mut trips, trip_infos) = trips::build_trips(&network.trips, &mut slice_builder);
        let headsigns: Vec<StringSlice> = network
            .headsigns
            .iter()
            .map(|headsign| slice_builder.add(headsign))
            .collect();

        stop_times::prepare_calls(&mut network.calls, &mut trips);

        let trip_lines: Vec<Option<usize>> = network.trips.iter().map(|trip| trip.line).collect();
        let BuiltShapes {
            shapes,
            points: shape_points,
            trip_shapes,
        } = shapes::build_trip_shapes(
            &mut trips,
            &trip_lines,
            &network.lines,
            &stops,
            &network.calls,
        );

        // Built even when the network has no transfers, because the consumer indexes the lookups
        // by stop.
        let BuiltTransfers {
            transfers,
            outbound: stop_to_transfers_out,
            inbound_indices: transfers_in_indices,
            inbound: stop_to_transfers_in,
        } = transfers::build_transfers(&network.transfers, stops.len());

        let timezone = network.info.timezone.clone().unwrap_or_default();
        let (metadata, metadata_json) =
            metadata::build_metadata(&self.metadata, &network.info, &services, &mut slice_builder)?;

        let (search_stops, search_to_stops) = stops::build_stop_search(&stops, &stop_infos);
        let (stop_grid, grid_stops) = grid::build_stop_grid(&stops);
        let (route_to_trips, route_to_trips_lookup) = routes::build_route_to_trips(&trips, &routes);
        let trip_patterns =
            trip_patterns::build_trip_patterns(&mut trips, &stops, &network.calls, &headsigns);
        let (stop_id_lookup, stop_ids) = stops::build_stop_ids(&mut stops, &network.stops);
        let (route_id_lookup, route_ids) = routes::build_route_ids(&mut routes, &network.routes);
        let (trip_id_lookup, trip_ids) = trips::build_trip_ids(&mut trips, &network.trips);
        let (service_id_lookup, service_ids) =
            services::build_service_ids(&mut services, &network.services);

        let TripPatterns {
            patterns,
            stop_sequences,
            pattern_stops,
            trips_in_patterns,
            trip_to_pattern,
            stop_to_pattern,
            stop_to_pattern_lookup,
            timings,
        } = trip_patterns;

        let mut writer = BinaryWriter::new().with_reserved(size_of::<Header>());

        let mut header = Header {
            magic: *b"FLNT",
            version: FLATNET_VERSION,
            stops: writer.write_section(&stops),
            stop_infos: writer.write_section(&stop_infos),
            stop_ids: writer.write_section(stop_ids.as_bytes()),
            stops_id_lookup: writer.write_section(&stop_id_lookup),

            routes: writer.write_section(&routes),
            route_ids: writer.write_section(route_ids.as_bytes()),
            route_id_lookup: writer.write_section(&route_id_lookup),

            trips: writer.write_section(&trips),
            trip_infos: writer.write_section(&trip_infos),
            trip_ids: writer.write_section(trip_ids.as_bytes()),
            trip_id_lookup: writer.write_section(&trip_id_lookup),

            services: writer.write_section(&services),
            service_ids: writer.write_section(service_ids.as_bytes()),
            service_id_lookup: writer.write_section(&service_id_lookup),
            active_mask: writer.write_section(&active_mask.into_vec()),

            timings: writer.write_section(&timings),

            shapes: writer.write_section(&shapes),
            shape_points: writer.write_section(&shape_points),
            trip_shapes: writer.write_section(&trip_shapes),

            trip_patterns: writer.write_section(&patterns),
            trip_patterns_stop_seq: writer.write_section(&stop_sequences),
            pattern_stops: writer.write_section(&pattern_stops),
            trip_patterns_trip_seq: writer.write_section(&trips_in_patterns),
            trip_to_trip_pattern: writer.write_section(&trip_to_pattern),
            stop_to_trip_pattern: writer.write_section(&stop_to_pattern),
            stop_to_trip_pattern_lookup: writer.write_section(&stop_to_pattern_lookup),

            route_to_trips: writer.write_section(&route_to_trips),
            route_to_trips_lookup: writer.write_section(&route_to_trips_lookup),

            search_stops: writer.write_section(&search_stops),
            search_to_stops: writer.write_section(&search_to_stops),

            transfers: writer.write_section(&transfers),
            stop_to_transfers_out: writer.write_section(&stop_to_transfers_out),
            transfers_in_indices: writer.write_section(&transfers_in_indices),
            stop_to_transfers_in: writer.write_section(&stop_to_transfers_in),

            strings: writer.write_section(slice_builder.into_buffer().as_bytes()),

            timezone: writer.write_section(timezone.as_bytes()),
            stop_grid: writer.write_section(&stop_grid),
            grid_stops: writer.write_section(&grid_stops),

            metadata: writer.write_section(&[metadata]),
            metadata_json: writer.write_section(metadata_json.as_bytes()),

            feed_start_date: network.info.valid_from.unwrap_or(Date::NONE),
            feed_end_date: network.info.valid_until.unwrap_or(Date::NONE),
            file_length: 0,
            checksum: 0,
        };
        header.file_length = writer.as_bytes().len() as u64;
        header.checksum = models::checksum(&writer.as_bytes()[size_of::<Header>()..]);
        writer.overwrite(0, bytemuck::bytes_of(&header));

        Ok(writer.into_bytes())
    }

    /// Reads the GTFS feed at `path`, a directory or a zip, and compiles it.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`gtfs::Reader::read`] and of [`Compiler::compile`].
    #[cfg(feature = "gtfs")]
    pub fn compile_gtfs(&self, path: &Path) -> Result<Vec<u8>> {
        self.compile(gtfs::Reader::new(path).read()?)
    }
}
