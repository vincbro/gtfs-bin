use crate::models::{Optional, RouteIdSlice, RouteIndex, RouteType, StringSlice};
use bytemuck::{Pod, Zeroable};

/// A single GTFS route.
///
/// Based on the GTFS standard: <https://gtfs.org/documentation/schedule/reference/#routestxt>
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Route {
    /// The string ID from the original GTFS file (e.g., `ROUTE_123`).
    pub id: RouteIdSlice,

    /// Full name of a route.
    pub long_name: Optional<StringSlice>,

    /// Short name of a route.
    pub short_name: Optional<StringSlice>,

    /// Description of a route that provides useful, quality information.
    pub description: Optional<StringSlice>,

    pub index: RouteIndex,

    pub route_type: RouteType,
    /// Always zero. Makes the layout free of implicit padding, which `Pod` requires.
    pub padding: [u8; 2],
}

impl Route {
    #[must_use]
    pub const fn new(
        id: RouteIdSlice,
        index: RouteIndex,
        long_name: Optional<StringSlice>,
        short_name: Optional<StringSlice>,
        description: Optional<StringSlice>,
        route_type: RouteType,
    ) -> Self {
        Self {
            id,
            long_name,
            short_name,
            description,
            index,
            route_type,
            padding: [0_u8; 2],
        }
    }
}
