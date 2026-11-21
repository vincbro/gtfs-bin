use std::ops::RangeInclusive;

use crate::{
    consumer::Consumer,
    models::{Coordinate, Slice, StopCell, StopIndex},
};

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

        let span = StopCell::cells_around(latitude, longitude, radius_meters);
        let runs = span
            .rows
            .clone()
            .flat_map(|row| span.columns.iter().map(move |columns| (row, columns)));
        for (row, columns) in runs {
            for cell in self.cells_in_row(row, columns) {
                self.collect_near(here, radius_meters, cell, found);
            }
        }
        found.sort_unstable_by(|first, second| {
            first.1.total_cmp(&second.1).then(first.0.cmp(&second.0))
        });
    }

    /// Replaces the contents of `found` with the stops inside a box, in stop order. The edges are
    /// inside the box.
    ///
    /// Stops with no position are never found. A box with a bound that isn't a finite number, or
    /// with its south edge north of its north edge or its west edge east of its east edge, finds
    /// nothing, so a box over the 180th meridian is two searches.
    pub fn stops_in_box(
        &self,
        south: f64,
        west: f64,
        north: f64,
        east: f64,
        found: &mut Vec<StopIndex>,
    ) {
        found.clear();
        let is_box = [south, west, north, east]
            .iter()
            .all(|bound| bound.is_finite())
            && south <= north
            && west <= east;
        if !is_box {
            return;
        }

        let (first_row, first_column) = StopCell::cell_of(south, west);
        let (last_row, last_column) = StopCell::cell_of(north, east);
        for row in first_row..=last_row {
            for cell in self.cells_in_row(row, &(first_column..=last_column)) {
                self.collect_in_box([south, west, north, east], cell, found);
            }
        }
        found.sort_unstable();
    }

    /// Returns the cells of one row that lie in a run of columns, found by binary search because
    /// the cells are sorted.
    fn cells_in_row(
        &self,
        row: u32,
        columns: &RangeInclusive<u32>,
    ) -> impl Iterator<Item = &StopCell> {
        let start = self.stop_grid.partition_point(|cell| {
            (cell.latitude_cell, cell.longitude_cell) < (row, *columns.start())
        });
        let last = *columns.end();
        self.stop_grid[start..]
            .iter()
            .take_while(move |cell| cell.latitude_cell == row && cell.longitude_cell <= last)
    }

    fn collect_in_box(&self, bounds: [f64; 4], cell: &StopCell, found: &mut Vec<StopIndex>) {
        let [south, west, north, east] = bounds;
        for &stop_index in &self.grid_stops[cell.stops.range()] {
            let Some(position) = self.stop(stop_index).coordinate.as_option() else {
                continue;
            };
            let (latitude, longitude) = position.to_f64_pair();
            if (south..=north).contains(&latitude) && (west..=east).contains(&longitude) {
                found.push(stop_index);
            }
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "meters in an f32 is exact enough to sort and show"
    )]
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
