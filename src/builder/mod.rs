use std::path::{Path, PathBuf};

use gtfs_structures::RawGtfs;
use rayon::slice::ParallelSliceMut;

use crate::models::{
    Coordinate, Opt, Sentinel, Slice, SliceBuilder, Stop, StopIdSlice, StopIdx, StringSlice,
};

/// Builds the .gtfs file
#[derive(Debug, Default, Clone)]
pub struct Builder {
    path: PathBuf,
}

impl Builder {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn build(&self) -> Result<Vec<u8>, gtfs_structures::Error> {
        let gtfs = RawGtfs::from_path(&self.path)?;
        let raw_stops = gtfs.stops?;
        let _stops = self.build_stops(&raw_stops)?;
        Ok(vec![])
    }

    fn build_stops(
        &self,
        raw_stops: &[gtfs_structures::Stop],
    ) -> Result<Vec<Stop>, gtfs_structures::Error> {
        // Recomended max length for a gtfs id is 36 characters
        let mut id_builder = SliceBuilder::with_capacity(36 * raw_stops.len());
        let mut str_builder = SliceBuilder::new();

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
                    code: stop.code.as_ref().map(|code| str_builder.add(code)).into(),
                    name: stop.name.as_ref().map(|name| str_builder.add(name)).into(),
                    description: stop
                        .description
                        .as_ref()
                        .map(|desc| str_builder.add(desc))
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

        // Add parent station logic
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

        Ok(stops)
    }
}
