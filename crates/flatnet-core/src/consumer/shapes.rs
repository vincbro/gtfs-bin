use crate::{
    consumer::Consumer,
    models::{Coordinate, Shape, ShapeIndex, Slice, TripIndex},
};

impl<'a> Consumer<'a> {
    /// # Panics
    ///
    /// Panics if `index` is out of range, which includes `ShapeIndex::NONE`.
    #[must_use]
    pub fn shape(&self, index: ShapeIndex) -> &'a Shape {
        &self.shapes[index.as_usize()]
    }

    /// Returns the coordinates of a shape, from the stop it starts at to the stop it ends at.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range.
    #[must_use]
    pub fn shape_coordinates(&self, index: ShapeIndex) -> &'a [Coordinate] {
        &self.shape_points[self.shape(index).points.range()]
    }

    /// Returns the shape of each hop between two stops of a trip, in order, so the first one runs
    /// from the trip's first stop to its second. Empty if the trip has no shape.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range.
    #[must_use]
    pub fn shapes_by_trip(&self, index: TripIndex) -> &'a [ShapeIndex] {
        self.trip(index)
            .shape
            .as_option()
            .map_or(&[], |slice| &self.trip_shapes[slice.range()])
    }
}
