use rayon::slice::ParallelSliceMut;
use std::collections::HashMap;

use crate::models::{Coordinate, Distance, Shape, ShapeSlice};

pub fn build_shapes(
    raw_shapes: &[gtfs_structures::Shape],
) -> (Vec<Shape>, HashMap<String, ShapeSlice>) {
    let mut raw_shape_map: HashMap<String, Vec<&gtfs_structures::Shape>> = HashMap::new();

    raw_shapes.iter().for_each(|shape| {
        raw_shape_map
            .entry(shape.id.to_string())
            .or_default()
            .push(shape);
    });

    let mut shapes: Vec<Shape> = Vec::new();
    let mut shape_map: HashMap<String, ShapeSlice> = HashMap::new();

    for (id, mut shape_seq) in raw_shape_map.into_iter() {
        shape_seq.par_sort_unstable_by_key(|shape| shape.sequence);
        let slice = ShapeSlice {
            start: shapes.len() as u32,
            count: shape_seq.len() as u32,
        };
        shape_map.insert(id, slice);

        shapes.extend(shape_seq.iter().map(|shape| Shape {
            coordinate: Coordinate::new(shape.latitude, shape.longitude),
            distance_traveled: shape.dist_traveled.map(Distance).into(),
        }));
    }

    (shapes, shape_map)
}
