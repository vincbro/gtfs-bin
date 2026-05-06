use crate::{
    GTFS_BIN_VERSION,
    compiler::{
        routes::{build_route_ids, build_route_to_trips, build_routes},
        services::{build_service_ids, build_services},
        shapes::build_shapes,
        stop_times::build_stop_times,
        stops::{build_stop_ids, build_stop_search, build_stop_to_trips, build_stops},
        transfers::build_transfers,
        trip_patterns::{BuildTripPatternsResult, build_trip_patterns},
        trips::{build_trip_ids, build_trips},
        writer::BinaryWriter,
    },
    models::{Header, SliceBuilder},
};
use bytemuck::bytes_of;
use gtfs_structures::RawGtfs;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

mod routes;
mod services;
mod shapes;
mod stop_times;
mod stops;
mod transfers;
mod trip_patterns;
mod trips;
mod writer;

/// Compiles the .gtfs file
#[derive(Debug, Default)]
pub struct Compiler {
    path: PathBuf,
}

impl Compiler {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn compile(&self) -> Result<Vec<u8>, gtfs_structures::Error> {
        let gtfs = RawGtfs::from_path(&self.path)?;
        let mut slice_builder = SliceBuilder::new();

        let raw_stops = gtfs.stops?;
        let (mut stops, stop_map) = build_stops(&raw_stops, &mut slice_builder);

        let raw_routes = gtfs.routes?;
        let (mut routes, route_map) = build_routes(&raw_routes, &mut slice_builder);

        let raw_calendar = gtfs.calendar.unwrap_or(Ok(vec![]))?;
        let raw_calendar_dates = gtfs.calendar_dates.unwrap_or(Ok(vec![]))?;
        let (mut services, service_map, active_mask) =
            build_services(&raw_calendar, &raw_calendar_dates);

        let (shapes, shape_map) = if let Some(raw_shapes) = gtfs.shapes {
            let raw_shapes = raw_shapes?;
            build_shapes(&raw_shapes)
        } else {
            (vec![], HashMap::new())
        };

        let raw_trips = gtfs.trips?;
        let (mut trips, trip_map) = build_trips(
            &raw_trips,
            &route_map,
            &service_map,
            &shape_map,
            &mut slice_builder,
        );

        let raw_stop_times = gtfs.stop_times?;
        let stop_times = build_stop_times(
            &raw_stop_times,
            &mut trips,
            &trip_map,
            &stop_map,
            &mut slice_builder,
        );

        let (transfers, stop_to_transfers_out, transfers_in_indencies, stop_to_transfers_in) =
            if let Some(raw_transfers) = gtfs.transfers {
                let raw_transfers = raw_transfers?;
                build_transfers(&raw_transfers, &stop_map)
            } else {
                (vec![], vec![], vec![], vec![])
            };

        let (stop_to_trips, stop_to_trips_lookup) = build_stop_to_trips(&stops, &stop_times);
        let (search_stops, search_to_stops) = build_stop_search(&stops);
        let (route_to_trips, route_to_trips_lookup) = build_route_to_trips(&trips, &routes);
        let BuildTripPatternsResult(
            trip_patterns,
            stop_sequences,
            trips_in_sequences,
            trip_to_trip_pattern,
            stop_to_trip_pattern,
            stop_to_trip_pattern_lookup,
        ) = build_trip_patterns(&trips, &stops, &stop_times);
        let (stop_id_lookup, stop_ids) = build_stop_ids(&mut stops, &stop_map);
        let (route_id_lookup, route_ids) = build_route_ids(&mut routes, &route_map);
        let (trip_id_lookup, trip_ids) = build_trip_ids(&mut trips, &trip_map);
        let (service_id_lookup, service_ids) = build_service_ids(&mut services, &service_map);

        let mut writer = BinaryWriter::new().resize(size_of::<Header>());

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

            trip_patterns: writer.write_section(&trip_patterns),
            trip_patterns_stop_seq: writer.write_section(&stop_sequences),
            trip_patterns_trip_seq: writer.write_section(&trips_in_sequences),
            trip_to_trip_pattern: writer.write_section(&trip_to_trip_pattern),
            stop_to_trip_pattern: writer.write_section(&stop_to_trip_pattern),
            stop_to_trip_pattern_lookup: writer.write_section(&stop_to_trip_pattern_lookup),

            route_to_trips: writer.write_section(&route_to_trips),
            route_to_trips_lookup: writer.write_section(&route_to_trips_lookup),

            stop_to_trips: writer.write_section(&stop_to_trips),
            stop_to_trips_lookup: writer.write_section(&stop_to_trips_lookup),

            search_stops: writer.write_section(&search_stops),
            search_to_stops: writer.write_section(&search_to_stops),

            transfers: writer.write_section(&transfers),
            stop_to_transfers_out: writer.write_section(&stop_to_transfers_out),
            transfers_in_indencies: writer.write_section(&transfers_in_indencies),
            stop_to_transfers_in: writer.write_section(&stop_to_transfers_in),

            strings: writer.write_section(slice_builder.take().as_bytes()),
        };
        writer.overwrite(0, bytes_of(&header));

        Ok(writer.take())
    }
}
