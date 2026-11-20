//! The types of the flatnet file, and the code that reads it back without copying anything. The
//! `flatnet-compiler` crate writes the file.
//!
//! - [`consumer`] maps the file and answers queries from it: trips by stop, stops near a position,
//!   places by name.
//! - [`rt`], behind the `gtfs-rt` feature, applies GTFS-realtime feeds on top of a consumer.
//! - [`models`] holds the fixed-size types the file is made of.
//!
//! The layout of the file is described in `docs/FORMAT.md`.

/// The version of the file layout. It changes whenever a table or header field does.
pub const FLATNET_VERSION: u32 = 15;

pub mod models;

#[cfg(feature = "consumer")]
pub mod consumer;
#[cfg(feature = "gtfs-rt")]
pub mod rt;
