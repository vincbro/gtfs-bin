use std::collections::HashMap;

use flatnet_core::models::{
    BoardingRule, Distance, Optional, Slice, SliceBuilder, StopIndex, StopTimeSlice, StringSlice,
    Time, Trip, TripIndex,
};
use rayon::slice::ParallelSliceMut;

use crate::{
    Result,
    table::{Row, Table},
    tables,
};

/// One row of `stop_times.txt` that names a trip and a stop the feed defines.
///
/// The file keeps the times of these, and the rest once for each pattern, so the compiler holds
/// them like this until the patterns are known.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct StopTimeRow {
    pub(super) trip_index: TripIndex,
    pub(super) stop_index: StopIndex,
    pub(super) sequence: u32,
    pub(super) arrival_time: Optional<Time>,
    pub(super) departure_time: Optional<Time>,
    pub(super) headsign: Optional<StringSlice>,
    pub(super) distance_traveled: Optional<Distance>,
    pub(super) pickup: BoardingRule,
    pub(super) drop_off: BoardingRule,
}

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

/// Reads the stop times a row at a time, skipping the ones whose trip or stop the feed doesn't
/// define, and points each trip at its run of stop times.
///
/// A stop time with no times between two that have them gets a time spread evenly between them, by
/// distance along the shape when the feed has it. The spec asks for exactly that.
pub(super) fn build_stop_times(
    table: &mut Table<'_>,
    trips: &mut [Trip],
    trip_map: &HashMap<String, TripIndex>,
    stop_map: &HashMap<String, StopIndex>,
    slice_builder: &mut SliceBuilder<StringSlice>,
) -> Result<Vec<StopTimeRow>> {
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

    let mut stop_times: Vec<StopTimeRow> = Vec::new();
    while let Some(row) = table.next_row()? {
        let parsed = parse_row(&row, &columns, trip_map, stop_map, slice_builder);
        if let Some(Some(stop_time)) = table.accept(parsed)? {
            stop_times.push(stop_time);
        }
    }

    stop_times.par_sort_unstable_by(|first, second| {
        first
            .trip_index
            .cmp(&second.trip_index)
            .then(first.sequence.cmp(&second.sequence))
            .then(first.stop_index.cmp(&second.stop_index))
    });

    let slices: Vec<StopTimeSlice> = tables::slices_by_key(
        stop_times.iter().map(|stop_time| stop_time.trip_index),
        trips.len(),
    );
    for (trip, slice) in trips.iter_mut().zip(slices) {
        trip.stop_times = slice;
        interpolate_times(&mut stop_times[slice.range()]);
    }

    Ok(stop_times)
}

/// Parses one row, or returns `None` when it names a trip or stop the feed doesn't define.
fn parse_row(
    row: &Row<'_>,
    columns: &Columns,
    trip_map: &HashMap<String, TripIndex>,
    stop_map: &HashMap<String, StopIndex>,
    slice_builder: &mut SliceBuilder<StringSlice>,
) -> Result<Option<StopTimeRow>> {
    let Some(&trip_index) = trip_map.get(row.required(columns.trip_id, "trip_id")?) else {
        return Ok(None);
    };
    let Some(&stop_index) = stop_map.get(row.required(columns.stop_id, "stop_id")?) else {
        return Ok(None);
    };
    let boarding = |column, name| -> Result<BoardingRule> {
        Ok(row
            .parse::<u8>(column, name)?
            .map_or(BoardingRule::REGULAR, BoardingRule::from))
    };

    Ok(Some(StopTimeRow {
        headsign: row
            .text(columns.headsign)
            .map(|text| slice_builder.add(text))
            .into(),
        stop_index,
        trip_index,
        sequence: row.parse_required(columns.sequence, "stop_sequence")?,
        arrival_time: row.time(columns.arrival_time, "arrival_time")?.into(),
        departure_time: row.time(columns.departure_time, "departure_time")?.into(),
        distance_traveled: row
            .float(columns.distance, "shape_dist_traveled")?
            .map(Distance::from_meters)
            .into(),
        pickup: boarding(columns.pickup, "pickup_type")?,
        drop_off: boarding(columns.drop_off, "drop_off_type")?,
    }))
}

/// Fills in the times of stops that have none, between two stops that do.
fn interpolate_times(run: &mut [StopTimeRow]) {
    let timed: Vec<usize> = (0..run.len())
        .filter(|&position| {
            run[position].arrival_time.is_some() || run[position].departure_time.is_some()
        })
        .collect();
    for pair in timed.windows(2) {
        if pair[1] - pair[0] > 1 {
            fill_between(run, pair[0], pair[1]);
        }
    }
}

/// Returns `part` as a share of `whole`.
#[allow(
    clippy::cast_precision_loss,
    reason = "a trip has far fewer than 2^52 stops"
)]
fn share(part: usize, whole: usize) -> f64 {
    part as f64 / whole as f64
}

/// Gives the stops strictly between `first` and `last` a time between theirs.
fn fill_between(run: &mut [StopTimeRow], first: usize, last: usize) {
    let leaves = |stop_time: &StopTimeRow| {
        stop_time
            .departure_time
            .as_option()
            .or_else(|| stop_time.arrival_time.as_option())
    };
    let arrives = |stop_time: &StopTimeRow| {
        stop_time
            .arrival_time
            .as_option()
            .or_else(|| stop_time.departure_time.as_option())
    };
    let (Some(start), Some(end)) = (leaves(&run[first]), arrives(&run[last])) else {
        return;
    };
    // A feed whose times run backwards has nothing sensible to spread.
    if end.0 < start.0 {
        return;
    }

    let distances = (
        run[first].distance_traveled.as_option(),
        run[last].distance_traveled.as_option(),
    );
    let span = f64::from(end.0 - start.0);
    for (offset, stop_time) in run[first + 1..last].iter_mut().enumerate() {
        let fraction = match (distances, stop_time.distance_traveled.as_option()) {
            ((Some(from), Some(to)), Some(at)) if to.0 > from.0 => {
                f64::from((at.0 - from.0) / (to.0 - from.0)).clamp(0.0, 1.0)
            }
            _ => share(offset + 1, last - first),
        };
        // The result is between the two times, so it fits in u32.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let time = Optional::new(Time(start.0 + (span * fraction).round() as u32));
        stop_time.arrival_time = time;
        stop_time.departure_time = time;
    }
}

#[cfg(test)]
mod tests {
    use flatnet_core::models::Distance;

    use super::*;

    fn stop(arrival: Option<u32>, departure: Option<u32>, distance: Option<f32>) -> StopTimeRow {
        StopTimeRow {
            arrival_time: arrival.map(Time).into(),
            departure_time: departure.map(Time).into(),
            distance_traveled: distance.map(Distance).into(),
            ..StopTimeRow::default()
        }
    }

    fn times(run: &[StopTimeRow]) -> Vec<Option<u32>> {
        run.iter()
            .map(|stop_time| stop_time.departure_time.as_option().map(|time| time.0))
            .collect()
    }

    #[test]
    fn untimed_stops_are_spread_evenly_between_timed_ones() {
        let mut run = [
            stop(Some(0), Some(0), None),
            stop(None, None, None),
            stop(None, None, None),
            stop(Some(300), Some(300), None),
        ];

        interpolate_times(&mut run);

        assert_eq!(
            times(&run),
            [Some(0), Some(100), Some(200), Some(300)],
            "thirds of 300"
        );
    }

    #[test]
    fn distance_decides_the_spread_when_the_feed_has_it() {
        let mut run = [
            stop(Some(0), Some(0), Some(0.0)),
            stop(None, None, Some(900.0)),
            stop(Some(1000), Some(1000), Some(1000.0)),
        ];

        interpolate_times(&mut run);

        assert_eq!(
            times(&run)[1],
            Some(900),
            "nine tenths of the way along is nine tenths of the time"
        );
    }

    #[test]
    fn stops_outside_the_timed_ones_stay_untimed() {
        let mut run = [
            stop(None, None, None),
            stop(Some(60), Some(60), None),
            stop(None, None, None),
        ];

        interpolate_times(&mut run);

        assert_eq!(
            times(&run),
            [None, Some(60), None],
            "there is nothing to spread from"
        );
    }

    #[test]
    fn times_that_run_backwards_are_left_alone() {
        let mut run = [
            stop(Some(500), Some(500), None),
            stop(None, None, None),
            stop(Some(100), Some(100), None),
        ];

        interpolate_times(&mut run);

        assert_eq!(times(&run)[1], None, "no sensible time exists");
    }
}
