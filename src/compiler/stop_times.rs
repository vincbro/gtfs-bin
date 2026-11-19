use std::collections::HashMap;

use rayon::slice::ParallelSliceMut;

use crate::{
    compiler::{
        Result,
        table::{Row, Table},
        tables,
    },
    models::{
        BoardingRule, Distance, Optional, Sentinel, Slice, SliceBuilder, StopIndex, StopTime,
        StopTimeIndex, StopTimeSlice, StringSlice, Time, Trip, TripIndex,
    },
};

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
    table: &mut Table,
    trips: &mut [Trip],
    trip_map: &HashMap<String, TripIndex>,
    stop_map: &HashMap<String, StopIndex>,
    slice_builder: &mut SliceBuilder<StringSlice>,
) -> Result<Vec<StopTime>> {
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

    let mut stop_times: Vec<StopTime> = Vec::new();
    while let Some(row) = table.next_row()? {
        let parsed = parse_row(&row, &columns, trip_map, stop_map, slice_builder);
        if let Some(Some(stop_time)) = table.accept(parsed)? {
            stop_times.push(stop_time);
        }
    }

    stop_times.par_sort_unstable_by(|a, b| {
        a.trip_index
            .cmp(&b.trip_index)
            .then(a.sequence.cmp(&b.sequence))
            .then(a.stop_index.cmp(&b.stop_index))
    });

    for (position, stop_time) in stop_times.iter_mut().enumerate() {
        stop_time.index = StopTimeIndex::from(position);
    }

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
    row: &Row,
    columns: &Columns,
    trip_map: &HashMap<String, TripIndex>,
    stop_map: &HashMap<String, StopIndex>,
    slice_builder: &mut SliceBuilder<StringSlice>,
) -> Result<Option<StopTime>> {
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

    Ok(Some(StopTime {
        index: StopTimeIndex::NONE,
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
            .map(|meters| Distance(meters as f32))
            .into(),
        pickup: boarding(columns.pickup, "pickup_type")?,
        drop_off: boarding(columns.drop_off, "drop_off_type")?,
        padding: [0; 2],
    }))
}

/// Fills in the times of stops that have none, between two stops that do.
fn interpolate_times(run: &mut [StopTime]) {
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

/// Gives the stops strictly between `first` and `last` a time between theirs.
fn fill_between(run: &mut [StopTime], first: usize, last: usize) {
    let leaves = |stop_time: &StopTime| {
        stop_time
            .departure_time
            .as_option()
            .or_else(|| stop_time.arrival_time.as_option())
    };
    let arrives = |stop_time: &StopTime| {
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
            _ => (offset + 1) as f64 / (last - first) as f64,
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
    use super::*;
    use crate::models::Distance;

    fn stop(arrival: Option<u32>, departure: Option<u32>, distance: Option<f32>) -> StopTime {
        StopTime {
            arrival_time: arrival.map(Time).into(),
            departure_time: departure.map(Time).into(),
            distance_traveled: distance.map(Distance).into(),
            ..StopTime::default()
        }
    }

    fn times(run: &[StopTime]) -> Vec<Option<u32>> {
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
