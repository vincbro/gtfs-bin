use crate::models::{Duration, Optional, StopIndex};
use bytemuck::{Pod, Zeroable};

/// A single GTFS transfer.
///
/// Based on the GTFS standard: <https://gtfs.org/documentation/schedule/reference/#transferstxt>
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Transfer {
    /// The stop the transfer starts at.
    pub from_stop_index: StopIndex,
    /// The stop the transfer ends at.
    pub to_stop_index: StopIndex,

    /// Amount of time, in seconds, that must be available to permit a transfer between routes at the specified stops.
    pub min_transfer_time: Optional<Duration>,
    /// The GTFS `transfer_type` code for the stop pair.
    pub transfer_type: u8,
    /// Always zero. Makes the layout free of implicit padding, which `Pod` requires.
    pub padding: [u8; 3],
}

impl Transfer {
    #[must_use]
    pub const fn new(
        from_stop_index: StopIndex,
        to_stop_index: StopIndex,
        min_transfer_time: Optional<Duration>,
        transfer_type: u8,
    ) -> Self {
        Self {
            from_stop_index,
            to_stop_index,
            min_transfer_time,
            transfer_type,
            padding: [0_u8; 3],
        }
    }
}
