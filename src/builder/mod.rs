use std::path::{Path, PathBuf};

use gtfs_structures::RawGtfs;

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
        let _gtfs = RawGtfs::from_path(&self.path)?;
        Ok(vec![])
    }
}
