pub const GTFS_BIN_VERSION: u32 = 1;

pub mod compiler;
pub mod consumer;
pub mod models;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("File is too small to contain a valid header")]
    FileTooSmall,
    #[error("Invalid magic number: Not a compiled GTFS file")]
    InvalidMagic,
    #[error("Unsupported GTFS binary version: expected {expected}, got {actual}")]
    UnsupportedVersion { expected: u32, actual: u32 },
}
