use crate::models::{StopSlice, TripPatternIdx, TripSlice};
use bytemuck::{Pod, Zeroable};

/// A single GTFS trip.
///
/// Based on the GTFS standard: https://gtfs.org/documentation/schedule/reference/#tripstxt
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct TripPattern {
    // ------------------------------------------------------------------------
    // 1. LARGEST FIELDS FIRST (8 to 16 bytes)
    // ------------------------------------------------------------------------
    /// The order of stops that define this trip pattern.
    pub stops: StopSlice,

    /// All the trips that share this stop seq.
    pub trips: TripSlice,

    // ------------------------------------------------------------------------
    // 2. SMALLER FIELDS LAST (4 bytes)
    // ------------------------------------------------------------------------
    /// The internal array index of this trip pattern in the file.
    pub idx: TripPatternIdx,
}
