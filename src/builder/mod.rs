use std::path::{Path, PathBuf};

use gtfs_structures::RawGtfs;
use rayon::slice::ParallelSliceMut;

use crate::models::{Stop, StopIdx};

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
        let stops = gtfs.stops?;
        let stop_ids: Vec<&str> = stops.iter().map(|stop| stop.id.as_ref()).collect();
        let mut lookup_vec: Vec<_> = (0..stops.len()).map(|i| StopIdx(i as u32)).collect();
        // lookup_vec.par_sort_unstable_by(|a, b| stop_ids[a.into()].cmp(stop_ids[b.into()]));
        Ok(vec![])
    }
}
