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
