/// A type with one value reserved to mean "absent", so `Optional<T>` needs no extra tag byte.
///
/// The reserved value must never be a real value of the type.
pub trait Sentinel: Sized + Copy + PartialEq {
    const NONE: Self;

    fn is_none(self) -> bool {
        self == Self::NONE
    }

    fn is_some(self) -> bool {
        self != Self::NONE
    }

    fn as_option(self) -> Option<Self> {
        if self.is_some() { Some(self) } else { None }
    }
}
