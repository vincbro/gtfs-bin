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

impl From<i64> for Time {
    fn from(value: i64) -> Self {
        u32::try_from(value).map_or(Self::NONE, Self)
    }
}
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
        write!(f, "{h:02}:{m:02}:{s:02}")
    }
}

impl Time {
    #[must_use]
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
    type Output = Self;

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
    type Output = Self;

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
impl Sub<Self> for Time {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        Duration(self.0.saturating_sub(rhs.0))
    }
}

// Time + Delay = Time
impl Add<Delay> for Time {
    type Output = Self;

    fn add(self, rhs: Delay) -> Self::Output {
        if rhs.0.is_negative() {
            Self(self.0.saturating_sub(u32::from(rhs.0.unsigned_abs())))
        } else {
            Self(self.0.saturating_add(u32::from(rhs.0.unsigned_abs())))
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
    type Output = Self;

    fn sub(self, rhs: Delay) -> Self::Output {
        // Subtracting a delay is the opposite of adding it
        if rhs.0.is_negative() {
            Self(self.0.saturating_add(u32::from(rhs.0.unsigned_abs())))
        } else {
            Self(self.0.saturating_sub(u32::from(rhs.0.unsigned_abs())))
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
impl Add<Self> for Duration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl AddAssign<Self> for Duration {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

// Duration - Duration = Duration
impl Sub<Self> for Duration {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl SubAssign<Self> for Duration {
    fn sub_assign(&mut self, rhs: Self) {
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

impl From<i32> for Delay {
    fn from(value: i32) -> Self {
        i16::try_from(value).map_or(Self::NONE, Self)
    }
}

impl From<i64> for Delay {
    fn from(value: i64) -> Self {
        i16::try_from(value).map_or(Self::NONE, Self)
    }
}

// Delay + Delay = Delay
impl Add<Self> for Delay {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl AddAssign<Self> for Delay {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
