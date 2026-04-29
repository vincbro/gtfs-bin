use chrono::NaiveDate;
use std::fmt::Display;

use crate::models::Sentinel;

#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, bytemuck::Pod, bytemuck::Zeroable,
)]
/// Days since unix epoch
pub struct Date(pub u32);

impl Sentinel for Date {
    const NONE: Self = Self(u32::MAX);
}

impl From<u32> for Date {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<NaiveDate> for Date {
    fn from(value: NaiveDate) -> Self {
        Self(value.to_epoch_days() as u32)
    }
}

impl Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Shift the epoch from 1970-01-01 to 0000-03-01
        // Because input is u32, z will never be negative.
        let z = self.0 + 719_468;

        // Calculate the "era" (a 400-year block)
        // We can safely use standard division and modulo now.
        let era = z / 146097;
        let doe = z % 146097; // Day of the era: [0, 146096]

        // Calculate the year of the era [0, 399]
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;

        // Extract the year and the day of the year
        let mut y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]

        // Calculate the month and day using the shifted calendar
        let mp = (5 * doy + 2) / 153; // [0, 11]
        let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]

        // Adjust the month and year back to the standard civil calendar
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        if m <= 2 {
            y += 1;
        }
        write!(f, "{y:04}-{m:02}-{d:02}")
    }
}

impl Default for Date {
    fn default() -> Self {
        Self(u32::MIN)
    }
}

impl Date {
    /// Returns the day of the week 0, 6
    pub fn get_day_of_week(&self) -> u8 {
        ((self.0 + 3) % 7) as u8
    }
}

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Weekday(pub u8);

impl From<u8> for Weekday {
    fn from(value: u8) -> Self {
        Self(1 << value)
    }
}

impl Weekday {
    pub const MONDAY: Self = Self(1 << 0);
    pub const TUESDAY: Self = Self(1 << 1);
    pub const WEDNESDAY: Self = Self(1 << 2);
    pub const THURSDAY: Self = Self(1 << 3);
    pub const FRIDAY: Self = Self(1 << 4);
    pub const SATURDAY: Self = Self(1 << 5);
    pub const SUNDAY: Self = Self(1 << 6);

    pub fn join(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}
