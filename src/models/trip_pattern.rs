use bytemuck::{Pod, Zeroable};

use crate::models::{StopSlice, TripPatternIndex, TripSlice};

/// A distinct sequence of stops, shared by every trip that visits those stops in that order.
///
/// This is not a GTFS concept: the compiler derives it from the stop times.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct TripPattern {
    /// The order of stops that define this trip pattern.
    pub stops: StopSlice,

    /// All the trips that share this stop seq.
    pub trips: TripSlice,

    pub index: TripPatternIndex,
}
