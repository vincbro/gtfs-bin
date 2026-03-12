use crate::{
    GTFS_BIN_VERSION,
    compiler::{
        routes::{build_route_ids, build_route_to_trips, build_routes},
        stops::{build_stop_ids, build_stop_to_trips, build_stops},
        stoptimes::build_stop_times,
        trips::{build_trip_ids, build_trips},
        writer::BinaryWriter,
    },
    models::{Header, SliceBuilder},
};
use bytemuck::bytes_of;
use gtfs_structures::RawGtfs;
use std::path::{Path, PathBuf};

mod routes;
mod stops;
mod stoptimes;
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
        let (mut stops, stop_map) = build_stops(&raw_stops, &mut slice_builder)?;
        let raw_routes = gtfs.routes?;
        let (mut routes, route_map) = build_routes(&raw_routes, &mut slice_builder)?;
        let raw_trips = gtfs.trips?;
        let (mut trips, trip_map) = build_trips(&raw_trips, &route_map, &mut slice_builder)?;
        let raw_stop_times = gtfs.stop_times?;
        let (stop_times, trip_to_stop_times) =
            build_stop_times(&raw_stop_times, &trip_map, &stop_map, &mut slice_builder)?;

        let (stop_to_trips, stop_to_trips_lookup) = build_stop_to_trips(&stops, &stop_times);
        let (route_to_trips, route_to_trips_lookup) = build_route_to_trips(&trips, &routes);

        let (stop_id_lookup, stop_ids) = build_stop_ids(&mut stops, &stop_map);
        let (route_id_lookup, route_ids) = build_route_ids(&mut routes, &route_map);
        let (trip_id_lookup, trip_ids) = build_trip_ids(&mut trips, &trip_map);

        let mut writer = BinaryWriter::new().resize(size_of::<Header>());

        let header = Header {
            magic: *b"GTFS",
            version: GTFS_BIN_VERSION,
            stops: writer.write_section(&stops),
            stop_ids: writer.write_section(stop_ids.as_bytes()),
            stop_id_lookup: writer.write_section(&stop_id_lookup),
            routes: writer.write_section(&routes),
            route_ids: writer.write_section(route_ids.as_bytes()),
            route_id_lookup: writer.write_section(&route_id_lookup),
            trips: writer.write_section(&trips),
            trip_ids: writer.write_section(trip_ids.as_bytes()),
            trip_id_lookup: writer.write_section(&trip_id_lookup),
            stop_times: writer.write_section(&stop_times),
            route_to_trips: writer.write_section(&route_to_trips),
            route_to_trips_lookup: writer.write_section(&route_to_trips_lookup),
            stop_to_trips: writer.write_section(&stop_to_trips),
            stop_to_trips_lookup: writer.write_section(&stop_to_trips_lookup),
            trip_to_stop_times: writer.write_section(&trip_to_stop_times),
            transfers: writer.write_section(&[0_u8; 8]),
            calendars: writer.write_section(&[0_u8; 8]),
        };
        writer.overwrite(0, bytes_of(&header));

        Ok(writer.take())
    }
}
