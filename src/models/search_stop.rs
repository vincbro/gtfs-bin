use bytemuck::{Pod, Zeroable};

use crate::models::{RouteType, SearchIdx, SearchSlice, StringSlice};

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct SearchStop {
    // ------------------------------------------------------------------------
    // 1. LARGEST FIELDS FIRST (8 to 16 bytes)
    // ------------------------------------------------------------------------
    pub idx: SearchIdx,

    pub name: StringSlice,

    pub stops: SearchSlice,
    // ------------------------------------------------------------------------
    // 2. SMALLER FIELDS LAST (4 bytes)
    // ------------------------------------------------------------------------
    pub route_type: RouteType,
    pad: [u8; 2],
}

impl SearchStop {
    #[must_use]
    pub const fn new(
        idx: SearchIdx,
        name: StringSlice,
        stops: SearchSlice,
        route_type: RouteType,
    ) -> Self {
        Self {
            idx,
            name,
            stops,
            route_type,
            pad: [0_u8, 2],
        }
    }
}
