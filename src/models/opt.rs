use crate::models::sentinel::Sentinel;
use bytemuck::{Pod, Zeroable};

/// A zero-overhead, memory-mappable wrapper for optional values.
#[repr(transparent)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Opt<T: Sentinel>(T);

impl<T: Sentinel> Opt<T> {
    pub const fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T: Sentinel> From<Option<T>> for Opt<T> {
    fn from(value: Option<T>) -> Self {
        value.map_or_else(|| Self::new(T::NONE), |value| Self::new(value))
    }
}

// By telling bytemuck this is transparent, it maps perfectly from disk
unsafe impl<T: Zeroable + Sentinel> Zeroable for Opt<T> {}
unsafe impl<T: Pod + Sentinel> Pod for Opt<T> {}

impl<T: Sentinel> Opt<T> {
    /// Forces the user to handle the Option
    pub fn as_option(self) -> Option<T> {
        self.0.as_option()
    }

    pub fn is_some(self) -> bool {
        self.0.is_some()
    }

    pub fn is_none(self) -> bool {
        self.0.is_none()
    }
}
