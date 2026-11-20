use bytemuck::{Pod, Zeroable};

use crate::models::{RouteType, SearchIndex, SearchSlice, StringSlice};

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct SearchStop {
    pub index: SearchIndex,

    pub name: StringSlice,

    pub stops: SearchSlice,
    pub route_type: RouteType,
    /// Always zero. Makes the layout free of implicit padding, which `Pod` requires.
    pub padding: [u8; 2],
}

impl SearchStop {
    #[must_use]
    pub const fn new(
        index: SearchIndex,
        name: StringSlice,
        stops: SearchSlice,
        route_type: RouteType,
    ) -> Self {
        Self {
            index,
            name,
            stops,
            route_type,
            padding: [0_u8; 2],
        }
    }
}
