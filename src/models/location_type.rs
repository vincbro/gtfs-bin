use bytemuck::{Pod, Zeroable};

use crate::models::Sentinel;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Pod, Zeroable)]
pub struct LocationType(u8);

impl From<u8> for LocationType {
    /// Converts a GTFS `location_type` code. Unknown codes become `NONE`.
    fn from(code: u8) -> Self {
        match code {
            0 => Self::STOP_POINT,
            1 => Self::STOP_AREA,
            2 => Self::STATION_ENTRANCE,
            3 => Self::GENERIC_NODE,
            4 => Self::BOARDING_AREA,
            _ => Self::NONE,
        }
    }
}

impl Sentinel for LocationType {
    const NONE: Self = Self(u8::MAX);
}

impl Default for LocationType {
    fn default() -> Self {
        Self::NONE
    }
}

impl LocationType {
    pub const STOP_POINT: Self = Self(0);
    pub const STOP_AREA: Self = Self(1);
    pub const STATION_ENTRANCE: Self = Self(2);
    pub const GENERIC_NODE: Self = Self(3);
    pub const BOARDING_AREA: Self = Self(4);
}
