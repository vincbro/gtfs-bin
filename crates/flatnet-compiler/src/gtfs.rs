//! Reads a GTFS feed, a directory or a zip, into a [`Network`].
//!
//! The reader has its own parser. It resolves every id to a position, and leaves out the rows that
//! point at something the feed doesn't have.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use flatnet_core::models::Date;
use source::Source;
use stop_times::Headsigns;
use table::Table;

use crate::{
    Error, Result,
    network::{
        Call, Network, NetworkInfo, RouteRecord, ServiceRecord, StopRecord, TransferRecord,
        TripRecord, Weekly,
    },
};

mod records;
mod shapes;
mod source;
mod stop_times;
mod table;

pub use table::SkippedRow;

/// What `Reader::read_with_report` has to say about a feed.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReadReport {
    /// The first rows that were left out, which is only ever non-empty in lenient mode.
    pub skipped_rows: Vec<SkippedRow>,
    /// How many rows were left out in all.
    pub skipped_total: usize,
}

/// Reads a GTFS feed into a [`Network`].
#[derive(Debug)]
pub struct Reader {
    path: PathBuf,
    lenient: bool,
}

impl Reader {
    /// Reads the feed at `path`, which is a directory or a zip.
    #[must_use]
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            lenient: false,
        }
    }

    /// Makes the reader lenient. In lenient mode, a row with a bad value is left out and reported, and the rest is read. By
    /// default it is an error. A missing required file or column is an error either way.
    #[must_use]
    pub const fn lenient(mut self) -> Self {
        self.lenient = true;
        self
    }

    /// Reads the feed.
    ///
    /// # Errors
    ///
    /// Returns an error if the feed can't be read, a required file or column is missing, or a value
    /// is malformed. Rows that point at an unknown route, trip or stop are skipped.
    pub fn read(&self) -> Result<Network> {
        self.read_with_report().map(|(network, _)| network)
    }

    /// Like `read`, and also says which rows were left out.
    ///
    /// # Errors
    ///
    /// Returns the same errors as `read`.
    pub fn read_with_report(&self) -> Result<(Network, ReadReport)> {
        let mut source = Source::open(&self.path)?;
        let mut files = Files {
            source: &mut source,
            lenient: self.lenient,
            report: ReadReport::default(),
        };

        let (stops, stop_map) = files.stops()?;
        let (routes, route_map) = files.routes()?;
        let (services, service_map) = files.services()?;
        let shape_lines = files.shapes()?;
        let trips = files.trips(&route_map, &service_map, &shape_lines.by_id)?;
        let trip_map = positions(trips.iter().map(|trip| trip.id.as_str()));
        let (calls, headsigns) = files.calls(&trip_map, &stop_map)?;
        let transfers = files.transfers(&stop_map, &trip_map)?;
        let info = files.info()?;

        let network = Network {
            info,
            stops,
            routes,
            services,
            trips,
            lines: shape_lines.lines,
            headsigns,
            calls,
            transfers,
        };
        Ok((network, files.report))
    }
}

/// The position of each id, which is where it lies in the sequence.
fn positions<'a>(ids: impl Iterator<Item = &'a str>) -> HashMap<String, usize> {
    ids.enumerate()
        .map(|(position, id)| (id.to_owned(), position))
        .collect()
}

/// The files of a feed, and what reading them has left out so far.
struct Files<'a> {
    source: &'a mut Source,
    lenient: bool,
    report: ReadReport,
}

/// Moves what a table skipped into the report.
fn absorb(report: &mut ReadReport, table: &mut Table<'_>) {
    let (rows, total) = table.take_skipped();
    report.skipped_total += total;
    report.skipped_rows.extend(rows);
}

/// Opens a file the feed must have.
fn open<'a>(source: &'a mut Source, lenient: bool, name: &'static str) -> Result<Table<'a>> {
    open_optional(source, lenient, name)?.ok_or(Error::MissingFile(name))
}

fn open_optional<'a>(
    source: &'a mut Source,
    lenient: bool,
    name: &'static str,
) -> Result<Option<Table<'a>>> {
    source
        .read(name)?
        .map(|input| Table::new(name, input, lenient))
        .transpose()
}

impl Files<'_> {
    /// Reads a file with `read_records` if the feed has it.
    fn read<T>(
        &mut self,
        name: &'static str,
        read_records: impl FnOnce(&mut Table<'_>) -> Result<T>,
    ) -> Result<Option<T>> {
        let Some(mut table) = open_optional(self.source, self.lenient, name)? else {
            return Ok(None);
        };
        let records = read_records(&mut table)?;
        absorb(&mut self.report, &mut table);
        Ok(Some(records))
    }

    fn stops(&mut self) -> Result<(Vec<StopRecord>, HashMap<String, usize>)> {
        let raw_stops = self
            .read("stops.txt", records::read_stops)?
            .ok_or(Error::MissingFile("stops.txt"))?;
        let stop_map = positions(raw_stops.iter().map(|stop| stop.id.as_str()));
        let stops = raw_stops
            .into_iter()
            .map(|stop| StopRecord {
                parent: stop
                    .parent_station
                    .as_ref()
                    .and_then(|parent| stop_map.get(parent).copied()),
                id: stop.id,
                code: stop.code,
                name: stop.name,
                description: stop.description,
                latitude: stop.latitude,
                longitude: stop.longitude,
                location_type: stop.location_type,
            })
            .collect();
        Ok((stops, stop_map))
    }

    fn routes(&mut self) -> Result<(Vec<RouteRecord>, HashMap<String, usize>)> {
        let raw_routes = self
            .read("routes.txt", records::read_routes)?
            .ok_or(Error::MissingFile("routes.txt"))?;
        let route_map = positions(raw_routes.iter().map(|route| route.id.as_str()));
        let routes = raw_routes
            .into_iter()
            .map(|route| RouteRecord {
                id: route.id,
                short_name: route.short_name,
                long_name: route.long_name,
                description: route.description,
                route_type: route.route_type,
            })
            .collect();
        Ok((routes, route_map))
    }

    fn shapes(&mut self) -> Result<shapes::ShapeLines> {
        let Some(mut table) = open_optional(self.source, self.lenient, "shapes.txt")? else {
            return Ok(shapes::ShapeLines::default());
        };
        let lines = shapes::read_shape_lines(&mut table)?;
        absorb(&mut self.report, &mut table);
        Ok(lines)
    }

    fn trips(
        &mut self,
        route_map: &HashMap<String, usize>,
        service_map: &HashMap<String, usize>,
        line_map: &HashMap<String, usize>,
    ) -> Result<Vec<TripRecord>> {
        let raw_trips = self
            .read("trips.txt", records::read_trips)?
            .ok_or(Error::MissingFile("trips.txt"))?;
        // A trip on a route the feed doesn't have can never run.
        Ok(raw_trips
            .into_iter()
            .filter_map(|trip| {
                let route = *route_map.get(&trip.route_id)?;
                Some(TripRecord {
                    route,
                    service: service_map.get(&trip.service_id).copied(),
                    line: trip
                        .shape_id
                        .as_ref()
                        .and_then(|shape_id| line_map.get(shape_id).copied()),
                    id: trip.id,
                    headsign: trip.headsign,
                    short_name: trip.short_name,
                    block: trip.block_id,
                    direction: trip.direction,
                })
            })
            .collect())
    }

    fn calls(
        &mut self,
        trip_map: &HashMap<String, usize>,
        stop_map: &HashMap<String, usize>,
    ) -> Result<(Vec<Call>, Vec<String>)> {
        let mut headsigns = Headsigns::default();
        let mut table = open(self.source, self.lenient, "stop_times.txt")?;
        let calls = stop_times::read_calls(&mut table, trip_map, stop_map, &mut headsigns)?;
        absorb(&mut self.report, &mut table);
        Ok((calls, headsigns.into_list()))
    }

    fn transfers(
        &mut self,
        stop_map: &HashMap<String, usize>,
        trip_map: &HashMap<String, usize>,
    ) -> Result<Vec<TransferRecord>> {
        let raw_transfers = self
            .read("transfers.txt", records::read_transfers)?
            .unwrap_or_default();
        // A transfer that names a stop or a trip the feed doesn't have can never apply.
        let trip = |id: &Option<String>| match id {
            Some(id) => trip_map.get(id).map(|&position| Some(position)),
            None => Some(None),
        };
        Ok(raw_transfers
            .into_iter()
            .filter_map(|transfer| {
                Some(TransferRecord {
                    from_stop: *stop_map.get(&transfer.from_stop_id)?,
                    to_stop: *stop_map.get(&transfer.to_stop_id)?,
                    from_trip: trip(&transfer.from_trip_id)?,
                    to_trip: trip(&transfer.to_trip_id)?,
                    transfer_type: transfer.transfer_type,
                    min_transfer_time: transfer.min_transfer_time,
                })
            })
            .collect())
    }

    fn info(&mut self) -> Result<NetworkInfo> {
        let agency = self
            .read("agency.txt", records::read_agency)?
            .unwrap_or_default();
        let feed = self
            .read("feed_info.txt", records::read_feed_info)?
            .unwrap_or_default();
        Ok(NetworkInfo {
            timezone: agency.timezone,
            agency_name: agency.name,
            agency_url: agency.url,
            valid_from: feed.start,
            valid_until: feed.end,
            publisher_name: feed.publisher_name,
            publisher_url: feed.publisher_url,
            language: feed.language,
            version: feed.version,
            contact_email: feed.contact_email,
        })
    }

    /// Leaves out a service that ends before it starts, which is an error unless lenient.
    fn skip_reversed(&mut self, service_id: &str) -> Result<()> {
        let error = Error::ReversedServiceRange {
            service_id: service_id.to_owned(),
        };
        if !self.lenient {
            return Err(error);
        }
        self.report.skipped_total += 1;
        self.report.skipped_rows.push(SkippedRow {
            file: "calendar.txt",
            message: error.to_string(),
        });
        Ok(())
    }

    /// Builds the services from the calendar, and from the calendar dates, which add or remove
    /// days. A set can have only a calendar, only calendar dates, or both. The calendar goes first,
    /// and a service only the calendar dates name runs on those dates alone.
    fn services(&mut self) -> Result<(Vec<ServiceRecord>, HashMap<String, usize>)> {
        let calendar = self.read("calendar.txt", records::read_calendar)?;
        let calendar_dates = self.read("calendar_dates.txt", records::read_calendar_dates)?;
        if calendar.is_none() && calendar_dates.is_none() {
            return Err(Error::MissingFile("calendar.txt or calendar_dates.txt"));
        }
        let calendar = calendar.unwrap_or_default();
        let calendar_dates = calendar_dates.unwrap_or_default();

        let mut services: Vec<ServiceRecord> = Vec::with_capacity(calendar.len());
        let mut service_map: HashMap<String, usize> = HashMap::with_capacity(calendar.len());

        for entry in &calendar {
            if entry.end_date < entry.start_date {
                self.skip_reversed(&entry.id)?;
                continue;
            }
            service_map.insert(entry.id.clone(), services.len());
            services.push(ServiceRecord {
                id: entry.id.clone(),
                weekly: Some(Weekly {
                    days: entry.days,
                    start: entry.start_date,
                    end: entry.end_date,
                }),
                exceptions: Vec::new(),
            });
        }

        for exception in &calendar_dates {
            let day: (Date, bool) = (exception.date, exception.added);
            if let Some(&position) = service_map.get(&exception.service_id) {
                services[position].exceptions.push(day);
                continue;
            }
            service_map.insert(exception.service_id.clone(), services.len());
            services.push(ServiceRecord {
                id: exception.service_id.clone(),
                weekly: None,
                exceptions: vec![day],
            });
        }
        Ok((services, service_map))
    }
}
