use crate::models::sentinel::Sentinel;

#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, bytemuck::Pod, bytemuck::Zeroable,
)]
/// Seconds since midnight, GTFS times can go past 24h if a trip runs past midnight
/// Better describe as seconds since the start of the day the trip started running
pub struct Time(pub u32);

impl Sentinel for Time {
    const NONE: Self = Self(u32::MAX);
}

impl Default for Time {
    fn default() -> Self {
        Self(u32::MIN)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
/// A duration between two time point in seconds.
pub struct Duration(pub u32);

impl Sentinel for Duration {
    const NONE: Self = Self(u32::MAX);
}

impl From<u32> for Duration {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl Default for Duration {
    fn default() -> Self {
        Self(u32::MIN)
    }
}
