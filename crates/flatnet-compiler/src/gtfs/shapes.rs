use std::collections::{BTreeMap, HashMap};

use flatnet_core::models::Coordinate;

use super::table::Table;
use crate::Result;

/// The shapes of `shapes.txt`, each as its points in order.
#[derive(Debug, Default)]
pub(super) struct ShapeLines {
    /// The position of each shape id in `lines`.
    pub(super) by_id: HashMap<String, usize>,
    pub(super) lines: Vec<Vec<Coordinate>>,
}

pub(super) fn read_shape_lines(table: &mut Table<'_>) -> Result<ShapeLines> {
    let id = table.require("shape_id")?;
    let latitude = table.require("shape_pt_lat")?;
    let longitude = table.require("shape_pt_lon")?;
    let sequence = table.require("shape_pt_sequence")?;

    // Ordered so the same input always gives the same bytes.
    let mut points_by_shape: BTreeMap<String, Vec<(u32, Coordinate)>> = BTreeMap::new();
    while let Some(row) = table.next_row()? {
        let shape_id = row.required(id, "shape_id")?;
        let point = (
            row.parse_required(sequence, "shape_pt_sequence")?,
            Coordinate::new(
                row.float_required(latitude, "shape_pt_lat")?,
                row.float_required(longitude, "shape_pt_lon")?,
            ),
        );
        // Looked up by `&str` first so a point of a known shape allocates nothing.
        match points_by_shape.get_mut(shape_id) {
            Some(points) => points.push(point),
            None => {
                points_by_shape.insert(shape_id.to_owned(), vec![point]);
            }
        }
    }

    let mut shape_lines = ShapeLines::default();
    for (shape_id, mut points) in points_by_shape {
        points.sort_unstable_by_key(|&(sequence, _)| sequence);
        shape_lines.by_id.insert(shape_id, shape_lines.lines.len());
        shape_lines
            .lines
            .push(points.into_iter().map(|(_, point)| point).collect());
    }
    Ok(shape_lines)
}
