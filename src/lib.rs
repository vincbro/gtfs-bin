//! Compiles GTFS transit feeds into one memory-mappable file, and reads that file back without
//! copying anything.
//!
//! - [`compiler`] reads a feed, a directory or a zip, and writes the file.
//! - [`consumer`] maps the file and answers queries from it: trips by stop, stops near a position,
//!   places by name.
//! - [`rt`] applies GTFS-realtime feeds on top of a consumer.
//! - [`models`] holds the fixed-size types the file is made of.
//!
//! The layout of the file is described in `docs/FORMAT.md`.

/// The version of the file layout. It changes whenever a table or header field does.
pub const GTFS_BIN_VERSION: u32 = 5;

pub mod models;

#[cfg(feature = "compiler")]
pub mod compiler;
#[cfg(feature = "consumer")]
pub mod consumer;
#[cfg(feature = "rt")]
pub mod rt;
