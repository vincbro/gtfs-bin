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
        Self(value.to_epoch_days().cast_unsigned())
    }
}

impl Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Shift the epoch from 1970-01-01 to 0000-03-01
        // Because input is u32, z will never be negative.
        let epoch = self.0 + 719_468;

        // Calculate the "era" (a 400-year block)
        // We can safely use standard division and modulo now.
        let era = epoch / 146_097;
        let doe = epoch % 146_097; // Day of the era: [0, 146096]

        // Calculate the year of the era [0, 399]
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;

        // Extract the year and the day of the year
        let mut year = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]

        // Calculate the month and day using the shifted calendar
        let month = (5 * doy + 2) / 153; // [0, 11]
        let day = doy - (153 * month + 2) / 5 + 1; // [1, 31]

        // Adjust the month and year back to the standard civil calendar
        let month = if month < 10 { month + 3 } else { month - 9 };
        if month <= 2 {
            year += 1;
        }
        write!(f, "{year:04}-{month:02}-{day:02}")
    }
}

impl Default for Date {
    fn default() -> Self {
        Self(u32::MIN)
    }
}

impl Date {
    /// Returns the day of the week 0, 6
    #[must_use]
    pub const fn get_day_of_week(&self) -> u8 {
        ((self.0 + 3) % 7) as u8
    }
}
