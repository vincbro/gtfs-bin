use crate::models::{AreaIdSlice, AreaIdx, StringSlice};
use bytemuck::{Pod, Zeroable};

/// Represents a geographic zone or area in the GTFS dataset.
///
/// Based on the GTFS standard: https://gtfs.org/documentation/schedule/reference/#areastxt
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Area {
    // ------------------------------------------------------------------------
    // 1. LARGEST FIELDS FIRST (8 bytes)
    // ------------------------------------------------------------------------
    /// The string ID from the original GTFS file (e.g., "AREA_A").
    pub id: AreaIdSlice,

    /// The human-readable name of the area (e.g., "Downtown").
    pub name: StringSlice,

    // ------------------------------------------------------------------------
    // 2. SMALLER FIELDS LAST (4 bytes)
    // ------------------------------------------------------------------------
    /// The internal array index of this area in the file.
    pub idx: AreaIdx,
}
