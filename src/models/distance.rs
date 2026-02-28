use crate::models::sentinel::Sentinel;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Distance(pub u32);

impl Sentinel for Distance {
    const NONE: Self = Self(u32::MAX);
}

impl Default for Distance {
    fn default() -> Self {
        Self(u32::MIN)
    }
}
