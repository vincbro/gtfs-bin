use crate::models::SliceBuilder;
use crate::{
    GTFS_BIN_VERSION,
    consumer::{Header, Section},
    models::{Route, RouteIdx, Stop, StopIdx, StopTime, StopTimeSlice, Trip, TripIdx, TripSlice},
};
use gtfs_structures::RawGtfs;
use std::path::{Path, PathBuf};

mod routes;
mod stops;
mod stoptimes;
mod trips;

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

        let header = self.build_header();

        let total_bytes = header.trip_to_stop_times.offset
            + (header.trip_to_stop_times.count * std::mem::size_of::<StopTimeSlice>() as u64);

        let mut binary_data = Vec::with_capacity(total_bytes as usize);

        let mut append = |slice: &[u8]| {
            let remainder = binary_data.len() % 8;
            if remainder != 0 {
                binary_data.resize(binary_data.len() + (8 - remainder), 0);
            }
            binary_data.extend_from_slice(slice);
        };

        // Write Header
        append(bytemuck::bytes_of(&header));

        // Write Stops
        append(bytemuck::cast_slice(&self.stops));
        append(self.stop_ids.as_bytes());
        append(bytemuck::cast_slice(&self.stop_id_lookup));

        // Write Routes
        append(bytemuck::cast_slice(&self.routes));
        append(self.route_ids.as_bytes());
        append(bytemuck::cast_slice(&self.route_id_lookup));

        // Write Trips
        append(bytemuck::cast_slice(&self.trips));
        append(self.trip_ids.as_bytes());
        append(bytemuck::cast_slice(&self.trip_id_lookup));

        // Write Stop Times
        append(bytemuck::cast_slice(&self.stop_times));

        // Write Lookups
        append(bytemuck::cast_slice(&self.route_to_trips));
        append(bytemuck::cast_slice(&self.route_to_trips_lookup));
        append(bytemuck::cast_slice(&self.stop_to_trips));
        append(bytemuck::cast_slice(&self.stop_to_trips_lookup));
        append(bytemuck::cast_slice(&self.trip_to_stop_times));

        Ok(binary_data)
    }

    fn build_header(&self) -> Header {
        let mut header = Header {
            magic: *b"GTFS",
            version: GTFS_BIN_VERSION,
            ..Default::default()
        };

        let mut current_offset = std::mem::size_of::<Header>() as u64;

        // A quick helper closure to fill a section and bump the offset
        let mut add_section = |count: usize, element_size: usize| -> Section {
            let remainder = current_offset % 8;
            if remainder != 0 {
                current_offset += 8 - remainder;
            }

            let section = Section {
                offset: current_offset,
                count: count as u64,
            };

            current_offset += (count * element_size) as u64;
            section
        };

        // Stops
        header.stops = add_section(self.stops.len(), std::mem::size_of::<Stop>());
        header.stop_ids = add_section(self.stop_ids.len(), std::mem::size_of::<u8>());
        header.stop_id_lookup =
            add_section(self.stop_id_lookup.len(), std::mem::size_of::<StopIdx>());

        // Routes
        header.routes = add_section(self.routes.len(), std::mem::size_of::<Route>());
        header.route_ids = add_section(self.route_ids.len(), std::mem::size_of::<u8>());
        header.route_id_lookup =
            add_section(self.route_id_lookup.len(), std::mem::size_of::<RouteIdx>());

        // Trips
        header.trips = add_section(self.trips.len(), std::mem::size_of::<Trip>());
        header.trip_ids = add_section(self.trip_ids.len(), std::mem::size_of::<u8>());
        header.trip_id_lookup =
            add_section(self.trip_id_lookup.len(), std::mem::size_of::<TripIdx>());

        // Stop Times
        header.stop_times = add_section(self.stop_times.len(), std::mem::size_of::<StopTime>());

        // Lookups and Relationships
        header.route_to_trips =
            add_section(self.route_to_trips.len(), std::mem::size_of::<TripSlice>());
        header.route_to_trips_lookup = add_section(
            self.route_to_trips_lookup.len(),
            std::mem::size_of::<TripIdx>(),
        );

        header.stop_to_trips =
            add_section(self.stop_to_trips.len(), std::mem::size_of::<TripSlice>());
        header.stop_to_trips_lookup = add_section(
            self.stop_to_trips_lookup.len(),
            std::mem::size_of::<TripIdx>(),
        );

        header.trip_to_stop_times = add_section(
            self.trip_to_stop_times.len(),
            std::mem::size_of::<StopTimeSlice>(),
        );

        header
    }
}
