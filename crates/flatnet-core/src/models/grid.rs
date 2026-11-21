use std::{f64::consts::PI, ops::RangeInclusive};

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

impl StopCell {
    /// Meters in a degree of latitude on the sphere `Coordinate::distance_meters` uses.
    const METERS_PER_DEGREE: f64 = 6_371_008.8 * PI / 180.0;

    /// Widens the span a little, against rounding.
    const SPAN_MARGIN: f64 = 1.01;

    /// Returns the cells that hold every position within `radius_meters` of a position.
    ///
    /// A degree of longitude is shorter away from the equator, and a circle reaches the poleward
    /// latitudes where it is shortest, so the width is taken there. The floor keeps it finite at
    /// a pole itself. A circle that reaches a pole holds every longitude, and one that crosses the
    /// 180th meridian continues on the other side. The position and the radius are finite.
    #[must_use]
    pub fn cells_around(latitude: f64, longitude: f64, radius_meters: f64) -> CellSpan {
        let latitude_span = radius_meters / Self::METERS_PER_DEGREE * Self::SPAN_MARGIN;
        let poleward = (latitude.abs() + latitude_span).min(90.0);
        let longitude_span = latitude_span / poleward.to_radians().cos().max(1e-9);
        let rows = Self::cell_of(latitude - latitude_span, 0.0).0
            ..=Self::cell_of(latitude + latitude_span, 0.0).0;

        let last_column = Self::cell_of(0.0, 180.0).1;
        let none = RangeInclusive::new(1, 0);
        let centre = (longitude + 180.0).rem_euclid(360.0) - 180.0;
        let (west, east) = (centre - longitude_span, centre + longitude_span);
        let columns = if latitude.abs() + latitude_span >= 90.0 || longitude_span >= 180.0 {
            [0..=last_column, none]
        } else if west < -180.0 {
            [
                Self::cell_of(0.0, west + 360.0).1..=last_column,
                0..=Self::cell_of(0.0, east).1,
            ]
        } else if east > 180.0 {
            [
                Self::cell_of(0.0, west).1..=last_column,
                0..=Self::cell_of(0.0, east - 360.0).1,
            ]
        } else {
            [
                Self::cell_of(0.0, west).1..=Self::cell_of(0.0, east).1,
                none,
            ]
        };
        CellSpan { rows, columns }
    }
}

/// The cells around a position: a run of rows, and one run of columns, or two when the area
/// crosses the 180th meridian. The second run of columns is empty when there is no crossing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellSpan {
    pub rows: RangeInclusive<u32>,
    pub columns: [RangeInclusive<u32>; 2],
}

impl CellSpan {
    /// Returns every `(row, column)` pair of the span.
    pub fn cells(self) -> impl Iterator<Item = (u32, u32)> {
        let Self { rows, columns } = self;
        rows.flat_map(move |row| {
            columns
                .clone()
                .into_iter()
                .flat_map(move |columns| columns.map(move |column| (row, column)))
        })
    }
}

impl Sentinel for StopCell {
    const NONE: Self = Self {
        latitude_cell: u32::MAX,
        longitude_cell: u32::MAX,
        stops: GridSlice::NONE,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Coordinate;

    #[test]
    fn a_wide_circle_at_high_latitude_stays_inside_its_cells() {
        // The easternmost point of a circle is not at the centre's latitude but nearer the pole, and
        // a span taken at the centre's latitude falls short of it for a wide circle.
        let (latitude, radius) = (60.0_f64, 1_000_000.0);
        let angle: f64 = radius / 6_371_008.8;
        let reach_latitude = (latitude.to_radians().sin() / angle.cos())
            .asin()
            .to_degrees();
        let reach_longitude = (angle.sin() / latitude.to_radians().cos())
            .asin()
            .to_degrees();

        let span = StopCell::cells_around(latitude, 10.0, radius);

        let farthest = Coordinate::new(reach_latitude, 10.0 + reach_longitude);
        assert!(
            Coordinate::new(latitude, 10.0).distance_meters(farthest) <= radius * 1.0001,
            "the point is on the circle"
        );
        assert!(
            span.columns[0].contains(&StopCell::cell_of(reach_latitude, 10.0 + reach_longitude).1),
            "and inside the cells searched"
        );
    }

    #[test]
    fn a_circle_over_the_180th_meridian_holds_both_sides() {
        let span = StopCell::cells_around(0.0, -179.9, 100_000.0);

        assert!(
            span.clone()
                .cells()
                .any(|cell| cell == StopCell::cell_of(0.0, 179.5)),
            "the east side of the meridian is searched from the west"
        );
        assert!(
            span.clone()
                .cells()
                .any(|cell| cell == StopCell::cell_of(0.0, -179.5)),
            "and the west side too"
        );
    }

    #[test]
    fn a_short_walk_near_a_pole_spans_many_degrees_of_longitude() {
        // At 89.975 degrees a degree of longitude is about 49 meters, so 4 meters of reach is 0.08
        // degrees either side, which a floor on the cosine of the latitude would shrink.
        let span = StopCell::cells_around(89.975, 0.0, 4.0);

        assert!(
            span.cells()
                .any(|cell| cell == StopCell::cell_of(89.975, 0.0239)),
            "a stop 0.0239 degrees east is in a cell searched"
        );
    }

    #[test]
    fn a_circle_over_a_pole_holds_every_longitude() {
        let span = StopCell::cells_around(89.9, 20.0, 50_000.0);

        for longitude in [-180.0, -90.0, 0.0, 90.0, 180.0] {
            assert!(
                span.clone()
                    .cells()
                    .any(|cell| cell == StopCell::cell_of(89.95, longitude)),
                "longitude {longitude} is within the circle near the pole"
            );
        }
    }
}
