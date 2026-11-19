use rayon::slice::ParallelSliceMut;
use std::collections::{BTreeMap, HashMap};

use crate::models::{Coordinate, Distance, Shape, ShapeSlice, Slice};

pub fn build_shapes(
    raw_shapes: &[gtfs_structures::Shape],
) -> (Vec<Shape>, HashMap<String, ShapeSlice>) {
    // Ordered so the same input always gives the same bytes.
    let mut raw_shape_map: BTreeMap<&str, Vec<&gtfs_structures::Shape>> = BTreeMap::new();

    for shape in raw_shapes {
        raw_shape_map.entry(&shape.id).or_default().push(shape);
    }

    let mut shapes: Vec<Shape> = Vec::with_capacity(raw_shapes.len());
    let mut shape_map: HashMap<String, ShapeSlice> = HashMap::with_capacity(raw_shape_map.len());

    for (id, mut shape_sequence) in raw_shape_map {
        shape_sequence.par_sort_unstable_by_key(|shape| shape.sequence);
        let slice = ShapeSlice::from_usize(shapes.len(), shape_sequence.len());
        shape_map.insert(id.to_string(), slice);

        shapes.extend(shape_sequence.iter().map(|shape| Shape {
            coordinate: Coordinate::new(shape.latitude, shape.longitude),
            distance_traveled: shape.dist_traveled.map(Distance).into(),
        }));
    }

    (shapes, shape_map)
}
