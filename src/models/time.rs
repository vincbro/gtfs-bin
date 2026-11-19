use std::{
    fmt::Display,
    ops::{Add, AddAssign, Sub, SubAssign},
};

use crate::models::sentinel::Sentinel;

/// Seconds since the start of the day the trip started running.
///
/// GTFS times go past 24 hours when a trip runs past midnight.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, bytemuck::Pod, bytemuck::Zeroable,
)]
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
        let hours = self.0 / 3600;
        let minutes = (self.0 % 3600) / 60;
        let seconds = self.0 % 60;
        write!(f, "{hours:02}:{minutes:02}:{seconds:02}")
    }
}

impl Time {
    /// Parses `H:MM:SS` or `HH:MM:SS`. Returns `None` if a part is missing, isn't a number, or
    /// the total doesn't fit.
    #[must_use]
    pub fn from_hms(time: &str) -> Option<Self> {
        let mut split = time.split(':');
        let hours: u32 = split.next()?.parse().ok()?;
        let minutes: u32 = split.next()?.parse().ok()?;
        let seconds: u32 = split.next()?.parse().ok()?;
        // Overflow is a real case here because the text comes from a file.
        let total = hours
            .checked_mul(3600)?
            .checked_add(minutes.checked_mul(60)?)?
            .checked_add(seconds)?;
        Some(Self(total))
    }
}

// Time + Duration = Time
impl Add<Duration> for Time {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        // The sum clamps at the largest value, which is the sentinel, so an overflow reads as
        // absent instead of panicking.
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
        // A time before the start of the day clamps to midnight.
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
        // The difference to a later time is zero, not negative.
        Duration(self.0.saturating_sub(rhs.0))
    }
}

// Time + Delay = Time
impl Add<Delay> for Time {
    type Output = Self;

    fn add(self, rhs: Delay) -> Self::Output {
        let magnitude = u32::from(rhs.0.unsigned_abs());
        if rhs.0.is_negative() {
            // An early time before the start of the day clamps to midnight.
            Self(self.0.saturating_sub(magnitude))
        } else {
            Self(self.0.saturating_add(magnitude))
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
        let magnitude = u32::from(rhs.0.unsigned_abs());
        if rhs.0.is_negative() {
            Self(self.0.saturating_add(magnitude))
        } else {
            // Clamps to midnight, like `Time - Duration`.
            Self(self.0.saturating_sub(magnitude))
        }
    }
}

impl SubAssign<Delay> for Time {
    fn sub_assign(&mut self, rhs: Delay) {
        *self = *self - rhs;
    }
}

/// A span between two times, in seconds.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
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
        // Clamps at the largest value, like `Time`.
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
        // A duration can't be negative, so subtracting a longer one gives zero.
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl SubAssign<Self> for Duration {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

/// A signed delay in seconds, negative when early.
///
/// `NONE` is `i16::MAX`, so a real delay of that size can't be told apart from it.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable, Default)]
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
        // A delay is an `i16`, so a sum past its range is an expected input and clamps.
        Self(self.0.saturating_add(rhs.0))
    }
}

impl AddAssign<Self> for Delay {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_times_past_midnight() {
        assert_eq!(
            Time::from_hms("25:30:15"),
            Some(Time(91_815)),
            "gtfs allows hours past 24"
        );
    }

    #[test]
    fn rejects_missing_parts_and_overflow() {
        assert_eq!(Time::from_hms("10:00"), None, "seconds are missing");
        assert_eq!(Time::from_hms("a:00:00"), None, "hours are not a number");
        assert_eq!(
            Time::from_hms("4294967295:00:00"),
            None,
            "the total overflows"
        );
    }

    #[test]
    fn subtraction_clamps_at_midnight() {
        assert_eq!(
            Time(10) - Duration(20),
            Time(0),
            "a time can't go before the day starts"
        );
        assert_eq!(
            Time(10) - Time(20),
            Duration(0),
            "a duration can't be negative"
        );
    }

    #[test]
    fn delays_move_a_time_both_ways() {
        assert_eq!(
            Time(100) + Delay(-30),
            Time(70),
            "an early vehicle arrives sooner"
        );
        assert_eq!(
            Time(100) - Delay(-30),
            Time(130),
            "removing an early delay adds time back"
        );
    }

    #[test]
    fn delay_sum_clamps_to_its_range() {
        assert_eq!(
            Delay(i16::MAX - 1) + Delay(10),
            Delay(i16::MAX),
            "the sum is clamped"
        );
    }
}
