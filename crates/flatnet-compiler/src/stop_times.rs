use flatnet_core::models::{Optional, Slice, StopTimeSlice, Time, Trip, TripIndex};
use rayon::slice::ParallelSliceMut;

use crate::{network::Call, tables};

/// Orders the calls by trip and sequence, gives each trip its run of them, and fills in the times
/// of the stops between two that have one.
///
/// A call with no times between two that have them gets a time spread evenly between them, by
/// distance along the shape when the source has it. The GTFS spec asks for exactly that, and
/// every source gains from a router that can rely on a time at each stop.
pub(super) fn prepare_calls(calls: &mut [Call], trips: &mut [Trip]) {
    calls.par_sort_unstable_by(|first, second| {
        first
            .trip
            .cmp(&second.trip)
            .then(first.sequence.cmp(&second.sequence))
            .then(first.stop.cmp(&second.stop))
    });

    let slices: Vec<StopTimeSlice> = tables::slices_by_key(
        calls.iter().map(|call| TripIndex::from(call.trip)),
        trips.len(),
    );
    for (trip, slice) in trips.iter_mut().zip(slices) {
        trip.stop_times = slice;
        interpolate_times(&mut calls[slice.range()]);
    }
}

/// Fills in the times of stops that have none, between two stops that do.
fn interpolate_times(run: &mut [Call]) {
    let timed: Vec<usize> = (0..run.len())
        .filter(|&position| run[position].arrival.is_some() || run[position].departure.is_some())
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
fn fill_between(run: &mut [Call], first: usize, last: usize) {
    let leaves = |stop_time: &Call| {
        stop_time
            .departure
            .as_option()
            .or_else(|| stop_time.arrival.as_option())
    };
    let arrives = |stop_time: &Call| {
        stop_time
            .arrival
            .as_option()
            .or_else(|| stop_time.departure.as_option())
    };
    let (Some(start), Some(end)) = (leaves(&run[first]), arrives(&run[last])) else {
        return;
    };
    // A feed whose times run backwards has nothing sensible to spread.
    if end.0 < start.0 {
        return;
    }

    let distances = (
        run[first].distance.as_option(),
        run[last].distance.as_option(),
    );
    let span = f64::from(end.0 - start.0);
    for (offset, stop_time) in run[first + 1..last].iter_mut().enumerate() {
        let fraction = match (distances, stop_time.distance.as_option()) {
            ((Some(from), Some(to)), Some(at)) if to.0 > from.0 => {
                f64::from((at.0 - from.0) / (to.0 - from.0)).clamp(0.0, 1.0)
            }
            _ => share(offset + 1, last - first),
        };
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the result is between the two times, so it fits in u32"
        )]
        let time = Optional::new(Time(start.0 + (span * fraction).round() as u32));
        stop_time.arrival = time;
        stop_time.departure = time;
    }
}

#[cfg(test)]
mod tests {
    use flatnet_core::models::{BoardingRule, Distance};

    use super::*;

    fn stop(arrival: Option<u32>, departure: Option<u32>, distance: Option<f32>) -> Call {
        Call {
            trip: 0,
            stop: 0,
            sequence: 0,
            arrival: arrival.map(Time).into(),
            departure: departure.map(Time).into(),
            headsign: None,
            distance: distance.map(Distance).into(),
            pickup: BoardingRule::REGULAR,
            drop_off: BoardingRule::REGULAR,
        }
    }

    fn times(run: &[Call]) -> Vec<Option<u32>> {
        run.iter()
            .map(|stop_time| stop_time.departure.as_option().map(|time| time.0))
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
