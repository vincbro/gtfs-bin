use std::result;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read gtfs data")]
    Gtfs(#[from] gtfs_structures::Error),
}

pub type Result<T> = result::Result<T, Error>;
