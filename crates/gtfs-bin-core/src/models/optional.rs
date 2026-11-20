use bytemuck::{Pod, Zeroable};

use crate::models::sentinel::Sentinel;

/// A zero-overhead, memory-mappable wrapper for optional values.
#[repr(transparent)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Optional<T: Sentinel>(T);

impl<T: Sentinel> Optional<T> {
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T: Sentinel> From<Option<T>> for Optional<T> {
    fn from(value: Option<T>) -> Self {
        value.map_or_else(|| Self::new(T::NONE), |value| Self::new(value))
    }
}

#[allow(
    unsafe_code,
    reason = "bytemuck's traits can only be implemented unsafely"
)]
// SAFETY: `Optional<T>` is `repr(transparent)` over `T`, so it has the layout and zeroability of `T`.
unsafe impl<T: Zeroable + Sentinel> Zeroable for Optional<T> {}

#[allow(
    unsafe_code,
    reason = "bytemuck's traits can only be implemented unsafely"
)]
// SAFETY: `Optional<T>` is `repr(transparent)` over `T`, which is `Pod`, so it has no padding and
// every bit pattern is valid.
unsafe impl<T: Pod + Sentinel> Pod for Optional<T> {}

impl<T: Sentinel> Optional<T> {
    #[must_use]
    pub fn as_option(self) -> Option<T> {
        self.0.as_option()
    }

    #[must_use]
    pub fn is_some(self) -> bool {
        self.0.is_some()
    }

    #[must_use]
    pub fn is_none(self) -> bool {
        self.0.is_none()
    }
}
