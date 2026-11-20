//! The network a reader gives the compiler.
//!
//! A reader turns what a source says into a [`Network`], and the compiler turns a `Network` into
//! the file. The `Network` is the one place the two meet, so a new source is a new reader and
//! nothing else changes.
//!
//! Everything in it refers to something else by its position in the vector that holds it, not by
//! the id the source used. The reader resolves ids, and leaves out what points at nothing.

use std::collections::HashSet;

use flatnet_core::models::{
    BoardingRule, Coordinate, Date, Direction, Distance, LocationType, Optional, RouteType, Time,
};

use crate::{Error, Result};

/// A transit network, as a source describes it.
///
/// The ids of one kind, such as the stops', are unique. A reference is a position in the vector of
/// the thing it names, and the compiler refuses a network where one is out of range.
#[derive(Debug, Default, Clone)]
pub struct Network {
    pub info: NetworkInfo,
    pub stops: Vec<StopRecord>,
    pub routes: Vec<RouteRecord>,
    pub services: Vec<ServiceRecord>,
    pub trips: Vec<TripRecord>,
    /// The paths of the shapes, each a line of points in order. A trip names one by position.
    pub lines: Vec<Vec<Coordinate>>,
    /// Headsigns calls use, each once. A call names one by position.
    pub headsigns: Vec<String>,
    /// Every stop a trip makes, in any order. The compiler orders them by trip and sequence.
    pub calls: Vec<Call>,
    pub transfers: Vec<TransferRecord>,
}

/// What the source says about itself and its agency.
#[derive(Debug, Default, Clone)]
pub struct NetworkInfo {
    /// The name of the time zone the times are local to, such as `Europe/Stockholm`.
    pub timezone: Option<String>,
    pub agency_name: Option<String>,
    pub agency_url: Option<String>,
    /// The first and last day the source says it is valid for.
    pub valid_from: Option<Date>,
    pub valid_until: Option<Date>,
    pub publisher_name: Option<String>,
    pub publisher_url: Option<String>,
    pub language: Option<String>,
    pub version: Option<String>,
    pub contact_email: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StopRecord {
    pub id: String,
    pub code: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    /// Degrees. A stop has a position only if it has both.
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub location_type: LocationType,
    /// The position of the station this stop belongs to.
    pub parent: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct RouteRecord {
    pub id: String,
    pub short_name: Option<String>,
    pub long_name: Option<String>,
    pub description: Option<String>,
    pub route_type: RouteType,
}

/// The days a service runs: a week that repeats between two dates, and days added or removed.
///
/// A service with neither is not allowed, because it has no days to give a range.
#[derive(Debug, Clone)]
pub struct ServiceRecord {
    pub id: String,
    pub weekly: Option<Weekly>,
    /// A date and whether service runs on it, in the order they apply. A later one wins.
    pub exceptions: Vec<(Date, bool)>,
}

/// A week that repeats from `start` to `end`, both included.
#[derive(Debug, Clone, Copy)]
pub struct Weekly {
    /// Monday first.
    pub days: [bool; 7],
    pub start: Date,
    pub end: Date,
}

#[derive(Debug, Clone)]
pub struct TripRecord {
    pub id: String,
    pub route: usize,
    pub service: Option<usize>,
    pub headsign: Option<String>,
    pub short_name: Option<String>,
    /// The position of the line it follows in `Network::lines`.
    pub line: Option<usize>,
    pub block: Option<String>,
    pub direction: Direction,
}

/// One stop of one trip.
#[derive(Debug, Clone, Copy)]
pub struct Call {
    pub trip: usize,
    pub stop: usize,
    /// Orders the stops of a trip.
    pub sequence: u32,
    pub arrival: Optional<Time>,
    pub departure: Optional<Time>,
    /// The position of the headsign in `Network::headsigns`.
    pub headsign: Option<u32>,
    pub distance: Optional<Distance>,
    pub pickup: BoardingRule,
    pub drop_off: BoardingRule,
}

#[derive(Debug, Clone)]
pub struct TransferRecord {
    pub from_stop: usize,
    pub to_stop: usize,
    /// The trip the transfer only applies when arriving on.
    pub from_trip: Option<usize>,
    /// The trip the transfer only applies when leaving on.
    pub to_trip: Option<usize>,
    pub transfer_type: u8,
    pub min_transfer_time: Option<u32>,
}

impl Network {
    /// Checks what the compiler relies on: ids that are unique and references that point at
    /// something.
    pub(crate) fn check(&self) -> Result<()> {
        unique("stops", self.stops.iter().map(|stop| stop.id.as_str()))?;
        unique("routes", self.routes.iter().map(|route| route.id.as_str()))?;
        unique(
            "services",
            self.services.iter().map(|service| service.id.as_str()),
        )?;
        unique("trips", self.trips.iter().map(|trip| trip.id.as_str()))?;

        let reference = |table, field, position: usize, length: usize| {
            if position < length {
                Ok(())
            } else {
                Err(Error::InvalidReference { table, field })
            }
        };
        let optional = |table, field, position: Option<usize>, length: usize| {
            position.map_or(Ok(()), |position| reference(table, field, position, length))
        };

        for stop in &self.stops {
            optional("stops", "parent", stop.parent, self.stops.len())?;
        }
        for trip in &self.trips {
            reference("trips", "route", trip.route, self.routes.len())?;
            optional("trips", "service", trip.service, self.services.len())?;
            optional("trips", "line", trip.line, self.lines.len())?;
        }
        for call in &self.calls {
            reference("calls", "trip", call.trip, self.trips.len())?;
            reference("calls", "stop", call.stop, self.stops.len())?;
            optional(
                "calls",
                "headsign",
                call.headsign.map(|headsign| headsign as usize),
                self.headsigns.len(),
            )?;
        }
        for transfer in &self.transfers {
            reference(
                "transfers",
                "from_stop",
                transfer.from_stop,
                self.stops.len(),
            )?;
            reference("transfers", "to_stop", transfer.to_stop, self.stops.len())?;
            optional(
                "transfers",
                "from_trip",
                transfer.from_trip,
                self.trips.len(),
            )?;
            optional("transfers", "to_trip", transfer.to_trip, self.trips.len())?;
        }
        Ok(())
    }
}

/// Fails on the first id that appears twice.
fn unique<'a>(table: &'static str, ids: impl Iterator<Item = &'a str>) -> Result<()> {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(Error::DuplicateId {
                table,
                id: id.to_owned(),
            });
        }
    }
    Ok(())
}
