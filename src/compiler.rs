use std::path::{Path, PathBuf};

use services::BuiltServices;
use source::Source;
use table::Table;
use transfers::BuiltTransfers;
use trip_patterns::TripPatterns;
use writer::BinaryWriter;

use crate::{
    GTFS_BIN_VERSION,
    models::{self, Date, Header, Sentinel, SliceBuilder},
};

mod error;
mod grid;
mod records;
mod routes;
mod services;
mod shapes;
mod source;
mod stop_times;
mod stops;
mod table;
mod tables;
mod transfers;
mod trip_patterns;
mod trips;
mod writer;

pub use error::{Error, Result};
pub use table::SkippedRow;

/// What `Compiler::compile_with_report` has to say about a feed.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CompileReport {
    /// The first rows that were left out, which is only ever non-empty in lenient mode.
    pub skipped_rows: Vec<SkippedRow>,
    /// How many rows were left out in all.
    pub skipped_total: usize,
}

/// Compiles a GTFS feed into the binary format `Consumer` reads.
///
/// The same feed always compiles to the same bytes.
#[derive(Debug, Default)]
pub struct Compiler {
    path: PathBuf,
    lenient: bool,
}

impl Compiler {
    /// Creates a compiler for a feed that is a directory of text files or a zip archive of them.
    #[must_use]
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            lenient: false,
        }
    }

    /// Leaves out a row with a bad value instead of failing the whole feed.
    ///
    /// Use `compile_with_report` to see what was left out. A missing file or column, or a file that
    /// isn't csv, is still an error.
    #[must_use]
    pub const fn lenient(mut self) -> Self {
        self.lenient = true;
        self
    }

    /// Reads the feed and returns the compiled file.
    ///
    /// # Errors
    ///
    /// Returns an error if the feed can't be read, a required file or column is missing, or a value
    /// is malformed. Rows that point at an unknown route, trip or stop are skipped.
    pub fn compile(&self) -> Result<Vec<u8>> {
        self.compile_with_report().map(|(bytes, _)| bytes)
    }

    /// Like `compile`, and also says which rows were left out.
    ///
    /// # Errors
    ///
    /// Returns the same errors as `compile`.
    #[allow(
        clippy::too_many_lines,
        reason = "one pass that reads each table and then lists every section in file order"
    )]
    pub fn compile_with_report(&self) -> Result<(Vec<u8>, CompileReport)> {
        let mut source = Source::open(&self.path)?;
        let mut report = CompileReport::default();
        let mut slice_builder = SliceBuilder::new();
        let lenient = self.lenient;

        let raw_stops = read(
            &mut source,
            "stops.txt",
            lenient,
            &mut report,
            records::read_stops,
        )?
        .ok_or(Error::MissingFile("stops.txt"))?;
        let (mut stops, stop_map) = stops::build_stops(&raw_stops, &mut slice_builder);

        let raw_routes = read(
            &mut source,
            "routes.txt",
            lenient,
            &mut report,
            records::read_routes,
        )?
        .ok_or(Error::MissingFile("routes.txt"))?;
        let (mut routes, route_map) = routes::build_routes(&raw_routes, &mut slice_builder);

        let raw_calendar = read(
            &mut source,
            "calendar.txt",
            lenient,
            &mut report,
            records::read_calendar,
        )?;
        let raw_calendar_dates = read(
            &mut source,
            "calendar_dates.txt",
            lenient,
            &mut report,
            records::read_calendar_dates,
        )?;
        if raw_calendar.is_none() && raw_calendar_dates.is_none() {
            return Err(Error::MissingFile("calendar.txt or calendar_dates.txt"));
        }
        let BuiltServices {
            mut services,
            id_map: service_map,
            active_mask,
            skipped,
        } = services::build_services(
            &raw_calendar.unwrap_or_default(),
            &raw_calendar_dates.unwrap_or_default(),
            lenient,
        )?;
        report.skipped_total += skipped.len();
        report.skipped_rows.extend(skipped);

        let (shapes, shape_map) = open_optional(&mut source, "shapes.txt", lenient)?
            .map(|mut table| {
                let built = shapes::build_shapes(&mut table);
                absorb(&mut report, &mut table);
                built
            })
            .transpose()?
            .unwrap_or_default();

        let raw_trips = read(
            &mut source,
            "trips.txt",
            lenient,
            &mut report,
            records::read_trips,
        )?
        .ok_or(Error::MissingFile("trips.txt"))?;
        let (mut trips, trip_map) = trips::build_trips(
            &raw_trips,
            &route_map,
            &service_map,
            &shape_map,
            &mut slice_builder,
        );

        let mut stop_times_table = open(&mut source, "stop_times.txt", lenient)?;
        let stop_times = stop_times::build_stop_times(
            &mut stop_times_table,
            &mut trips,
            &trip_map,
            &stop_map,
            &mut slice_builder,
        )?;
        absorb(&mut report, &mut stop_times_table);
        drop(stop_times_table);

        // Built even when the feed has no transfers, because the consumer indexes the lookups by
        // stop.
        let raw_transfers = read(
            &mut source,
            "transfers.txt",
            lenient,
            &mut report,
            records::read_transfers,
        )?
        .unwrap_or_default();
        let BuiltTransfers {
            transfers,
            outbound: stop_to_transfers_out,
            inbound_indices: transfers_in_indices,
            inbound: stop_to_transfers_in,
        } = transfers::build_transfers(&raw_transfers, &stop_map, &trip_map, stops.len());

        let timezone = read(
            &mut source,
            "agency.txt",
            lenient,
            &mut report,
            records::read_timezone,
        )?
        .flatten()
        .unwrap_or_default();
        let validity = read(
            &mut source,
            "feed_info.txt",
            lenient,
            &mut report,
            records::read_feed_validity,
        )?
        .unwrap_or_default();

        let (stop_to_trips, stop_to_trips_lookup) = stops::build_stop_to_trips(&stops, &stop_times);
        let (search_stops, search_to_stops) = stops::build_stop_search(&stops);
        let (stop_grid, grid_stops) = grid::build_stop_grid(&stops);
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

        let mut header = Header {
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

            timezone: writer.write_section(timezone.as_bytes()),
            stop_grid: writer.write_section(&stop_grid),
            grid_stops: writer.write_section(&grid_stops),

            feed_start_date: validity.start.unwrap_or(Date::NONE),
            feed_end_date: validity.end.unwrap_or(Date::NONE),
            file_length: 0,
            checksum: 0,
        };
        header.file_length = writer.as_bytes().len() as u64;
        header.checksum = models::checksum(&writer.as_bytes()[size_of::<Header>()..]);
        writer.overwrite(0, bytemuck::bytes_of(&header));

        Ok((writer.into_bytes(), report))
    }
}

/// Moves what a table skipped into the report.
fn absorb(report: &mut CompileReport, table: &mut Table<'_>) {
    let (rows, total) = table.take_skipped();
    report.skipped_total += total;
    report.skipped_rows.extend(rows);
}

/// Reads a file with `read_records` if the feed has it.
fn read<T>(
    source: &mut Source,
    name: &'static str,
    lenient: bool,
    report: &mut CompileReport,
    read_records: impl FnOnce(&mut Table<'_>) -> Result<T>,
) -> Result<Option<T>> {
    let Some(mut table) = open_optional(source, name, lenient)? else {
        return Ok(None);
    };
    let records = read_records(&mut table)?;
    absorb(report, &mut table);
    Ok(Some(records))
}

/// Opens a file the feed must have.
fn open<'a>(source: &'a mut Source, name: &'static str, lenient: bool) -> Result<Table<'a>> {
    open_optional(source, name, lenient)?.ok_or(Error::MissingFile(name))
}

fn open_optional<'a>(
    source: &'a mut Source,
    name: &'static str,
    lenient: bool,
) -> Result<Option<Table<'a>>> {
    source
        .read(name)?
        .map(|input| Table::new(name, input, lenient))
        .transpose()
}
