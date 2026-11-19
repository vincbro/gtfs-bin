use bytemuck::{Pod, Zeroable};

use crate::models::{Duration, Optional, StopIndex, TripIndex};

/// A single GTFS transfer.
///
/// Based on the GTFS standard: <https://gtfs.org/documentation/schedule/reference/#transferstxt>
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Transfer {
    pub from_stop_index: StopIndex,
    pub to_stop_index: StopIndex,

    /// If set, the transfer only applies when arriving on this trip.
    pub from_trip_index: Optional<TripIndex>,
    /// If set, the transfer only applies when leaving on this trip.
    pub to_trip_index: Optional<TripIndex>,

    /// Amount of time, in seconds, that must be available to permit a transfer between routes at the specified stops.
    pub min_transfer_time: Optional<Duration>,
    /// The GTFS `transfer_type` code. 4 and 5 mean the passenger stays in the vehicle, which
    /// continues as the next trip.
    pub transfer_type: u8,
    /// Always zero. Makes the layout free of implicit padding, which `Pod` requires.
    pub padding: [u8; 3],
}

impl Transfer {
    #[must_use]
    pub const fn new(
        from_stop_index: StopIndex,
        to_stop_index: StopIndex,
        from_trip_index: Optional<TripIndex>,
        to_trip_index: Optional<TripIndex>,
        min_transfer_time: Optional<Duration>,
        transfer_type: u8,
    ) -> Self {
        Self {
            from_stop_index,
            to_stop_index,
            from_trip_index,
            to_trip_index,
            min_transfer_time,
            transfer_type,
            padding: [0_u8; 3],
        }
    }
}
