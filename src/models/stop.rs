use crate::models::{Coordinate, Opt, StopIdSlice, StopIdx, StringSlice};
use bytemuck::{Pod, Zeroable};

/// A single GTFS stop, station, or entrance.
///
/// Based on the GTFS standard: https://gtfs.org/documentation/schedule/reference/#stopstxt
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Stop {
    // ------------------------------------------------------------------------
    // 1. LARGEST FIELDS FIRST (8 to 16 bytes)
    // ------------------------------------------------------------------------
    /// The geographic location of the stop.
    pub coordinate: Opt<Coordinate>,

    /// The string ID from the original GTFS file (e.g., "STOP_123").
    pub id: StopIdSlice,

    /// Short text or a number that identifies the stop to passengers.
    pub code: Opt<StringSlice>,

    /// The name of the location.
    pub name: Opt<StringSlice>,

    /// A description of the location that provides useful, quality-of-life information.
    pub desc: Opt<StringSlice>,

    // ------------------------------------------------------------------------
    // 2. SMALLER FIELDS LAST (4 bytes)
    // ------------------------------------------------------------------------
    /// The internal array index of this stop in the file.
    pub idx: StopIdx,

    /// The internal array index of the parent station, if one exists.
    pub parent_idx: Opt<StopIdx>,
}
