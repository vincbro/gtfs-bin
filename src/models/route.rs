use crate::models::{Opt, RouteIdSlice, RouteIdx, RouteType, StringSlice};
use bytemuck::{Pod, Zeroable};

/// A single GTFS route.
///
/// Based on the GTFS standard: <https://gtfs.org/documentation/schedule/reference/#routestxt>
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Route {
    // ------------------------------------------------------------------------
    // 1. LARGEST FIELDS FIRST (8 to 16 bytes)
    // ------------------------------------------------------------------------
    /// The string ID from the original GTFS file (e.g., ``STOP_123``).
    pub id: RouteIdSlice,

    /// Full name of a route.
    pub long_name: Opt<StringSlice>,

    /// Short name of a route.
    pub short_name: Opt<StringSlice>,

    /// Description of a route that provides useful, quality information.
    pub description: Opt<StringSlice>,

    // ------------------------------------------------------------------------
    // 2. SMALLER FIELDS LAST (4 bytes)
    // ------------------------------------------------------------------------
    /// The internal array index of this route in the file.
    pub idx: RouteIdx,

    ///Indicates the type of transportation used on a route
    pub route_type: RouteType,
    pad: [u8; 2],
}

impl Route {
    #[must_use]
    pub const fn new(
        id: RouteIdSlice,
        idx: RouteIdx,
        long_name: Opt<StringSlice>,
        short_name: Opt<StringSlice>,
        description: Opt<StringSlice>,
        route_type: RouteType,
    ) -> Self {
        Self {
            id,
            long_name,
            short_name,
            description,
            idx,
            route_type,
            pad: [0_u8, 2],
        }
    }
}
