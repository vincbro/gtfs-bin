use crate::models::sentinel::Sentinel;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Distance(pub f32);

impl Sentinel for Distance {
    const NONE: Self = Self(f32::MAX);
}

impl Default for Distance {
    fn default() -> Self {
        Self(f32::MIN)
    }
}
