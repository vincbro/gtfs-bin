use bytemuck::{Pod, Zeroable};

use crate::models::{Coordinate, LocationType, Optional, StopIdSlice, StopIndex, StringSlice};

/// What routing reads about a GTFS stop, station, or entrance. The text a person reads is in the
/// `StopInfo` at the same position, so a scan over the stops doesn't read it.
///
/// Based on the GTFS standard: <https://gtfs.org/documentation/schedule/reference/#stopstxt>.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Stop {
    /// The geographic location of the stop.
    pub coordinate: Optional<Coordinate>,

    /// The string ID from the original GTFS file (e.g., `STOP_123`).
    pub id: StopIdSlice,

    pub index: StopIndex,

    pub parent_index: Optional<StopIndex>,

    pub location_type: LocationType,

    /// Always zero. Makes the layout free of implicit padding, which `Pod` requires.
    pub padding: [u8; 3],
}

impl Stop {
    #[must_use]
    pub const fn new(
        id: StopIdSlice,
        index: StopIndex,
        parent_index: Optional<StopIndex>,
        coordinate: Optional<Coordinate>,
        location_type: LocationType,
    ) -> Self {
        Self {
            coordinate,
            id,
            index,
            parent_index,
            location_type,
            padding: [0_u8; 3],
        }
    }
}

/// The text of a stop, for a person to read. The `StopInfo` of a stop lies at the stop's position.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct StopInfo {
    /// Short text or a number that identifies the stop to passengers.
    pub code: Optional<StringSlice>,

    /// The name of the location.
    pub name: Optional<StringSlice>,

    /// A description of the location that provides useful, quality-of-life information.
    pub description: Optional<StringSlice>,
}
