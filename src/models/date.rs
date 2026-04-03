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
        ((self.0 + 4) % 7) as u8
    }
}

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WeekdaySet(pub u8);

impl WeekdaySet {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_day(&mut self, idx: u8, value: bool) {
        assert!(idx < 7, "The day index is out of bounds, max value 6");
        self.0 = set_bit(self.0, idx, value)
    }

    pub fn get_day(&self, idx: u8) -> bool {
        assert!(idx < 7, "The day index is out of bounds, max value 6");
        is_bit_flipped(self.0, idx)
    }

    pub fn monday(&self) -> bool {
        is_bit_flipped(self.0, 0)
    }

    pub fn tuesday(&self) -> bool {
        is_bit_flipped(self.0, 1)
    }

    pub fn wednesday(&self) -> bool {
        is_bit_flipped(self.0, 2)
    }

    pub fn thursday(&self) -> bool {
        is_bit_flipped(self.0, 3)
    }

    pub fn friday(&self) -> bool {
        is_bit_flipped(self.0, 4)
    }

    pub fn saturday(&self) -> bool {
        is_bit_flipped(self.0, 5)
    }

    pub fn sunday(&self) -> bool {
        is_bit_flipped(self.0, 6)
    }

    pub fn with_monday(self, value: bool) -> Self {
        Self(set_bit(self.0, 0, value))
    }

    pub fn with_tuesday(self, value: bool) -> Self {
        Self(set_bit(self.0, 1, value))
    }

    pub fn with_wednesday(self, value: bool) -> Self {
        Self(set_bit(self.0, 2, value))
    }

    pub fn with_thursday(self, value: bool) -> Self {
        Self(set_bit(self.0, 3, value))
    }

    pub fn with_friday(self, value: bool) -> Self {
        Self(set_bit(self.0, 4, value))
    }

    pub fn with_saturday(self, value: bool) -> Self {
        Self(set_bit(self.0, 5, value))
    }

    pub fn with_sunday(self, value: bool) -> Self {
        Self(set_bit(self.0, 6, value))
    }
}
fn is_bit_flipped(byte: u8, n: u8) -> bool {
    assert!(n < 8, "Bit index out of bounds for u8");
    (byte & (1 << n)) != 0
}

fn set_bit(byte: u8, n: u8, value: bool) -> u8 {
    assert!(n < 8, "Bit index out of bounds for u8");
    let mask = 1 << n;
    if value { byte | mask } else { byte & !mask }
}
