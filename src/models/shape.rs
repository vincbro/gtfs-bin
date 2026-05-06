use crate::models::{Coordinate, Distance, Opt};
use bytemuck::{Pod, Zeroable};

/// Represents a single point in a shape in the GTFS dataset.
///
/// Based on the GTFS standard: <https://gtfs.org/documentation/schedule/reference/#shapestxt>
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Shape {
    // ------------------------------------------------------------------------
    // 1. LARGEST FIELDS FIRST (8 bytes)
    // ------------------------------------------------------------------------
    /// The geographic point of the shape.
    pub coordinate: Coordinate,

    // ------------------------------------------------------------------------
    // 2. SMALLER FIELDS LAST (4 bytes)
    // ------------------------------------------------------------------------
    /// Actual distance traveled along the shape from the first shape point to the point specified in this record.
    pub distance_traveled: Opt<Distance>,
}
