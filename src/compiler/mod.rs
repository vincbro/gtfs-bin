use crate::{
    GTFS_BIN_VERSION,
    compiler::writer::BinaryWriter,
    models::{
        Header, Route, RouteIdx, SliceBuilder, Stop, StopIdx, StopTime, StopTimeSlice, Trip,
        TripIdx, TripSlice,
    },
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

    stops: Vec<Stop>,
    stop_id_lookup: Vec<StopIdx>,
    stop_ids: String,

    routes: Vec<Route>,
    route_id_lookup: Vec<RouteIdx>,
    route_ids: String,

    trips: Vec<Trip>,
    trip_id_lookup: Vec<TripIdx>,
    trip_ids: String,

    stop_times: Vec<StopTime>,

    // One to many route -> trips
    route_to_trips: Vec<TripSlice>,
    route_to_trips_lookup: Vec<TripIdx>,

    // One to many stop -> trips
    stop_to_trips: Vec<TripSlice>,
    stop_to_trips_lookup: Vec<TripIdx>,

    // One to one slice trip -> stop times
    trip_to_stop_times: Vec<StopTimeSlice>,
}

impl Compiler {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            stops: vec![],
            stop_id_lookup: vec![],
            stop_ids: String::new(),
            routes: vec![],
            route_id_lookup: vec![],
            route_ids: String::new(),
            trips: vec![],
            trip_id_lookup: vec![],
            trip_ids: String::new(),
            stop_times: vec![],
            route_to_trips: vec![],
            route_to_trips_lookup: vec![],
            stop_to_trips: vec![],
            stop_to_trips_lookup: vec![],
            trip_to_stop_times: vec![],
        }
    }

    pub fn compile(mut self) -> Result<Vec<u8>, gtfs_structures::Error> {
        let gtfs = RawGtfs::from_path(&self.path)?;
        let mut slice_builder = SliceBuilder::new();

        let raw_stops = gtfs.stops?;
        self.build_stops(&raw_stops, &mut slice_builder)?;
        let raw_routes = gtfs.routes?;
        self.build_routes(&raw_routes, &mut slice_builder)?;
        let raw_trips = gtfs.trips?;
        self.build_trips(&raw_trips, &mut slice_builder)?;
        let raw_stop_times = gtfs.stop_times?;
        self.build_stop_times(&raw_stop_times, &mut slice_builder)?;

        self.build_stop_to_trips();
        self.build_route_to_trips();

        let mut writer = BinaryWriter::new().resize(size_of::<Header>());

        let header = Header {
            magic: *b"GTFS",
            version: GTFS_BIN_VERSION,
            stops: writer.write_section(&self.stops),
            stop_ids: writer.write_section(self.stop_ids.as_bytes()),
            stop_id_lookup: writer.write_section(&self.stop_id_lookup),
            routes: writer.write_section(&self.routes),
            route_ids: writer.write_section(self.route_ids.as_bytes()),
            route_id_lookup: writer.write_section(&self.route_id_lookup),
            trips: writer.write_section(&self.trips),
            trip_ids: writer.write_section(self.trip_ids.as_bytes()),
            trip_id_lookup: writer.write_section(&self.trip_id_lookup),
            stop_times: writer.write_section(&self.stop_times),
            route_to_trips: writer.write_section(&self.route_to_trips),
            route_to_trips_lookup: writer.write_section(&self.route_to_trips_lookup),
            stop_to_trips: writer.write_section(&self.stop_to_trips),
            stop_to_trips_lookup: writer.write_section(&self.stop_to_trips_lookup),
            trip_to_stop_times: writer.write_section(&self.trip_to_stop_times),
            transfers: writer.write_section(&[0_u8; 8]),
            calendars: writer.write_section(&[0_u8; 8]),
        };
        writer.overwrite(0, bytes_of(&header));

        Ok(writer.take())
    }
}
