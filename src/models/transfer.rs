use crate::models::{Opt, RouteIdx, ServiceIdx, StopIdx, StringSlice, TripIdSlice, TripIdx};
use bytemuck::{Pod, Zeroable};

/// A single GTFS transfer.
///
/// Based on the GTFS standard: https://gtfs.org/documentation/schedule/reference/#transferstxt
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Transfer {
    // ------------------------------------------------------------------------
    // 1. LARGEST FIELDS FIRST (8 to 16 bytes)
    // ------------------------------------------------------------------------

    // ------------------------------------------------------------------------
    // 2. SMALLER FIELDS LAST (4 bytes)
    // ------------------------------------------------------------------------
    /// Identifies a stop
    pub from_stop_idx: StopIdx,
    /// Identifies a stop
    pub to_stop_idx: StopIdx,

    /// Identifies a route where a connection begins.
    pub from_route_idx: Opt<RouteIdx>,
    /// Identifies a route where a connection ends.
    pub to_route_idx: Opt<RouteIdx>,

    /// Identifies a trip where a connection between routes begins.
    pub from_trip_idx: Opt<TripIdx>,
    /// Identifies a trip where a connection between routes ends.
    pub to_trip_idx: Opt<TripIdx>,

    /// Amount of time, in seconds, that must be available to permit a transfer between routes at the specified stops.
    pub min_transfer_time: u32,
    /// Indicates the type of connection for the specified pair.
    pub transfer_type: u8,
    pub _pad: [u8; 3], // Pad to 32 bytes for perfect alignment
}
