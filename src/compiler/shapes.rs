use std::collections::{BTreeMap, HashMap};

use crate::{
    compiler::{Result, table::Table},
    models::{Coordinate, Distance, Shape, ShapeSlice, Slice},
};

pub(super) fn build_shapes(table: &mut Table) -> Result<(Vec<Shape>, HashMap<String, ShapeSlice>)> {
    let id = table.require("shape_id")?;
    let latitude = table.require("shape_pt_lat")?;
    let longitude = table.require("shape_pt_lon")?;
    let sequence = table.require("shape_pt_sequence")?;
    let distance = table.column("shape_dist_traveled");

    // Ordered so the same input always gives the same bytes.
    let mut points_by_shape: BTreeMap<String, Vec<(u32, Shape)>> = BTreeMap::new();
    let mut point_count = 0;
    while let Some(row) = table.next_row()? {
        let shape_id = row.required(id, "shape_id")?;
        let point = (
            row.parse_required(sequence, "shape_pt_sequence")?,
            Shape {
                coordinate: Coordinate::new(
                    row.float_required(latitude, "shape_pt_lat")?,
                    row.float_required(longitude, "shape_pt_lon")?,
                ),
                distance_traveled: row
                    .float(distance, "shape_dist_traveled")?
                    .map(|meters| Distance(meters as f32))
                    .into(),
            },
        );
        point_count += 1;
        // Looked up by `&str` first so a point of a known shape allocates nothing.
        match points_by_shape.get_mut(shape_id) {
            Some(points) => points.push(point),
            None => {
                points_by_shape.insert(shape_id.to_string(), vec![point]);
            }
        }
    }

    let mut shapes: Vec<Shape> = Vec::with_capacity(point_count);
    let mut shape_map: HashMap<String, ShapeSlice> = HashMap::with_capacity(points_by_shape.len());
    for (shape_id, mut points) in points_by_shape {
        points.sort_unstable_by_key(|&(sequence, _)| sequence);
        shape_map.insert(shape_id, ShapeSlice::from_usize(shapes.len(), points.len()));
        shapes.extend(points.into_iter().map(|(_, shape)| shape));
    }

    Ok((shapes, shape_map))
}
