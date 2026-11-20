use bytemuck::{Pod, Zeroable};

use crate::models::{BoardingRule, Distance, Optional, StringSlice};

/// What a pattern says about one of its stops, the same for every trip that follows it.
///
/// The stop itself is in `trip_patterns_stop_seq` at the same position. Trips that agree on the stops
/// but not on this are in different patterns.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct PatternStop {
    /// Text that appears on signage identifying the trip's destination to riders.
    pub headsign: Optional<StringSlice>,

    /// Order of stops. It increases along a pattern.
    pub sequence: u32,

    /// Actual distance traveled along the associated shape.
    pub distance_traveled: Optional<Distance>,

    pub pickup: BoardingRule,

    pub drop_off: BoardingRule,

    /// Always zero. Makes the layout free of implicit padding, which `Pod` requires.
    pub padding: [u8; 2],
}
