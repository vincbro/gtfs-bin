/// The version of the file layout. It changes whenever a table or header field does.
pub const GTFS_BIN_VERSION: u32 = 5;

pub mod models;

#[cfg(feature = "compiler")]
pub mod compiler;
#[cfg(feature = "consumer")]
pub mod consumer;
#[cfg(feature = "rt")]
pub mod rt;
