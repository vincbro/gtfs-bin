use bytemuck::{Pod, Zeroable};

use crate::models::{Coordinate, Distance, Optional};

/// Represents a single point in a shape in the GTFS dataset.
///
/// Based on the GTFS standard: <https://gtfs.org/documentation/schedule/reference/#shapestxt>.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Shape {
    /// The geographic point of the shape.
    pub coordinate: Coordinate,

    /// Actual distance traveled along the shape from the first shape point to the point specified in this record.
    pub distance_traveled: Optional<Distance>,
}
