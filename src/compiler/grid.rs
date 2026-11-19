use rayon::slice::ParallelSliceMut;

use crate::models::{GridSlice, Slice, Stop, StopCell, StopIndex};

/// Groups the stops that have a position into grid cells, sorted so the consumer can binary search.
pub(super) fn build_stop_grid(stops: &[Stop]) -> (Vec<StopCell>, Vec<StopIndex>) {
    let mut placed: Vec<((u32, u32), StopIndex)> = stops
        .iter()
        .filter_map(|stop| {
            let coordinate = stop.coordinate.as_option()?;
            let cell = StopCell::cell_of(coordinate.latitude_f64(), coordinate.longitude_f64());
            Some((cell, stop.index))
        })
        .collect();
    placed.par_sort_unstable();

    let grid_stops: Vec<StopIndex> = placed.iter().map(|&(_, stop)| stop).collect();
    let mut cells: Vec<StopCell> = Vec::new();
    for (position, &((latitude_cell, longitude_cell), _)) in placed.iter().enumerate() {
        match cells.last_mut() {
            Some(cell)
                if (cell.latitude_cell, cell.longitude_cell) == (latitude_cell, longitude_cell) =>
            {
                cell.stops =
                    GridSlice::from_usize(cell.stops.start as usize, cell.stops.count as usize + 1);
            }
            _ => cells.push(StopCell {
                latitude_cell,
                longitude_cell,
                stops: GridSlice::from_usize(position, 1),
            }),
        }
    }
    (cells, grid_stops)
}
