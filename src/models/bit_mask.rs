pub trait BitMask {
    #[must_use]
    fn join(self, other: Self) -> Self;
    #[must_use]
    fn contains(self, other: Self) -> bool;
}
