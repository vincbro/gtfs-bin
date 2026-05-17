use std::collections::HashMap;

use crate::models::sentinel::Sentinel;

pub trait Slice: Sentinel {
    /// Helper to instantly convert this into a standard Rust range
    fn range(self) -> std::ops::Range<usize>;

    fn new(start: u32, count: u32) -> Self;
    fn from_usize(start: usize, count: usize) -> Self;
}

macro_rules! define_slice {
    ($name:ident) => {
        #[repr(C)]
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            bytemuck::Pod,
            bytemuck::Zeroable,
        )]
        pub struct $name {
            pub start: u32,
            pub count: u32,
        }

        impl Default for $name {
            fn default() -> Self {
                Self::NONE
            }
        }

        impl From<(u32, u32)> for $name {
            fn from(value: (u32, u32)) -> Self {
                let start = value.0;
                let count = value.1;
                Self::new(start, count)
            }
        }

        impl From<(usize, usize)> for $name {
            fn from(value: (usize, usize)) -> Self {
                let start = u32::try_from(value.0);
                let count = u32::try_from(value.1);
                if let Ok(start) = start
                    && let Ok(count) = count
                {
                    Self::new(start, count)
                } else {
                    Self::NONE
                }
            }
        }

        impl Sentinel for $name {
            const NONE: Self = Self {
                start: u32::MIN,
                count: u32::MIN,
            };
        }

        impl Slice for $name {
            fn new(start: u32, count: u32) -> Self {
                Self { start, count }
            }

            fn from_usize(start: usize, count: usize) -> Self {
                let start = u32::try_from(start);
                let count = u32::try_from(count);

                if let Ok(start) = start
                    && let Ok(count) = count
                {
                    Self { start, count }
                } else {
                    Self::NONE
                }
            }

            /// Helper to instantly convert this into a standard Rust range
            #[inline(always)]
            fn range(self) -> std::ops::Range<usize> {
                let start = self.start as usize;
                let end = start + self.count as usize;
                start..end
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
define_slice!(TripPatternSlice);
define_slice!(RouteSlice);
define_slice!(StopTimeSlice);
define_slice!(StopSlice);
define_slice!(SearchSlice);
define_slice!(TransferSlice);
define_slice!(ServiceBinarySlice);
define_slice!(ShapeSlice);

#[derive(Default)]
pub struct SliceBuilder<T: Slice> {
    buffer: String,
    map: HashMap<String, T>,
}

impl<T: Slice> SliceBuilder<T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            map: HashMap::new(),
        }
    }

    #[must_use]
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
            let slice = T::from_usize(start, count);
            self.map.insert(value.to_string(), slice);
            slice
        }
    }

    /// Returns the buffer
    #[must_use]
    pub fn take(self) -> String {
        self.buffer
    }
}
