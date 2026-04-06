use crate::models::sentinel::Sentinel;
use std::fmt::Display;

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

impl Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let h = self.0 / 3600;
        let m = (self.0 % 3600) / 60;
        let s = self.0 % 60;
        write!(f, "{:02}:{:02}:{:02}", h, m, s)
    }
}

impl Time {
    pub fn from_hms(time: &str) -> Option<Self> {
        const HOUR_TO_SEC: u32 = 60 * 60;
        const MINUTE_TO_SEC: u32 = 60;
        let mut split = time.split(':');
        let hours: u32 = split.next()?.parse().ok()?;
        let hours = hours * HOUR_TO_SEC;
        let minutes: u32 = split.next()?.parse().ok()?;
        let minutes = minutes * MINUTE_TO_SEC;
        let seconds: u32 = split.next()?.parse().ok()?;
        let seconds = hours + minutes + seconds;
        Some(Self(seconds))
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
