pub const GTFS_BIN_VERSION: u32 = 3;

pub mod compiler;
pub mod consumer;
pub mod models;

#[cfg(feature = "rt")]
pub mod rt;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("File is too small to contain a valid header")]
    FileTooSmall,
    #[error("Section out of bounds")]
    SectionOutOfBound,
    #[error("Invalid magic number: Not a compiled GTFS file")]
    InvalidMagic,
    #[error("Unsupported GTFS binary version: expected {expected}, got {actual}")]
    UnsupportedVersion { expected: u32, actual: u32 },
}
