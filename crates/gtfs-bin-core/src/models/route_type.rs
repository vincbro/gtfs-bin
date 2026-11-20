use bytemuck::{Pod, Zeroable};

use crate::models::{BitMask, Sentinel};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Pod, Zeroable)]
pub struct RouteType(pub u16);

impl From<u16> for RouteType {
    /// Converts a GTFS `route_type` code, including the extended codes from Google's list:
    /// <https://developers.google.com/transit/gtfs/reference/extended-route-types>.
    ///
    /// Codes that match no kind become `NONE`.
    fn from(code: u16) -> Self {
        match code {
            0 | 900..=999 => Self::TRAM,
            1 | 400..=699 => Self::SUBWAY,
            // 2 rail, 12 monorail, 100-199 railway, 300-399 suburban railway
            2 | 12 | 100..=199 | 300..=399 => Self::RAIL,
            // 3 bus, 11 trolleybus, 700-899 bus and trolleybus
            3 | 11 | 700..=899 => Self::BUS,
            // 4 ferry, 1000-1099 water transport, 1200-1299 ferry
            4 | 1000..=1099 | 1200..=1299 => Self::FERRY,
            5 => Self::CABLE_CAR,
            // 6 gondola, 1300-1399 aerial lift
            6 | 1300..=1399 => Self::GONDOLA,
            // 7 funicular, 1400-1499 funicular
            7 | 1400..=1499 => Self::FUNICULAR,
            200..=299 => Self::COACH,
            1100..=1199 => Self::AIR,
            1500..=1599 => Self::TAXI,
            _ => Self::NONE,
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
}

impl BitMask for RouteType {
    fn join(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}
