use bytemuck::{Pod, Zeroable};

/// The optional parts of a file: which of them the compiler wrote. A part it left out is empty,
/// and what reads it finds nothing, so a routing-only file is smaller.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Pod, Zeroable)]
pub struct Parts(pub u64);

impl Parts {
    /// The `shapes`, `shape_points` and `trip_shapes` sections. Without them no trip has a shape.
    pub const SHAPES: Self = Self(1);

    /// The `search_stops` and `search_to_stops` sections. Needs `TEXT`, because a place is found by
    /// its name.
    pub const SEARCH: Self = Self(1 << 1);

    /// The code, name and description of every stop and the headsign, short name and block of every
    /// trip. Without it a headsign is not part of what makes a pattern, so patterns may be fewer.
    pub const TEXT: Self = Self(1 << 2);

    /// The GTFS ids and the lookups by id. Without them every id reads as empty and `stop_by_id` and
    /// its kin find nothing, so realtime updates match no trip.
    pub const IDS: Self = Self(1 << 3);

    /// Every part.
    pub const ALL: Self = Self(0b1111);

    /// No part: a file for routing alone.
    pub const NONE: Self = Self(0);

    /// Whether every part in `other` is in this set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// This set and the parts of `other`.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// This set without the parts of `other`.
    #[must_use]
    pub const fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }
}

impl Default for Parts {
    fn default() -> Self {
        Self::ALL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_removes_only_the_given_parts() {
        let parts = Parts::ALL.without(Parts::SHAPES);

        assert!(!parts.contains(Parts::SHAPES), "shapes are gone");
        assert!(
            parts.contains(Parts::SEARCH.with(Parts::TEXT)),
            "the rest stays"
        );
    }
}
