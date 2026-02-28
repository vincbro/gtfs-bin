use crate::models::sentinel::Sentinel;

pub trait Idx: Sentinel {}

macro_rules! define_index {
    ($name:ident) => {
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
        pub struct $name(pub u32);

        impl Sentinel for $name {
            const NONE: Self = Self(u32::MAX);
        }

        impl Default for $name {
            fn default() -> Self {
                Self::NONE
            }
        }

        impl From<$name> for usize {
            #[inline(always)]
            fn from(idx: $name) -> Self {
                idx.0 as usize
            }
        }
    };
}

define_index!(StopIdx);
define_index!(TripIdx);
define_index!(AreaIdx);
define_index!(RouteIdx);
define_index!(ServiceIdx);
