use bytemuck::{Pod, Zeroable};

use crate::models::{Distance, ShapePointSlice};

/// The path a vehicle takes from one stop to the next, so many trips share it.
///
/// It is not a GTFS shape: that is a whole route variant, and here it is cut at every stop. The
/// points of a `Shape` run from the vertex nearest the first stop to the vertex nearest the next
/// one. A trip holds one `Shape` for each pair of stops it calls at in a row.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Shape {
    /// Where the points are in the `shape_points` table. Never empty in a file the compiler wrote.
    pub points: ShapePointSlice,

    /// How long the path is, in meters, along its points.
    pub length: Distance,
}
