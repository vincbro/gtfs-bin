use crate::{
    compiler::Compiler,
    models::{
        Coordinate, Opt, Sentinel, Slice, SliceBuilder, Stop, StopIdx, StringSlice, TripIdx,
        TripSlice,
    },
};
use rayon::slice::ParallelSliceMut;

impl Compiler {
    pub(crate) fn build_stops(
        &mut self,
        raw_stops: &[gtfs_structures::Stop],
        slice_builder: &mut SliceBuilder<StringSlice>,
    ) -> Result<(), gtfs_structures::Error> {
        // Recomended max length for a gtfs id is 36 characters
        let mut id_builder = SliceBuilder::with_capacity(36 * raw_stops.len());

        // Convert raw_stops to stops
        let mut stops: Vec<_> = raw_stops
            .iter()
            .enumerate()
            .map(|(i, stop)| {
                let coordinate = if let Some(lat) = stop.latitude
                    && let Some(lon) = stop.longitude
                {
                    Opt::new(Coordinate::new(lat, lon))
                } else {
                    Opt::new(Coordinate::NONE)
                };
                Stop {
                    coordinate,
                    id: id_builder.add(&stop.id),
                    code: stop
                        .code
                        .as_ref()
                        .map(|code| slice_builder.add(code))
                        .into(),
                    name: stop
                        .name
                        .as_ref()
                        .map(|name| slice_builder.add(name))
                        .into(),
                    description: stop
                        .description
                        .as_ref()
                        .map(|desc| slice_builder.add(desc))
                        .into(),
                    idx: StopIdx(i as u32),
                    parent_idx: Opt::new(StopIdx::NONE),
                }
            })
            .collect();

        let stop_ids = id_builder.take();

        // Build binary search friendly id lookup
        let mut stop_id_lookup: Vec<_> = (0..stops.len()).map(|i| StopIdx(i as u32)).collect();
        stop_id_lookup.par_sort_unstable_by(|a, b| {
            let id_a = &stop_ids[stops[a.to_usize()].id.range()];
            let id_b = &stop_ids[stops[b.to_usize()].id.range()];
            id_a.cmp(id_b)
        });

        // Map parent stations
        raw_stops
            .iter()
            .enumerate()
            .filter_map(|(i, raw_stop)| raw_stop.parent_station.clone().map(|pt_id| (i, pt_id)))
            .for_each(|(i, pt_id)| {
                let result = stop_id_lookup.binary_search_by(|&idx| {
                    let stop = &stops[idx.to_usize()];
                    let current_id = &stop_ids[stop.id.range()];
                    current_id.cmp(&pt_id)
                });

                if let Ok(idx) = result {
                    stops[i].parent_idx = Opt::new(StopIdx(idx as u32));
                }
            });

        self.stops = stops;
        self.stop_id_lookup = stop_id_lookup;
        self.stop_ids = stop_ids;

        Ok(())
    }

    pub(crate) fn build_stop_to_trips(&mut self) {
        let mut stop_trip_pairs: Vec<(StopIdx, TripIdx)> = self
            .stop_times
            .iter()
            .map(|st| (st.stop_idx, st.trip_idx))
            .collect();

        stop_trip_pairs.par_sort_unstable();

        stop_trip_pairs.dedup();

        let mut stop_to_trips = vec![TripSlice::NONE; self.stops.len()];
        let mut stop_to_trips_lookup = Vec::with_capacity(stop_trip_pairs.len());

        let mut current_stop = StopIdx::NONE;
        let mut start = 0;
        let mut count = 0;

        for (i, &(stop_idx, trip_idx)) in stop_trip_pairs.iter().enumerate() {
            if stop_idx != current_stop {
                if current_stop != StopIdx::NONE {
                    stop_to_trips[current_stop.to_usize()] = TripSlice { start, count };
                }
                start = i as u32;
                count = 0;
                current_stop = stop_idx;
            }
            stop_to_trips_lookup.push(trip_idx);
            count += 1;
        }

        if current_stop != StopIdx::NONE {
            stop_to_trips[current_stop.to_usize()] = TripSlice { start, count };
        }

        self.stop_to_trips = stop_to_trips;
        self.stop_to_trips_lookup = stop_to_trips_lookup;
    }
}
