use crate::models::BitMask;

/// The days of the week a service runs on, one bit each starting with Monday.
#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Weekday(pub u8);

impl From<u8> for Weekday {
    /// Converts a day of the week, with Monday as 0 and Sunday as 6, as `Date::day_of_week` returns.
    ///
    /// # Panics
    ///
    /// Panics if `value` is 7 or more.
    fn from(value: u8) -> Self {
        assert!(
            value < 7,
            "a day of the week is 0 for Monday to 6 for Sunday"
        );
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
}

impl BitMask for Weekday {
    fn join(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_numbers_map_to_their_bits() {
        assert_eq!(Weekday::from(0), Weekday::MONDAY, "0 is Monday");
        assert_eq!(Weekday::from(6), Weekday::SUNDAY, "6 is Sunday");
    }

    #[test]
    #[should_panic(expected = "a day of the week")]
    fn a_seventh_day_panics() {
        let _ = Weekday::from(7);
    }

    #[test]
    fn joined_days_contain_each_part() {
        let days = Weekday::MONDAY.join(Weekday::FRIDAY);

        assert!(days.contains(Weekday::FRIDAY), "friday was joined in");
        assert!(!days.contains(Weekday::SUNDAY), "sunday was not");
    }
}
