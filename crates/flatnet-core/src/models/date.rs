use std::fmt::{self, Display};

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

impl Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_year_month_day() {
            Some((year, month, day)) => write!(f, "{year:04}-{month:02}-{day:02}"),
            None => f.write_str("none"),
        }
    }
}

impl Default for Date {
    fn default() -> Self {
        Self(u32::MIN)
    }
}

impl Date {
    /// Builds a date from a calendar day. Returns `None` for an impossible day, a year before
    /// 1970 or a year after 9999.
    #[must_use]
    pub fn from_year_month_day(year: u32, month: u32, day: u32) -> Option<Self> {
        if !(1970..=9999).contains(&year) || !(1..=12).contains(&month) || day < 1 {
            return None;
        }
        let is_leap =
            year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
        let days_in_month = match month {
            2 if is_leap => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        if day > days_in_month {
            return None;
        }

        // Count from March so the leap day is the last day of the year.
        let shifted_year = if month <= 2 { year - 1 } else { year };
        let era = shifted_year / 400;
        let year_of_era = shifted_year % 400;
        let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        // The year is at least 1970, so this can't underflow.
        Some(Self(era * 146_097 + day_of_era - 719_468))
    }

    /// Returns the calendar day as year, month and day, or `None` for a date after year 9999,
    /// which includes the sentinel. That is the last year `from_year_month_day` builds, and the
    /// last one a calendar library accepts.
    #[must_use]
    pub fn to_year_month_day(self) -> Option<(u32, u32, u32)> {
        // Shift the epoch from 1970-01-01 to 0000-03-01.
        let epoch = self.0.checked_add(719_468)?;

        // Calculate the "era" (a 400-year block) and the day within it.
        let era = epoch / 146_097;
        let day_of_era = epoch % 146_097;

        // The year of the era [0, 399], then the year and the day of the year.
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365;
        let year = year_of_era + era * 400;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);

        // The month and day in the calendar that starts in March.
        let shifted_month = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;

        // Back to the calendar that starts in January.
        let month = if shifted_month < 10 {
            shifted_month + 3
        } else {
            shifted_month - 9
        };
        let year = if month <= 2 { year + 1 } else { year };
        (year <= 9999).then_some((year, month, day))
    }

    /// Parses a GTFS date, which is exactly 8 digits as `YYYYMMDD`.
    #[must_use]
    pub fn parse_gtfs(text: &str) -> Option<Self> {
        if text.len() != 8 || !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let year = text[..4].parse().ok()?;
        let month = text[4..6].parse().ok()?;
        let day = text[6..].parse().ok()?;
        Self::from_year_month_day(year, month, day)
    }

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
    fn parses_gtfs_dates() {
        assert_eq!(Date::parse_gtfs("19700101"), Some(Date(0)), "the epoch");
        assert_eq!(
            Date::parse_gtfs("20000229"),
            Some(Date(11_016)),
            "a leap day"
        );
        assert_eq!(
            Date::parse_gtfs("20170101"),
            Some(Date(17_167)),
            "a date in the fixture"
        );
    }

    #[test]
    fn rejects_impossible_dates() {
        for text in [
            "20230229",
            "20231301",
            "20230100",
            "19691231",
            "2023011",
            "2023-01-01",
            "",
        ] {
            assert_eq!(Date::parse_gtfs(text), None, "{text:?} is not a date");
        }
    }

    #[test]
    fn parsing_and_formatting_round_trip() {
        for text in ["19700101", "20240229", "20991231", "99991231"] {
            let date = Date::parse_gtfs(text).expect("a valid date");
            assert_eq!(
                date.to_string().replace('-', ""),
                text,
                "round trip of {text}"
            );
        }
    }

    #[test]
    fn a_date_after_year_9999_has_no_calendar_day() {
        let last = Date::from_year_month_day(9999, 12, 31).expect("the last valid day");

        assert_eq!(
            last.to_year_month_day(),
            Some((9999, 12, 31)),
            "the last day still converts"
        );
        assert_eq!(
            Date(last.0 + 1).to_year_month_day(),
            None,
            "the day after is out of range"
        );
        assert_eq!(
            Date(u32::MAX).to_year_month_day(),
            None,
            "the sentinel has none"
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
