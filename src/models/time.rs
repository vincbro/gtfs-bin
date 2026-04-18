use crate::models::sentinel::Sentinel;
use std::{
    fmt::Display,
    ops::{Add, AddAssign, Sub, SubAssign},
};

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

// Time + Duration = Time
impl Add<Duration> for Time {
    type Output = Time;

    fn add(self, rhs: Duration) -> Self::Output {
        // Use saturating_add to prevent overflow panics.
        // Note: Assumes neither value is the NONE sentinel.
        Self(self.0.saturating_add(rhs.0))
    }
}

impl AddAssign<Duration> for Time {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

// Time - Duration = Time
impl Sub<Duration> for Time {
    type Output = Time;

    fn sub(self, rhs: Duration) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl SubAssign<Duration> for Time {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

// Time - Time = Duration
impl Sub<Time> for Time {
    type Output = Duration;

    fn sub(self, rhs: Time) -> Self::Output {
        Duration(self.0.saturating_sub(rhs.0))
    }
}

// Time + Delay = Time
impl Add<Delay> for Time {
    type Output = Time;

    fn add(self, rhs: Delay) -> Self::Output {
        if rhs.0.is_negative() {
            Self(self.0.saturating_sub(rhs.0.unsigned_abs() as u32))
        } else {
            Self(self.0.saturating_add(rhs.0 as u32))
        }
    }
}

impl AddAssign<Delay> for Time {
    fn add_assign(&mut self, rhs: Delay) {
        *self = *self + rhs;
    }
}

// Time - Delay = Time
impl Sub<Delay> for Time {
    type Output = Time;

    fn sub(self, rhs: Delay) -> Self::Output {
        // Subtracting a delay is the opposite of adding it
        if rhs.0.is_negative() {
            Self(self.0.saturating_add(rhs.0.unsigned_abs() as u32))
        } else {
            Self(self.0.saturating_sub(rhs.0 as u32))
        }
    }
}

impl SubAssign<Delay> for Time {
    fn sub_assign(&mut self, rhs: Delay) {
        *self = *self - rhs;
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

// Duration + Duration = Duration
impl Add<Duration> for Duration {
    type Output = Duration;

    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl AddAssign<Duration> for Duration {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

// Duration - Duration = Duration
impl Sub<Duration> for Duration {
    type Output = Duration;

    fn sub(self, rhs: Duration) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl SubAssign<Duration> for Duration {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable, Default)]
/// A delay, can be positive and negative
pub struct Delay(pub i16);

impl Sentinel for Delay {
    const NONE: Self = Self(i16::MAX);
}

impl From<i16> for Delay {
    fn from(value: i16) -> Self {
        Self(value)
    }
}

// Delay + Delay = Delay
impl Add<Delay> for Delay {
    type Output = Delay;

    fn add(self, rhs: Delay) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl AddAssign<Delay> for Delay {
    fn add_assign(&mut self, rhs: Delay) {
        *self = *self + rhs;
    }
}
