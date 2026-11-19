use bytemuck::{Pod, Zeroable};

/// Whether passengers can get on or off at a stop time, from GTFS `pickup_type` and
/// `drop_off_type`.
#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Pod, Zeroable)]
pub struct BoardingRule(pub u8);

impl BoardingRule {
    /// Boarding or alighting is allowed as scheduled. This is the default.
    pub const REGULAR: Self = Self(0);
    /// Boarding or alighting is not possible.
    pub const NOT_AVAILABLE: Self = Self(1);
    /// The passenger must phone the agency to arrange it.
    pub const PHONE_AGENCY: Self = Self(2);
    /// The passenger must arrange it with the driver.
    pub const ARRANGE_WITH_DRIVER: Self = Self(3);

    /// Returns whether the passenger can do it without arranging anything first.
    #[must_use]
    pub const fn is_regular(self) -> bool {
        self.0 == Self::REGULAR.0
    }
}

impl From<u8> for BoardingRule {
    /// Converts a GTFS code. An unknown code is treated as the default, as the spec says.
    fn from(code: u8) -> Self {
        match code {
            1..=3 => Self(code),
            _ => Self::REGULAR,
        }
    }
}
