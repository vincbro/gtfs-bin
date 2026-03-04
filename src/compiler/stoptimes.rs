use crate::{
    compiler::Compiler,
    models::{
        Distance, Opt, Sentinel, Slice, SliceBuilder, StopIdx, StopTime, StopTimeIdx,
        StopTimeSlice, StringSlice, Time, TripIdx,
    },
};
use rayon::slice::ParallelSliceMut;

impl Compiler {
    pub(crate) fn build_stop_times(
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
}
