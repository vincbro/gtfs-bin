use crate::models::sentinel::Sentinel;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Distance(pub f32);

impl Distance {
    /// Stores a distance in meters, which an `f32` holds with more precision than any feed has.
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a feed's distances are far inside f32's range"
    )]
    pub fn from_meters(meters: f64) -> Self {
        Self(meters as f32)
    }
}

impl Sentinel for Distance {
    const NONE: Self = Self(f32::MAX);
}

impl Default for Distance {
    fn default() -> Self {
        Self(f32::MIN)
    }
}
