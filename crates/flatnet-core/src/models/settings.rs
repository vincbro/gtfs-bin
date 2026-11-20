use bytemuck::{Pod, Zeroable};

/// The choices the compiler was given that shape what the file holds. The file keeps them, so a
/// reader knows what the tables mean, such as how far a footpath reaches, and doesn't have to
/// guess.
///
/// There is exactly one `Settings` in a file, inside its `Metadata`. `Settings::default` is what a
/// compile uses when nobody chose.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct Settings {
    /// The longest straight line between two places a footpath may hold, in meters. Nobody walks in
    /// a straight line, so a reader that wants walks of at most some length asks for a shorter
    /// reach, not for a factor.
    pub footpath_max_meters: u32,

    /// The most footpaths one place keeps, the nearest first. `u32::MAX` keeps every one in reach.
    /// A place that lost a neighbor to this limit may still appear in that neighbor's list, so the
    /// footpaths are not always symmetric.
    pub footpath_max_per_place: u32,
}

impl Settings {
    /// The longest `footpath_max_meters` a compile accepts, which bounds how many grid cells it
    /// searches around a stop.
    pub const MAX_FOOTPATH_METERS: u32 = 50_000;

    /// Whether every setting is one a compile can use: a reach of at most `MAX_FOOTPATH_METERS`.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.footpath_max_meters <= Self::MAX_FOOTPATH_METERS
    }
}

impl Default for Settings {
    /// A reach of 400 meters in a straight line, and the nearest 32 places.
    fn default() -> Self {
        Self {
            footpath_max_meters: 400,
            footpath_max_per_place: 32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_settings_are_valid() {
        assert!(
            Settings::default().is_valid(),
            "a compile must accept the default"
        );
    }

    #[test]
    fn a_reach_past_the_limit_is_invalid() {
        let settings = Settings {
            footpath_max_meters: Settings::MAX_FOOTPATH_METERS + 1,
            ..Settings::default()
        };

        assert!(!settings.is_valid(), "too far to search");
    }
}
