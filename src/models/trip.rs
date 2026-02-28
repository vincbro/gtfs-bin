use crate::models::{Opt, RouteIdx, ServiceIdx, StringSlice, TripIdSlice, TripIdx};
use bytemuck::{Pod, Zeroable};

/// A single GTFS trip.
///
/// Based on the GTFS standard: https://gtfs.org/documentation/schedule/reference/#tripstxt
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Trip {
    // ------------------------------------------------------------------------
    // 1. LARGEST FIELDS FIRST (8 to 16 bytes)
    // ------------------------------------------------------------------------
    /// The string ID from the original GTFS file (e.g., "STOP_123").
    pub id: TripIdSlice,

    /// Text that appears on signage identifying the trip's destination to riders.
    pub headsign: Opt<StringSlice>,

    /// The public facing name of the trip.
    pub short_name: Opt<StringSlice>,

    // ------------------------------------------------------------------------
    // 2. SMALLER FIELDS LAST (4 bytes)
    // ------------------------------------------------------------------------
    /// The internal array index of this trip in the file.
    pub idx: TripIdx,

    /// The internal array index of the route in the file.
    pub route_idx: RouteIdx,

    /// The internal array index of the service in the file.
    pub service_idx: ServiceIdx,
}
