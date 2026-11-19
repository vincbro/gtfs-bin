use crate::{
    GTFS_BIN_VERSION,
    models::{Header, SliceBuilder},
};
use gtfs_structures::RawGtfs;
use std::path::{Path, PathBuf};

use transfers::BuiltTransfers;
use trip_patterns::TripPatterns;
use writer::BinaryWriter;

mod error;
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

pub use error::{Error, Result};

/// Compiles a GTFS feed into the binary format `Consumer` reads.
///
/// The same feed always compiles to the same bytes.
#[derive(Debug, Default)]
pub struct Compiler {
    path: PathBuf,
}

impl Compiler {
    #[must_use]
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
        }
    }

    /// Reads the feed and returns the compiled file.
    ///
    /// # Errors
    ///
    /// Returns an error if the feed can't be read or a required file is missing or malformed.
    pub fn compile(&self) -> Result<Vec<u8>> {
        let gtfs = RawGtfs::from_path(&self.path)?;
        let mut slice_builder = SliceBuilder::new();

        let raw_stops = gtfs.stops?;
        let (mut stops, stop_map) = stops::build_stops(&raw_stops, &mut slice_builder);

        let raw_routes = gtfs.routes?;
        let (mut routes, route_map) = routes::build_routes(&raw_routes, &mut slice_builder);

        let raw_calendar = gtfs.calendar.unwrap_or(Ok(vec![]))?;
        let raw_calendar_dates = gtfs.calendar_dates.unwrap_or(Ok(vec![]))?;
        let (mut services, service_map, active_mask) =
            services::build_services(&raw_calendar, &raw_calendar_dates);

        let (shapes, shape_map) = gtfs
            .shapes
            .transpose()?
            .map(|raw_shapes| shapes::build_shapes(&raw_shapes))
            .unwrap_or_default();

        let raw_trips = gtfs.trips?;
        let (mut trips, trip_map) = trips::build_trips(
            &raw_trips,
            &route_map,
            &service_map,
            &shape_map,
            &mut slice_builder,
        );

        let raw_stop_times = gtfs.stop_times?;
        let stop_times = stop_times::build_stop_times(
            &raw_stop_times,
            &mut trips,
            &trip_map,
            &stop_map,
            &mut slice_builder,
        );

        // Built even when the feed has no transfers, because the consumer indexes the lookups by
        // stop.
        let raw_transfers = gtfs.transfers.unwrap_or(Ok(vec![]))?;
        let BuiltTransfers {
            transfers,
            outbound: stop_to_transfers_out,
            inbound_indices: transfers_in_indices,
            inbound: stop_to_transfers_in,
        } = transfers::build_transfers(&raw_transfers, &stop_map);

        let (stop_to_trips, stop_to_trips_lookup) = stops::build_stop_to_trips(&stops, &stop_times);
        let (search_stops, search_to_stops) = stops::build_stop_search(&stops);
        let (route_to_trips, route_to_trips_lookup) = routes::build_route_to_trips(&trips, &routes);
        let trip_patterns = trip_patterns::build_trip_patterns(&trips, &stops, &stop_times);
        let (stop_id_lookup, stop_ids) = stops::build_stop_ids(&mut stops, &stop_map);
        let (route_id_lookup, route_ids) = routes::build_route_ids(&mut routes, &route_map);
        let (trip_id_lookup, trip_ids) = trips::build_trip_ids(&mut trips, &trip_map);
        let (service_id_lookup, service_ids) =
            services::build_service_ids(&mut services, &service_map);

        let TripPatterns {
            patterns,
            stop_sequences,
            trips_in_patterns,
            trip_to_pattern,
            stop_to_pattern,
            stop_to_pattern_lookup,
        } = trip_patterns;

        let mut writer = BinaryWriter::new().with_reserved(size_of::<Header>());

        let header = Header {
            magic: *b"GTFS",
            version: GTFS_BIN_VERSION,
            stops: writer.write_section(&stops),
            stop_ids: writer.write_section(stop_ids.as_bytes()),
            stops_id_lookup: writer.write_section(&stop_id_lookup),

            routes: writer.write_section(&routes),
            route_ids: writer.write_section(route_ids.as_bytes()),
            route_id_lookup: writer.write_section(&route_id_lookup),

            trips: writer.write_section(&trips),
            trip_ids: writer.write_section(trip_ids.as_bytes()),
            trip_id_lookup: writer.write_section(&trip_id_lookup),

            services: writer.write_section(&services),
            service_ids: writer.write_section(service_ids.as_bytes()),
            service_id_lookup: writer.write_section(&service_id_lookup),
            active_mask: writer.write_section(&active_mask.into_vec()),

            stop_times: writer.write_section(&stop_times),

            shapes: writer.write_section(&shapes),

            trip_patterns: writer.write_section(&patterns),
            trip_patterns_stop_seq: writer.write_section(&stop_sequences),
            trip_patterns_trip_seq: writer.write_section(&trips_in_patterns),
            trip_to_trip_pattern: writer.write_section(&trip_to_pattern),
            stop_to_trip_pattern: writer.write_section(&stop_to_pattern),
            stop_to_trip_pattern_lookup: writer.write_section(&stop_to_pattern_lookup),

            route_to_trips: writer.write_section(&route_to_trips),
            route_to_trips_lookup: writer.write_section(&route_to_trips_lookup),

            stop_to_trips: writer.write_section(&stop_to_trips),
            stop_to_trips_lookup: writer.write_section(&stop_to_trips_lookup),

            search_stops: writer.write_section(&search_stops),
            search_to_stops: writer.write_section(&search_to_stops),

            transfers: writer.write_section(&transfers),
            stop_to_transfers_out: writer.write_section(&stop_to_transfers_out),
            transfers_in_indices: writer.write_section(&transfers_in_indices),
            stop_to_transfers_in: writer.write_section(&stop_to_transfers_in),

            strings: writer.write_section(slice_builder.into_buffer().as_bytes()),
        };
        writer.overwrite(0, bytemuck::bytes_of(&header));

        Ok(writer.into_bytes())
    }
}
