use bytemuck::{Pod, Zeroable};

use crate::models::Sentinel;

/// The direction of travel of a trip, from GTFS `direction_id`.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Pod, Zeroable)]
pub struct Direction(pub u8);

impl Direction {
    pub const OUTBOUND: Self = Self(0);
    pub const INBOUND: Self = Self(1);
}

impl Sentinel for Direction {
    const NONE: Self = Self(u8::MAX);
}

impl Default for Direction {
    fn default() -> Self {
        Self::NONE
    }
}

impl From<u8> for Direction {
    /// Converts a GTFS `direction_id`. Anything but 0 and 1 is `NONE`.
    fn from(code: u8) -> Self {
        match code {
            0 | 1 => Self(code),
            _ => Self::NONE,
        }
    }
}
