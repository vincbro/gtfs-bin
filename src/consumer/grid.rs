use crate::{
    consumer::Consumer,
    models::{Coordinate, Slice, StopCell, StopIndex},
};

/// Meters in a degree of latitude, to size the part of the grid to look at.
const METERS_PER_DEGREE: f64 = 111_320.0;

impl Consumer<'_> {
    /// Replaces the contents of `found` with the stops within `radius_meters` of a position,
    /// nearest first, with their distance in meters.
    ///
    /// Stops with no position are never found. A radius that isn't a finite number finds nothing.
    pub fn stops_near(
        &self,
        latitude: f64,
        longitude: f64,
        radius_meters: f64,
        found: &mut Vec<(StopIndex, f32)>,
    ) {
        found.clear();
        if !(latitude.is_finite() && longitude.is_finite() && radius_meters.is_finite()) {
            return;
        }
        let radius_meters = radius_meters.max(0.0);
        let here = Coordinate::new(latitude, longitude);

        // A degree of longitude is shorter away from the equator. The floor keeps the width finite
        // next to the poles.
        let latitude_span = radius_meters / METERS_PER_DEGREE;
        let longitude_span = latitude_span / latitude.to_radians().cos().abs().max(0.01);
        let (first_row, first_column) =
            StopCell::cell_of(latitude - latitude_span, longitude - longitude_span);
        let (last_row, last_column) =
            StopCell::cell_of(latitude + latitude_span, longitude + longitude_span);

        for row in first_row..=last_row {
            let start = self.stop_grid.partition_point(|cell| {
                (cell.latitude_cell, cell.longitude_cell) < (row, first_column)
            });
            let cells = self.stop_grid[start..]
                .iter()
                .take_while(|cell| cell.latitude_cell == row && cell.longitude_cell <= last_column);
            for cell in cells {
                self.collect_near(here, radius_meters, cell, found);
            }
        }
        found.sort_unstable_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    }

    fn collect_near(
        &self,
        here: Coordinate,
        radius_meters: f64,
        cell: &StopCell,
        found: &mut Vec<(StopIndex, f32)>,
    ) {
        for &stop_index in &self.grid_stops[cell.stops.range()] {
            let Some(position) = self.stop(stop_index).coordinate.as_option() else {
                continue;
            };
            let distance = here.distance_meters(position);
            if distance <= radius_meters {
                found.push((stop_index, distance as f32));
            }
        }
    }
}
