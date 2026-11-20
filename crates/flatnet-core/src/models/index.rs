use crate::models::sentinel::Sentinel;

/// A position in one of the file's tables, stored as a `u32`.
///
/// `NONE` is `u32::MAX`, and a `usize` that doesn't fit becomes `NONE`.
pub trait Index: Sentinel + From<usize> + Ord {
    fn as_usize(self) -> usize;
}

// A macro because each index needs its own distinct type with identical derives and impls,
// and a generic wrapper would make `StopIndex` and `TripIndex` the same type.

macro_rules! define_index {
    ($name:ident) => {
        #[repr(transparent)]
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
        pub struct $name(pub u32);

        impl Sentinel for $name {
            const NONE: Self = Self(u32::MAX);
        }

        impl From<usize> for $name {
            fn from(value: usize) -> Self {
                u32::try_from(value).map_or(Self::NONE, Self)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::NONE
            }
        }

        impl $name {
            #[must_use]
            pub const fn as_usize(self) -> usize {
                self.0 as usize
            }
        }

        impl Index for $name {
            fn as_usize(self) -> usize {
                Self::as_usize(self)
            }
        }
    };
}

define_index!(StopIndex);
define_index!(SearchIndex);
define_index!(TripIndex);
define_index!(TripPatternIndex);
define_index!(RouteIndex);
define_index!(StopTimeIndex);
define_index!(ShapeIndex);
define_index!(ServiceIndex);
define_index!(TransferIndex);
