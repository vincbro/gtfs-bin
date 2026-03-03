use crate::{
    GTFS_BIN_VERSION,
    consumer::{Header, Section},
    models::{
        Coordinate, Distance, Opt, Route, RouteIdx, Sentinel, ServiceIdx, Slice, SliceBuilder,
        Stop, StopIdx, StopTime, StopTimeIdx, StopTimeSlice, StringSlice, Time, Trip, TripIdx,
        TripSlice,
    },
};
use gtfs_structures::RawGtfs;
use rayon::slice::ParallelSliceMut;
use std::path::{Path, PathBuf};

/// Builds the .gtfs file
#[derive(Debug, Default)]
pub struct Builder {
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

impl Builder {
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

    pub fn build(mut self) -> Result<Vec<u8>, gtfs_structures::Error> {
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

        // Write Header
        binary_data.extend_from_slice(bytemuck::bytes_of(&header));

        // Write Stops
        binary_data.extend_from_slice(bytemuck::cast_slice(&self.stops));
        binary_data.extend_from_slice(self.stop_ids.as_bytes());

        // Write Routes
        binary_data.extend_from_slice(bytemuck::cast_slice(&self.routes));
        binary_data.extend_from_slice(self.route_ids.as_bytes());

        // Write Trips
        binary_data.extend_from_slice(bytemuck::cast_slice(&self.trips));
        binary_data.extend_from_slice(self.trip_ids.as_bytes());

        // Write Stop Times
        binary_data.extend_from_slice(bytemuck::cast_slice(&self.stop_times));

        // Write Lookups
        binary_data.extend_from_slice(bytemuck::cast_slice(&self.route_to_trips));
        binary_data.extend_from_slice(bytemuck::cast_slice(&self.route_to_trips_lookup));
        binary_data.extend_from_slice(bytemuck::cast_slice(&self.stop_to_trips));
        binary_data.extend_from_slice(bytemuck::cast_slice(&self.stop_to_trips_lookup));
        binary_data.extend_from_slice(bytemuck::cast_slice(&self.trip_to_stop_times));

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
            let section = Section {
                offset: current_offset,
                count: count as u64,
            };
            // Ensure we step forward by the total byte size of the array
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

    fn build_stops(
        &mut self,
        raw_stops: &[gtfs_structures::Stop],
        slice_builder: &mut SliceBuilder<StringSlice>,
    ) -> Result<(), gtfs_structures::Error> {
        // Recomended max length for a gtfs id is 36 characters
        let mut id_builder = SliceBuilder::with_capacity(36 * raw_stops.len());

        // Convert raw_stops to stops
        let mut stops: Vec<_> = raw_stops
            .iter()
            .enumerate()
            .map(|(i, stop)| {
                let coordinate = if let Some(lat) = stop.latitude
                    && let Some(lon) = stop.longitude
                {
                    Opt::new(Coordinate::new(lat, lon))
                } else {
                    Opt::new(Coordinate::NONE)
                };
                Stop {
                    coordinate,
                    id: id_builder.add(&stop.id),
                    code: stop
                        .code
                        .as_ref()
                        .map(|code| slice_builder.add(code))
                        .into(),
                    name: stop
                        .name
                        .as_ref()
                        .map(|name| slice_builder.add(name))
                        .into(),
                    description: stop
                        .description
                        .as_ref()
                        .map(|desc| slice_builder.add(desc))
                        .into(),
                    idx: StopIdx(i as u32),
                    parent_idx: Opt::new(StopIdx::NONE),
                }
            })
            .collect();

        let stop_ids = id_builder.take();

        // Build binary search friendly id lookup
        let mut stop_id_lookup: Vec<_> = (0..stops.len()).map(|i| StopIdx(i as u32)).collect();
        stop_id_lookup.par_sort_unstable_by(|a, b| {
            let id_a = &stop_ids[stops[a.to_usize()].id.range()];
            let id_b = &stop_ids[stops[b.to_usize()].id.range()];
            id_a.cmp(id_b)
        });

        // Map parent stations
        raw_stops
            .iter()
            .enumerate()
            .filter_map(|(i, raw_stop)| raw_stop.parent_station.clone().map(|pt_id| (i, pt_id)))
            .for_each(|(i, pt_id)| {
                let result = stop_id_lookup.binary_search_by(|&idx| {
                    let stop = &stops[idx.to_usize()];
                    let current_id = &stop_ids[stop.id.range()];
                    current_id.cmp(&pt_id)
                });

                if let Ok(idx) = result {
                    stops[i].parent_idx = Opt::new(StopIdx(idx as u32));
                }
            });

        self.stops = stops;
        self.stop_id_lookup = stop_id_lookup;
        self.stop_ids = stop_ids;

        Ok(())
    }

    fn build_routes(
        &mut self,
        raw_routes: &[gtfs_structures::Route],
        slice_builder: &mut SliceBuilder<StringSlice>,
    ) -> Result<(), gtfs_structures::Error> {
        // Recomended max length for a gtfs id is 36 characters
        let mut id_builder = SliceBuilder::with_capacity(36 * raw_routes.len());

        let routes: Vec<_> = raw_routes
            .iter()
            .enumerate()
            .map(|(i, route)| Route {
                id: id_builder.add(&route.id),
                long_name: route
                    .long_name
                    .as_ref()
                    .map(|ln| slice_builder.add(ln))
                    .into(),
                short_name: route
                    .short_name
                    .as_ref()
                    .map(|sn| slice_builder.add(sn))
                    .into(),
                description: route
                    .desc
                    .as_ref()
                    .map(|desc| slice_builder.add(desc))
                    .into(),
                idx: RouteIdx(i as u32),
                rtype: 0,
            })
            .collect();

        let route_ids = id_builder.take();

        // Build binary search friendly id lookup
        let mut route_id_lookup: Vec<_> = (0..routes.len()).map(|i| RouteIdx(i as u32)).collect();
        route_id_lookup.par_sort_unstable_by(|a, b| {
            let id_a = &route_ids[routes[a.to_usize()].id.range()];
            let id_b = &route_ids[routes[b.to_usize()].id.range()];
            id_a.cmp(id_b)
        });

        self.routes = routes;
        self.route_id_lookup = route_id_lookup;
        self.route_ids = route_ids;

        Ok(())
    }

    fn build_trips(
        &mut self,
        raw_trips: &[gtfs_structures::RawTrip],
        slice_builder: &mut SliceBuilder<StringSlice>,
    ) -> Result<(), gtfs_structures::Error> {
        // Recomended max length for a gtfs id is 36 characters
        let mut id_builder = SliceBuilder::with_capacity(36 * raw_trips.len());

        let trips: Vec<_> = raw_trips
            .iter()
            .enumerate()
            .filter_map(|(i, trip)| {
                let result = self.route_id_lookup.binary_search_by(|&idx| {
                    let stop = &self.routes[idx.to_usize()];
                    let current_id = &self.route_ids[stop.id.range()];
                    current_id.cmp(&trip.route_id)
                });
                result.ok().map(|idx| (i, trip, RouteIdx(idx as u32)))
            })
            .map(|(i, trip, route_idx)| Trip {
                id: id_builder.add(&trip.id),
                idx: TripIdx(i as u32),
                headsign: trip
                    .trip_headsign
                    .as_ref()
                    .map(|hs| slice_builder.add(hs))
                    .into(),
                route_idx,
                service_idx: ServiceIdx::NONE,
                short_name: trip
                    .trip_short_name
                    .as_ref()
                    .map(|sn| slice_builder.add(sn))
                    .into(),
            })
            .collect();

        let trip_ids = id_builder.take();

        // Build binary search friendly id lookup
        let mut trip_id_lookup: Vec<_> = (0..trips.len()).map(|i| TripIdx(i as u32)).collect();
        trip_id_lookup.par_sort_unstable_by(|a, b| {
            let id_a = &trip_ids[trips[a.to_usize()].id.range()];
            let id_b = &trip_ids[trips[b.to_usize()].id.range()];
            id_a.cmp(id_b)
        });

        self.trips = trips;
        self.trip_id_lookup = trip_id_lookup;
        self.trip_ids = trip_ids;
        Ok(())
    }

    fn build_stop_times(
        &mut self,
        raw_stop_times: &[gtfs_structures::RawStopTime],
        slice_builder: &mut SliceBuilder<StringSlice>,
    ) -> Result<(), gtfs_structures::Error> {
        let mut stop_times: Vec<_> = raw_stop_times
            .iter()
            .filter_map(|stop_time| {
                let result = self.trip_id_lookup.binary_search_by(|&idx| {
                    let trip = &self.trips[idx.to_usize()];
                    let current_id = &self.trip_ids[trip.id.range()];
                    current_id.cmp(&stop_time.trip_id)
                });
                result.ok().map(|idx| (stop_time, TripIdx(idx as u32)))
            })
            .filter_map(|(stop_time, trip_idx)| {
                let result = self.stop_id_lookup.binary_search_by(|&idx| {
                    let stop = &self.stops[idx.to_usize()];
                    let current_id = &self.stop_ids[stop.id.range()];
                    current_id.cmp(&stop_time.stop_id)
                });
                result
                    .ok()
                    .map(|idx| (stop_time, trip_idx, StopIdx(idx as u32)))
            })
            .map(|(stop_time, trip_idx, stop_idx)| StopTime {
                idx: StopTimeIdx::NONE,
                headsign: stop_time
                    .stop_headsign
                    .as_ref()
                    .map(|hs| slice_builder.add(hs))
                    .into(),
                stop_idx,
                trip_idx,
                sequence: stop_time.stop_sequence,
                arrival_time: Opt::new(Time::NONE),
                departure_time: Opt::new(Time::NONE),
                distance_traveled: Opt::new(Distance::NONE),
            })
            .collect();

        stop_times.par_sort_unstable_by(|a, b| {
            a.trip_idx
                .cmp(&b.trip_idx)
                .then(a.sequence.cmp(&b.sequence))
        });

        let mut trip_to_stop_times = vec![StopTimeSlice::NONE; self.trips.len()];
        let mut trip_idx = TripIdx::NONE;
        let mut start: u32 = u32::MAX;
        let mut count: u32 = 0;
        for (i, stop_times) in stop_times.iter_mut().enumerate() {
            if stop_times.trip_idx != trip_idx {
                if trip_idx != TripIdx::NONE {
                    trip_to_stop_times[trip_idx.to_usize()] = StopTimeSlice { start, count }
                }
                start = i as u32;
                count = 0;
                trip_idx = stop_times.trip_idx;
            }

            stop_times.idx = StopTimeIdx(i as u32);
            count += 1;
        }
        if trip_idx != TripIdx::NONE {
            trip_to_stop_times[trip_idx.to_usize()] = StopTimeSlice { start, count };
        }

        self.stop_times = stop_times;
        self.trip_to_stop_times = trip_to_stop_times;
        Ok(())
    }

    fn build_route_to_trips(&mut self) {
        let mut route_trip_pairs: Vec<(RouteIdx, TripIdx)> = self
            .trips
            .iter()
            .map(|trip| (trip.route_idx, trip.idx))
            .collect();

        route_trip_pairs.par_sort_unstable_by_key(|&(route_idx, _)| route_idx);

        let mut route_to_trips = vec![TripSlice::NONE; self.routes.len()];
        let mut route_to_trips_lookup = Vec::with_capacity(route_trip_pairs.len());

        let mut current_route = RouteIdx::NONE;
        let mut start = 0;
        let mut count = 0;

        for (i, &(route_idx, trip_idx)) in route_trip_pairs.iter().enumerate() {
            if route_idx != current_route {
                if current_route != RouteIdx::NONE {
                    route_to_trips[current_route.to_usize()] = TripSlice { start, count };
                }
                start = i as u32;
                count = 0;
                current_route = route_idx;
            }
            route_to_trips_lookup.push(trip_idx);
            count += 1;
        }

        if current_route != RouteIdx::NONE {
            route_to_trips[current_route.to_usize()] = TripSlice { start, count };
        }

        self.route_to_trips = route_to_trips;
        self.route_to_trips_lookup = route_to_trips_lookup;
    }

    fn build_stop_to_trips(&mut self) {
        let mut stop_trip_pairs: Vec<(StopIdx, TripIdx)> = self
            .stop_times
            .iter()
            .map(|st| (st.stop_idx, st.trip_idx))
            .collect();

        stop_trip_pairs.par_sort_unstable();

        stop_trip_pairs.dedup();

        let mut stop_to_trips = vec![TripSlice::NONE; self.stops.len()];
        let mut stop_to_trips_lookup = Vec::with_capacity(stop_trip_pairs.len());

        let mut current_stop = StopIdx::NONE;
        let mut start = 0;
        let mut count = 0;

        for (i, &(stop_idx, trip_idx)) in stop_trip_pairs.iter().enumerate() {
            if stop_idx != current_stop {
                if current_stop != StopIdx::NONE {
                    stop_to_trips[current_stop.to_usize()] = TripSlice { start, count };
                }
                start = i as u32;
                count = 0;
                current_stop = stop_idx;
            }
            stop_to_trips_lookup.push(trip_idx);
            count += 1;
        }

        if current_stop != StopIdx::NONE {
            stop_to_trips[current_stop.to_usize()] = TripSlice { start, count };
        }

        self.stop_to_trips = stop_to_trips;
        self.stop_to_trips_lookup = stop_to_trips_lookup;
    }
}
