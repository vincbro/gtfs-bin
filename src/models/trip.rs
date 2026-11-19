use crate::models::{
    Optional, RouteIndex, ServiceIndex, ShapeSlice, StopTimeSlice, StringSlice, TripIdSlice,
    TripIndex,
};
use bytemuck::{Pod, Zeroable};

/// A single GTFS trip.
///
/// Based on the GTFS standard: <https://gtfs.org/documentation/schedule/reference/#tripstxt>
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Trip {
    /// The string ID from the original GTFS file (e.g., `TRIP_123`).
    pub id: TripIdSlice,

    /// Text that appears on signage identifying the trip's destination to riders.
    pub headsign: Optional<StringSlice>,

    /// The public facing name of the trip.
    pub short_name: Optional<StringSlice>,

    pub stop_times: StopTimeSlice,

    /// Identifies a geospatial shape describing the vehicle travel path for a trip.
    pub shape: Optional<ShapeSlice>,

    pub index: TripIndex,

    pub route_index: RouteIndex,

    pub service_index: ServiceIndex,
}
