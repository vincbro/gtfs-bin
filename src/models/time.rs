use crate::models::sentinel::Sentinel;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Time(pub u32);

impl Sentinel for Time {
    const NONE: Self = Self(u32::MAX);
}

impl Default for Time {
    fn default() -> Self {
        Self(u32::MIN)
    }
}

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
