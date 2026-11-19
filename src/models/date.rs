use chrono::NaiveDate;
use std::fmt::Display;

use crate::models::Sentinel;

/// Days since the unix epoch.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, bytemuck::Pod, bytemuck::Zeroable,
)]
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
        // Shift the epoch from 1970-01-01 to 0000-03-01. The sentinel is the one value that
        // overflows here.
        let Some(epoch) = self.0.checked_add(719_468) else {
            return f.write_str("none");
        };

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
    /// Returns the day of the week, with Monday as 0 and Sunday as 6.
    #[must_use]
    pub const fn day_of_week(self) -> u8 {
        // 1970-01-01 was a Thursday. Reducing before adding keeps the sentinel from overflowing.
        ((self.0 % 7 + 3) % 7) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_epoch_is_a_thursday() {
        assert_eq!(
            Date(0).day_of_week(),
            3,
            "1970-01-01 was a Thursday, Monday being 0"
        );
    }

    #[test]
    fn sentinel_does_not_overflow() {
        assert_eq!(
            Date::NONE.to_string(),
            "none",
            "the sentinel has no calendar date"
        );
        assert!(
            Date::NONE.day_of_week() < 7,
            "the sentinel still maps to a weekday"
        );
    }

    #[test]
    fn formats_a_leap_day() {
        assert_eq!(
            Date(11_016).to_string(),
            "2000-02-29",
            "2000 was a leap year"
        );
    }
}
