pub const GTFS_BIN_VERSION: u32 = 4;

pub mod models;

#[cfg(feature = "compiler")]
pub mod compiler;
#[cfg(feature = "consumer")]
pub mod consumer;
#[cfg(feature = "rt")]
pub mod rt;
