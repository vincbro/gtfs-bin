use bytemuck::{Pod, Zeroable};

use crate::models::{Distance, StopIndex};

/// A walk from one place to another, which the compiler worked out from the stops' positions.
///
/// Footpaths run between places only: a place is a parent station with all the stops under it, or
/// a stop that has no parent. Walking between the stops of one place is a given and has no
/// footpath. A place is named by the index of its root stop. The footpaths of a place lie together,
/// nearest first, and `Consumer::footpaths_by_place` gives them. How they were chosen is in the
/// file's `Settings`.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Footpath {
    /// The place the walk ends at, as the index of its root stop.
    pub to_place_index: StopIndex,

    /// The straight line between the nearest stops of the two places, in meters. The file holds no
    /// detour and no walking speed: turning a line into a walk, and a walk into time, is the
    /// reader's.
    pub meters: Distance,
}
