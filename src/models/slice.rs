use crate::models::sentinel::Sentinel;

pub trait Slice: Sentinel {
    /// Helper to instantly convert this into a standard Rust range
    fn range(self) -> std::ops::Range<usize>;
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
        }
    };
}

define_slice!(StringSlice);
define_slice!(AreaIdSlice);
define_slice!(StopIdSlice);
define_slice!(RouteIdSlice);
define_slice!(TripIdSlice);
