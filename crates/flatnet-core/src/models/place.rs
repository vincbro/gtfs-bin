use bytemuck::{Pod, Zeroable};

use crate::models::{FootpathSlice, PlaceStopSlice, Stop, StopIndex};

/// What a place holds: its stops and the walks that start at it. There is one for every stop, at the
/// stop's position, and it is empty unless the stop is a place, the root of its parent chain.
///
/// A place has the stops a passenger can board at: its own if it has no parent, or those under it.
/// A station, an entrance and a generic node are not among them.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Place {
    /// The stops of the place, a run of the `place_stops` table, in index order.
    pub stops: PlaceStopSlice,

    /// The walks to other places, a run of the `footpaths` table, nearest first.
    pub footpaths: FootpathSlice,
}

impl Place {
    /// How many parents a stop is followed up. Real feeds nest three deep (a boarding area in a stop
    /// point in a station), and the limit stops a damaged file whose parents loop.
    pub const MAX_PARENT_DEPTH: usize = 8;

    /// Returns the place a stop belongs to: the stop itself if it has no parent, otherwise the top of
    /// its parent chain. The compiler and the consumer both use this, so they agree.
    ///
    /// # Panics
    ///
    /// Panics if `index` or a parent in the chain is out of range of `stops`.
    #[must_use]
    pub fn root_of(stops: &[Stop], index: StopIndex) -> StopIndex {
        let mut root = index;
        for _ in 0..Self::MAX_PARENT_DEPTH {
            match stops[root.as_usize()].parent_index.as_option() {
                Some(parent) => root = parent,
                None => break,
            }
        }
        root
    }
}
