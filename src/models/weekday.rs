use crate::models::BitMask;

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
}

impl BitMask for Weekday {
    fn join(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}
