use crate::models::{Duration, Opt, StopIdx};
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

    /// Amount of time, in seconds, that must be available to permit a transfer between routes at the specified stops.
    pub min_transfer_time: Opt<Duration>,
    /// Indicates the type of connection for the specified pair.
    pub transfer_type: u8,
    pub _pad: [u8; 3], // Pad to 32 bytes for perfect alignment
}
