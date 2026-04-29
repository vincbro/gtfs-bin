use bytemuck::{Pod, Zeroable};

use crate::models::Sentinel;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Pod, Zeroable)]
pub struct RouteType(u16);

impl From<gtfs_structures::RouteType> for RouteType {
    fn from(value: gtfs_structures::RouteType) -> Self {
        match value {
            gtfs_structures::RouteType::Tramway => Self::TRAM,
            gtfs_structures::RouteType::Subway => Self::SUBWAY,
            gtfs_structures::RouteType::Rail => Self::RAIL,
            gtfs_structures::RouteType::Bus => Self::BUS,
            gtfs_structures::RouteType::Ferry => Self::FERRY,
            gtfs_structures::RouteType::CableCar => Self::CABLE_CAR,
            gtfs_structures::RouteType::Gondola => Self::GONDOLA,
            gtfs_structures::RouteType::Funicular => Self::FUNICULAR,
            gtfs_structures::RouteType::Coach => Self::COACH,
            gtfs_structures::RouteType::Air => Self::AIR,
            gtfs_structures::RouteType::Taxi => Self::TAXI,

            // From googles documentation about extended GTFS route types: https://developers.google.com/transit/gtfs/reference/extended-route-types
            gtfs_structures::RouteType::Other(code) => match code {
                // 100-199: Railway Service | 300-399: Suburban Railway
                100..=199 | 300..=399 => Self::RAIL,

                // 200-299: Coach Service
                200..=299 => Self::COACH,

                // 400-699: Urban/Metro/Underground Railway
                400..=699 => Self::SUBWAY,

                // 700-899: Bus and Trolleybus
                700..=899 => Self::BUS,

                // 900-999: Tram Service
                900..=999 => Self::TRAM,

                // 1000-1099: Water Transport | 1200-1299: Ferry Service
                1000..=1099 | 1200..=1299 => Self::FERRY,

                // 1100-1199: Air Service
                1100..=1199 => Self::AIR,

                // 1300-1399: Aerial Lift Service (Telecabin)
                1300..=1399 => Self::GONDOLA,

                // 1400-1499: Funicular Service
                1400..=1499 => Self::FUNICULAR,

                // 1500-1599: Taxi Service
                1500..=1599 => Self::TAXI,
                _ => Self(0),
            },
        }
    }
}
impl Sentinel for RouteType {
    const NONE: Self = Self(0);
}

impl Default for RouteType {
    fn default() -> Self {
        Self::NONE
    }
}

impl RouteType {
    pub const TRAM: Self = Self(1 << 0);
    pub const SUBWAY: Self = Self(1 << 1);
    pub const RAIL: Self = Self(1 << 2);
    pub const BUS: Self = Self(1 << 3);
    pub const FERRY: Self = Self(1 << 4);
    pub const CABLE_CAR: Self = Self(1 << 5);
    pub const GONDOLA: Self = Self(1 << 6);
    pub const FUNICULAR: Self = Self(1 << 7);
    pub const COACH: Self = Self(1 << 8);
    pub const AIR: Self = Self(1 << 9);
    pub const TAXI: Self = Self(1 << 10);

    pub fn join(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}
