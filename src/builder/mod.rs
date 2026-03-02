use std::path::{Path, PathBuf};

use gtfs_structures::RawGtfs;
use rayon::slice::ParallelSliceMut;

use crate::models::{Coordinate, Opt, Sentinel, Slice, Stop, StopIdSlice, StopIdx, StringSlice};

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
        let mut stop_ids = String::with_capacity(36 * raw_stops.len());
        let mut stops: Vec<_> = raw_stops
            .iter()
            .map(|stop| {
                let id = StopIdSlice {
                    start: stop_ids.len() as u32,
                    count: stop.id.len() as u32,
                };
                stop_ids.push_str(&stop.id);
                let coordinate = if let Some(lat) = stop.latitude
                    && let Some(lon) = stop.longitude
                {
                    Opt::new(Coordinate::new(lat, lon))
                } else {
                    Opt::new(Coordinate::NONE)
                };
                Stop {
                    coordinate,
                    id,
                    code: Opt::new(StringSlice::NONE),
                    name: Opt::new(StringSlice::NONE),
                    desc: Opt::new(StringSlice::NONE),
                    idx: StopIdx::NONE,
                    parent_idx: Opt::new(StopIdx::NONE),
                }
            })
            .collect();
        //TODO: ADD SORT STOPS LOGIC GOES HERE
        // Set stop idx
        stops
            .iter_mut()
            .enumerate()
            .for_each(|(i, stop)| stop.idx = StopIdx(i as u32));
        let mut stop_id_lookup: Vec<_> = (0..stops.len()).map(|i| StopIdx(i as u32)).collect();
        stop_id_lookup.par_sort_unstable_by(|a, b| {
            let id_a = &stop_ids[stops[a.to_usize()].id.range()];
            let id_b = &stop_ids[stops[b.to_usize()].id.range()];
            id_a.cmp(id_b)
        });
        Ok(stops)
    }
}
