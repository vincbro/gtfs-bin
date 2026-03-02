use crate::models::sentinel::Sentinel;
use bytemuck::{Pod, Zeroable};

/// A zero-overhead, memory-mappable wrapper for optional values.
#[repr(transparent)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Opt<T: Sentinel>(T);

impl<T: Sentinel> Opt<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

// By telling bytemuck this is transparent, it maps perfectly from disk
unsafe impl<T: Zeroable + Sentinel> Zeroable for Opt<T> {}
unsafe impl<T: Pod + Sentinel> Pod for Opt<T> {}

impl<T: Sentinel> Opt<T> {
    /// Forces the user to handle the Option
    #[inline(always)]
    pub fn get(self) -> Option<T> {
        self.0.as_option()
    }

    #[inline(always)]
    pub fn is_some(self) -> bool {
        self.0.is_some()
    }

    #[inline(always)]
    pub fn is_none(self) -> bool {
        self.0.is_none()
    }
}
