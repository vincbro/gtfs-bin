use bytemuck::{Pod, Zeroable};

use crate::models::Sentinel;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Pod, Zeroable)]
pub struct LocationType(u8);

impl From<gtfs_structures::LocationType> for LocationType {
    fn from(value: gtfs_structures::LocationType) -> Self {
        match value {
            gtfs_structures::LocationType::StopPoint => Self::STOP_POINT,
            gtfs_structures::LocationType::StopArea => Self::STOP_AREA,
            gtfs_structures::LocationType::StationEntrance => Self::STATION_ENTRANCE,
            gtfs_structures::LocationType::GenericNode => Self::GENERIC_NODE,
            gtfs_structures::LocationType::BoardingArea => Self::BOARDING_AREA,
            gtfs_structures::LocationType::Unknown(_) => Self(u8::MAX),
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
