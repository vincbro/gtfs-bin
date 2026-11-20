use std::collections::HashMap;

use flatnet_core::models::{BoardingRule, Distance};

use super::table::{Row, Table};
use crate::{Result, network::Call};

/// Where each column of `stop_times.txt` is.
struct Columns {
    trip_id: usize,
    stop_id: usize,
    sequence: usize,
    arrival_time: Option<usize>,
    departure_time: Option<usize>,
    headsign: Option<usize>,
    distance: Option<usize>,
    pickup: Option<usize>,
    drop_off: Option<usize>,
}

/// The headsigns the stop times use, each stored once, in the order they first appear.
#[derive(Debug, Default)]
pub(super) struct Headsigns {
    positions: HashMap<String, u32>,
    list: Vec<String>,
}

impl Headsigns {
    fn intern(&mut self, text: &str) -> u32 {
        if let Some(&position) = self.positions.get(text) {
            return position;
        }
        let position = u32::try_from(self.list.len()).unwrap_or(u32::MAX);
        self.positions.insert(text.to_owned(), position);
        self.list.push(text.to_owned());
        position
    }

    pub(super) fn into_list(self) -> Vec<String> {
        self.list
    }
}

/// Reads the stop times a row at a time, skipping the ones whose trip or stop the feed doesn't
/// define.
pub(super) fn read_calls(
    table: &mut Table<'_>,
    trip_map: &HashMap<String, usize>,
    stop_map: &HashMap<String, usize>,
    headsigns: &mut Headsigns,
) -> Result<Vec<Call>> {
    let columns = Columns {
        trip_id: table.require("trip_id")?,
        stop_id: table.require("stop_id")?,
        sequence: table.require("stop_sequence")?,
        arrival_time: table.column("arrival_time"),
        departure_time: table.column("departure_time"),
        headsign: table.column("stop_headsign"),
        distance: table.column("shape_dist_traveled"),
        pickup: table.column("pickup_type"),
        drop_off: table.column("drop_off_type"),
    };

    let mut calls: Vec<Call> = Vec::new();
    while let Some(row) = table.next_row()? {
        let parsed = parse_row(&row, &columns, trip_map, stop_map, headsigns);
        if let Some(Some(call)) = table.accept(parsed)? {
            calls.push(call);
        }
    }
    Ok(calls)
}

/// Parses one row, or returns `None` when it names a trip or stop the feed doesn't define.
fn parse_row(
    row: &Row<'_>,
    columns: &Columns,
    trip_map: &HashMap<String, usize>,
    stop_map: &HashMap<String, usize>,
    headsigns: &mut Headsigns,
) -> Result<Option<Call>> {
    let Some(&trip) = trip_map.get(row.required(columns.trip_id, "trip_id")?) else {
        return Ok(None);
    };
    let Some(&stop) = stop_map.get(row.required(columns.stop_id, "stop_id")?) else {
        return Ok(None);
    };
    let boarding = |column, name| -> Result<BoardingRule> {
        Ok(row
            .parse::<u8>(column, name)?
            .map_or(BoardingRule::REGULAR, BoardingRule::from))
    };

    Ok(Some(Call {
        trip,
        stop,
        headsign: row
            .text(columns.headsign)
            .map(|text| headsigns.intern(text)),
        sequence: row.parse_required(columns.sequence, "stop_sequence")?,
        arrival: row.time(columns.arrival_time, "arrival_time")?.into(),
        departure: row.time(columns.departure_time, "departure_time")?.into(),
        distance: row
            .float(columns.distance, "shape_dist_traveled")?
            .map(Distance::from_meters)
            .into(),
        pickup: boarding(columns.pickup, "pickup_type")?,
        drop_off: boarding(columns.drop_off, "drop_off_type")?,
    }))
}
