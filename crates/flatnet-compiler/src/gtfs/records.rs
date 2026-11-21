use std::collections::HashSet;

use flatnet_core::models::{Coordinate, Date, Direction, LocationType, RouteType, Sentinel};

use super::table::{Row, Table};
use crate::Result;

pub(super) struct RawStop {
    pub id: String,
    pub code: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub location_type: LocationType,
    pub parent_station: Option<String>,
}

pub(super) struct RawRoute {
    pub id: String,
    pub short_name: Option<String>,
    pub long_name: Option<String>,
    pub description: Option<String>,
    pub route_type: RouteType,
}

pub(super) struct RawTrip {
    pub id: String,
    pub route_id: String,
    pub service_id: String,
    pub headsign: Option<String>,
    pub short_name: Option<String>,
    pub shape_id: Option<String>,
    pub block_id: Option<String>,
    pub direction: Direction,
}

pub(super) struct RawCalendar {
    pub id: String,
    /// Monday first.
    pub days: [bool; 7],
    pub start_date: Date,
    pub end_date: Date,
}

pub(super) struct RawCalendarDate {
    pub service_id: String,
    pub date: Date,
    /// Whether service is added on the date, as opposed to removed.
    pub added: bool,
}

pub(super) struct RawTransfer {
    pub from_stop_id: String,
    pub to_stop_id: String,
    pub from_trip_id: Option<String>,
    pub to_trip_id: Option<String>,
    pub transfer_type: u8,
    pub min_transfer_time: Option<u32>,
}

/// What `feed_info.txt` says.
#[derive(Debug, Default, Clone)]
pub(super) struct FeedInfo {
    pub start: Option<Date>,
    pub end: Option<Date>,
    pub publisher_name: Option<String>,
    pub publisher_url: Option<String>,
    pub language: Option<String>,
    pub version: Option<String>,
    pub contact_email: Option<String>,
}

/// What `agency.txt` says about the first agency that has a time zone.
#[derive(Debug, Default, Clone)]
pub(super) struct AgencyInfo {
    pub timezone: Option<String>,
    pub name: Option<String>,
    pub url: Option<String>,
}

/// Drops every item whose id was already used, keeping the first. Ids are meant to be unique, and a
/// feed that repeats one would otherwise leave an item nothing can look up.
fn keep_first<T>(items: &mut Vec<T>, id: impl Fn(&T) -> &str) {
    let keep: Vec<bool> = {
        let mut seen = HashSet::with_capacity(items.len());
        items.iter().map(|item| seen.insert(id(item))).collect()
    };
    let mut keep = keep.into_iter();
    items.retain(|_| keep.next().unwrap_or(true));
}

fn owned(text: Option<&str>) -> Option<String> {
    text.map(str::to_owned)
}

/// Reads every row of a table with `parse`, dropping the ones lenient mode skips.
fn read_rows<T>(table: &mut Table<'_>, parse: impl Fn(&Row<'_>) -> Result<T>) -> Result<Vec<T>> {
    let mut items = Vec::new();
    while let Some(row) = table.next_row()? {
        let parsed = parse(&row);
        if let Some(item) = table.accept(parsed)? {
            items.push(item);
        }
    }
    Ok(items)
}

pub(super) fn read_stops(table: &mut Table<'_>) -> Result<Vec<RawStop>> {
    let id = table.require("stop_id")?;
    let code = table.column("stop_code");
    let name = table.column("stop_name");
    let description = table.column("stop_desc");
    let latitude = table.column("stop_lat");
    let longitude = table.column("stop_lon");
    let location_type = table.column("location_type");
    let parent_station = table.column("parent_station");

    let mut stops = read_rows(table, |row| {
        Ok(RawStop {
            id: row.required(id, "stop_id")?.to_owned(),
            code: owned(row.text(code)),
            name: owned(row.text(name)),
            description: owned(row.text(description)),
            latitude: row.degrees(latitude, "stop_lat", Coordinate::MAX_LATITUDE)?,
            longitude: row.degrees(longitude, "stop_lon", Coordinate::MAX_LONGITUDE)?,
            location_type: row
                .parse::<u8>(location_type, "location_type")?
                .map_or(LocationType::STOP_POINT, LocationType::from),
            parent_station: owned(row.text(parent_station)),
        })
    })?;
    keep_first(&mut stops, |stop| &stop.id);
    Ok(stops)
}

pub(super) fn read_routes(table: &mut Table<'_>) -> Result<Vec<RawRoute>> {
    let id = table.require("route_id")?;
    let route_type = table.require("route_type")?;
    let short_name = table.column("route_short_name");
    let long_name = table.column("route_long_name");
    let description = table.column("route_desc");

    let mut routes = read_rows(table, |row| {
        Ok(RawRoute {
            id: row.required(id, "route_id")?.to_owned(),
            short_name: owned(row.text(short_name)),
            long_name: owned(row.text(long_name)),
            description: owned(row.text(description)),
            route_type: RouteType::from(row.parse_required::<u16>(route_type, "route_type")?),
        })
    })?;
    keep_first(&mut routes, |route| &route.id);
    Ok(routes)
}

pub(super) fn read_trips(table: &mut Table<'_>) -> Result<Vec<RawTrip>> {
    let id = table.require("trip_id")?;
    let route_id = table.require("route_id")?;
    let service_id = table.require("service_id")?;
    let headsign = table.column("trip_headsign");
    let short_name = table.column("trip_short_name");
    let shape_id = table.column("shape_id");
    let block_id = table.column("block_id");
    let direction_id = table.column("direction_id");

    let mut trips = read_rows(table, |row| {
        Ok(RawTrip {
            id: row.required(id, "trip_id")?.to_owned(),
            route_id: row.required(route_id, "route_id")?.to_owned(),
            service_id: row.required(service_id, "service_id")?.to_owned(),
            headsign: owned(row.text(headsign)),
            short_name: owned(row.text(short_name)),
            shape_id: owned(row.text(shape_id)),
            block_id: owned(row.text(block_id)),
            direction: row
                .parse::<u8>(direction_id, "direction_id")?
                .map_or(Direction::NONE, Direction::from),
        })
    })?;
    keep_first(&mut trips, |trip| &trip.id);
    Ok(trips)
}

const WEEKDAY_COLUMNS: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];

pub(super) fn read_calendar(table: &mut Table<'_>) -> Result<Vec<RawCalendar>> {
    let id = table.require("service_id")?;
    let start_date = table.require("start_date")?;
    let end_date = table.require("end_date")?;
    let mut day_columns = [0; 7];
    for (column, name) in day_columns.iter_mut().zip(WEEKDAY_COLUMNS) {
        *column = table.require(name)?;
    }

    let mut calendar = read_rows(table, |row| {
        let mut days = [false; 7];
        for ((runs, &column), name) in days.iter_mut().zip(&day_columns).zip(WEEKDAY_COLUMNS) {
            *runs = row.flag(column, name)?;
        }
        Ok(RawCalendar {
            id: row.required(id, "service_id")?.to_owned(),
            days,
            start_date: row.date(start_date, "start_date")?,
            end_date: row.date(end_date, "end_date")?,
        })
    })?;
    keep_first(&mut calendar, |service| &service.id);
    Ok(calendar)
}

pub(super) fn read_calendar_dates(table: &mut Table<'_>) -> Result<Vec<RawCalendarDate>> {
    let service_id = table.require("service_id")?;
    let date = table.require("date")?;
    let exception_type = table.require("exception_type")?;

    read_rows(table, |row| {
        let added = match row.required(exception_type, "exception_type")? {
            "1" => true,
            "2" => false,
            other => return Err(row.invalid("exception_type", other)),
        };
        Ok(RawCalendarDate {
            service_id: row.required(service_id, "service_id")?.to_owned(),
            date: row.date(date, "date")?,
            added,
        })
    })
}

pub(super) fn read_transfers(table: &mut Table<'_>) -> Result<Vec<RawTransfer>> {
    let from_stop_id = table.require("from_stop_id")?;
    let to_stop_id = table.require("to_stop_id")?;
    let from_trip_id = table.column("from_trip_id");
    let to_trip_id = table.column("to_trip_id");
    let transfer_type = table.column("transfer_type");
    let min_transfer_time = table.column("min_transfer_time");

    read_rows(table, |row| {
        Ok(RawTransfer {
            from_stop_id: row.required(from_stop_id, "from_stop_id")?.to_owned(),
            to_stop_id: row.required(to_stop_id, "to_stop_id")?.to_owned(),
            from_trip_id: owned(row.text(from_trip_id)),
            to_trip_id: owned(row.text(to_trip_id)),
            transfer_type: row.parse(transfer_type, "transfer_type")?.unwrap_or(0),
            min_transfer_time: row.parse(min_transfer_time, "min_transfer_time")?,
        })
    })
}

/// Reads the first agency that has a time zone. A feed's agencies all share a zone.
pub(super) fn read_agency(table: &mut Table<'_>) -> Result<AgencyInfo> {
    let timezone = table.require("agency_timezone")?;
    let name = table.column("agency_name");
    let url = table.column("agency_url");
    let agencies = read_rows(table, |row| {
        Ok(AgencyInfo {
            timezone: owned(row.text(Some(timezone))),
            name: owned(row.text(name)),
            url: owned(row.text(url)),
        })
    })?;
    Ok(agencies
        .into_iter()
        .find(|agency| agency.timezone.is_some())
        .unwrap_or_default())
}

pub(super) fn read_feed_info(table: &mut Table<'_>) -> Result<FeedInfo> {
    let start = table.column("feed_start_date");
    let end = table.column("feed_end_date");
    let publisher_name = table.column("feed_publisher_name");
    let publisher_url = table.column("feed_publisher_url");
    let language = table.column("feed_lang");
    let version = table.column("feed_version");
    let contact_email = table.column("feed_contact_email");
    let parse_date = |row: &Row<'_>, column: Option<usize>, name: &'static str| match column {
        Some(column) if row.text(Some(column)).is_some() => row.date(column, name).map(Some),
        _ => Ok(None),
    };

    let rows = read_rows(table, |row| {
        Ok(FeedInfo {
            start: parse_date(row, start, "feed_start_date")?,
            end: parse_date(row, end, "feed_end_date")?,
            publisher_name: owned(row.text(publisher_name)),
            publisher_url: owned(row.text(publisher_url)),
            language: owned(row.text(language)),
            version: owned(row.text(version)),
            contact_email: owned(row.text(contact_email)),
        })
    })?;
    Ok(rows.into_iter().next().unwrap_or_default())
}
