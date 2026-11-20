use bytemuck::{Pod, Zeroable};

use crate::models::{Optional, Time};

/// When a trip arrives at and leaves one of the stops of its pattern.
///
/// A trip's timings are the run of the `timings` table that `Trip::stop_times` names, one for each
/// stop of its pattern in order. What does not change from trip to trip, such as the stop, is in
/// the pattern's `PatternStop`.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct Timing {
    /// Arrival time at the stop.
    pub arrival: Optional<Time>,

    /// Departure time from the stop.
    pub departure: Optional<Time>,
}
