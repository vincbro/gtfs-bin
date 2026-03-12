use std::collections::HashMap;

use crate::models::{
    Distance, Opt, Sentinel, SliceBuilder, StopIdx, StopTime, StopTimeIdx, StopTimeSlice,
    StringSlice, Time, TripIdx,
};
use rayon::slice::ParallelSliceMut;

pub(crate) fn build_stop_times(
    raw_stop_times: &[gtfs_structures::RawStopTime],
    trip_map: &HashMap<String, TripIdx>,
    stop_map: &HashMap<String, StopIdx>,
    slice_builder: &mut SliceBuilder<StringSlice>,
) -> Result<(Vec<StopTime>, Vec<StopTimeSlice>), gtfs_structures::Error> {
    let mut stop_times: Vec<_> = raw_stop_times
        .iter()
        .filter_map(|stop_time| {
            trip_map
                .get(&stop_time.trip_id)
                .copied()
                .and_then(|trip_idx| {
                    stop_map
                        .get(&stop_time.stop_id)
                        .copied()
                        .map(|idx| (stop_time, trip_idx, idx))
                })
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

    let mut trip_to_stop_times = vec![StopTimeSlice::NONE; trip_map.len()];
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

    Ok((stop_times, trip_to_stop_times))
}
