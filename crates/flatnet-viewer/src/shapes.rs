//! Every shape in the file, with the box it fits in, so the map can skip the ones out of view.

use flatnet_core::{consumer::Consumer, models::ShapeIndex};

use crate::overlay::{self, Bounds};

/// One piece of path between two stops.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ShapeBox {
    pub(crate) index: ShapeIndex,
    pub(crate) bounds: Bounds,
}

pub(crate) fn all(consumer: &Consumer<'_>) -> Vec<ShapeBox> {
    (0..consumer.shapes().len())
        .map(ShapeIndex::from)
        .filter_map(|index| {
            let points = consumer
                .shape_coordinates(index)
                .iter()
                .map(|&coordinate| overlay::position(coordinate));
            Bounds::of(points).map(|bounds| ShapeBox { index, bounds })
        })
        .collect()
}
