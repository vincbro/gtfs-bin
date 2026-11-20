use bytemuck::{Pod, Zeroable};

use crate::models::{StopSlice, TripPatternIndex, TripSlice};

/// The trips that call at the same stops in the same order, with the same sequence numbers, boarding
/// rules, headsigns and distances, and that never overtake each other: at every stop, a trip that
/// comes later in the pattern is never earlier than one before it.
///
/// This is not a GTFS concept: the compiler derives it from the stop times. The trips are in the
/// order they leave, and their timings lie back to back in the same order.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct TripPattern {
    /// The order of stops that define this trip pattern.
    pub stops: StopSlice,

    /// The trips of the pattern, in the order they leave.
    pub trips: TripSlice,

    pub index: TripPatternIndex,
}
