use std::{collections::HashMap, ops::Range};

use crate::models::sentinel::Sentinel;

/// A `start` and `count` pair that points into one of the file's tables.
///
/// `NONE` is `(0, 0)`, which is also a valid empty slice, so a `NONE` slice and an empty one read
/// the same.
pub trait Slice: Sentinel {
    /// Converts the slice into a range that indexes its table.
    fn range(self) -> Range<usize>;

    fn new(start: u32, count: u32) -> Self;
    fn from_usize(start: usize, count: usize) -> Self;
}

// A macro because each slice needs its own distinct type, so a `TripSlice` can't be used where a
// `StopSlice` is expected, and a generic wrapper would make them one type.
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
                Self::from_usize(value.0, value.1)
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
                match (u32::try_from(start), u32::try_from(count)) {
                    (Ok(start), Ok(count)) => Self { start, count },
                    _ => Self::NONE,
                }
            }

            fn range(self) -> Range<usize> {
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
define_slice!(FootpathSlice);
define_slice!(PlaceStopSlice);
define_slice!(ServiceBinarySlice);
define_slice!(ShapePointSlice);
define_slice!(TripShapeSlice);
define_slice!(GridSlice);

/// Collects strings into one buffer and hands back a `Slice` for each, sharing repeats.
#[derive(Debug)]
pub struct SliceBuilder<T: Slice> {
    buffer: String,
    map: HashMap<String, T>,
}

impl<T: Slice> Default for SliceBuilder<T> {
    fn default() -> Self {
        Self {
            buffer: String::new(),
            map: HashMap::new(),
        }
    }
}

impl<T: Slice> SliceBuilder<T> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
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
            return slice;
        }
        let slice = T::from_usize(self.buffer.len(), value.len());
        self.buffer.push_str(value);
        self.map.insert(value.to_owned(), slice);
        slice
    }

    #[must_use]
    pub fn into_buffer(self) -> String {
        self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_strings_share_one_slice() {
        let mut builder = SliceBuilder::<StringSlice>::new();

        let first = builder.add("central");
        let other = builder.add("north");
        let again = builder.add("central");

        assert_eq!(first, again, "the same text points at the same bytes");
        assert_eq!(
            other,
            StringSlice::from_usize(7, 5),
            "new text is appended after it"
        );
        assert_eq!(
            builder.into_buffer(),
            "centralnorth",
            "each text is stored once"
        );
    }

    #[test]
    fn too_large_a_slice_becomes_none() {
        let slice = StringSlice::from_usize(usize::MAX, 1);

        assert_eq!(slice, StringSlice::NONE, "it doesn't fit in u32");
    }

    #[test]
    fn range_covers_start_to_end() {
        assert_eq!(
            StringSlice::from_usize(3, 4).range(),
            3..7,
            "count items from start"
        );
    }
}
