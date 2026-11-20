use bytemuck::{Pod, Zeroable};

use crate::models::{GridSlice, Sentinel};

/// One square of the grid that finds stops by position.
///
/// The cells are stored sorted by `(latitude_cell, longitude_cell)`, and only cells that hold a
/// stop exist.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable, PartialEq, Eq)]
pub struct StopCell {
    pub latitude_cell: u32,
    pub longitude_cell: u32,
    /// The stops in the cell, as a run of the grid's stop list.
    pub stops: GridSlice,
}

impl StopCell {
    /// The side of a cell in degrees, which is about 550 meters of latitude.
    pub const SIZE_DEGREES: f64 = 0.005;

    /// Returns the cell a position falls in. A position outside the globe is clamped to its edge.
    #[must_use]
    pub fn cell_of(latitude: f64, longitude: f64) -> (u32, u32) {
        let cell = |degrees: f64, offset: f64| {
            // The input is clamped, so the quotient is between 0 and 72,000.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let cell = ((degrees + offset) / Self::SIZE_DEGREES).floor() as u32;
            cell
        };
        (
            cell(latitude.clamp(-90.0, 90.0), 90.0),
            cell(longitude.clamp(-180.0, 180.0), 180.0),
        )
    }
}

impl Sentinel for StopCell {
    const NONE: Self = Self {
        latitude_cell: u32::MAX,
        longitude_cell: u32::MAX,
        stops: GridSlice::NONE,
    };
}
