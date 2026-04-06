use std::collections::HashMap;

use crate::models::sentinel::Sentinel;

pub trait Slice: Sentinel {
    /// Helper to instantly convert this into a standard Rust range
    fn range(self) -> std::ops::Range<usize>;

    fn new(start: u32, count: u32) -> Self;
}

macro_rules! define_slice {
    ($name:ident) => {
        #[repr(C)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
        pub struct $name {
            pub start: u32,
            pub count: u32,
        }

        impl Default for $name {
            fn default() -> Self {
                Self::NONE
            }
        }

        impl Sentinel for $name {
            const NONE: Self = Self {
                start: u32::MAX,
                count: u32::MIN,
            };
        }

        impl Slice for $name {
            /// Helper to instantly convert this into a standard Rust range
            #[inline(always)]
            fn range(self) -> std::ops::Range<usize> {
                let start = self.start as usize;
                let end = start + self.count as usize;
                start..end
            }

            fn new(start: u32, count: u32) -> Self {
                Self { start, count }
            }
        }
    };
}

define_slice!(StringSlice);
define_slice!(StopIdSlice);
define_slice!(RouteIdSlice);
define_slice!(TripIdSlice);
define_slice!(ServiceIdSlice);

define_slice!(TripSlice);
define_slice!(RouteSlice);
define_slice!(StopTimeSlice);
define_slice!(StopSlice);
define_slice!(TransferSlice);
define_slice!(ServiceBinarySlice);

#[derive(Default)]
pub struct SliceBuilder<T: Slice> {
    buffer: String,
    map: HashMap<String, T>,
}

impl<T: Slice> SliceBuilder<T> {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            map: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: String::with_capacity(capacity),
            map: HashMap::new(),
        }
    }

    /// Returns where in the buffer to find the string. It only adds new strings to the buffer.
    pub fn add(&mut self, value: &str) -> T {
        if let Some(slice) = self.map.get(value).copied() {
            slice
        } else {
            let start = self.buffer.len();
            let count = value.len();
            self.buffer.push_str(value);
            let slice = T::new(start as u32, count as u32);
            self.map.insert(value.to_string(), slice);
            slice
        }
    }

    /// Returns the buffer
    pub fn take(self) -> String {
        self.buffer
    }
}
