use bytemuck::{Pod, Zeroable};

use crate::models::{
    Direction, Optional, RouteIndex, ServiceIndex, StopTimeSlice, StringSlice, TripIdSlice,
    TripIndex, TripShapeSlice,
};

/// What routing reads about a GTFS trip. The text is in the `TripInfo` at the same position.
///
/// Based on the GTFS standard: <https://gtfs.org/documentation/schedule/reference/#tripstxt>.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Trip {
    /// The string ID from the original GTFS file (e.g., `TRIP_123`).
    pub id: TripIdSlice,

    pub stop_times: StopTimeSlice,

    /// The shapes the trip travels, one for each pair of stops in a row, so one fewer than it has
    /// stop times. A slice of the `trip_shapes` table. Absent when the feed gave the trip no shape.
    pub shape: Optional<TripShapeSlice>,

    pub index: TripIndex,

    pub route_index: RouteIndex,

    pub service_index: ServiceIndex,

    pub direction: Direction,

    /// Always zero. Makes the layout free of implicit padding, which `Pod` requires.
    pub padding: [u8; 3],
}

/// The text and the block of a trip, for a person or a vehicle scheduler. The `TripInfo` of a trip
/// lies at the trip's position.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct TripInfo {
    /// Text that appears on signage identifying the trip's destination to riders.
    pub headsign: Optional<StringSlice>,

    /// The public facing name of the trip.
    pub short_name: Optional<StringSlice>,

    /// The id trips of the same vehicle block share, so a vehicle can continue as the next trip.
    pub block: Optional<StringSlice>,
}
