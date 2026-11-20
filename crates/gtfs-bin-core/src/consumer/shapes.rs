use crate::{
    consumer::Consumer,
    models::{Shape, ShapeSlice, Slice},
};

impl<'a> Consumer<'a> {
    /// # Panics
    ///
    /// Panics if the slice is out of range.
    #[must_use]
    pub fn shapes_in(&self, slice: ShapeSlice) -> &'a [Shape] {
        &self.shapes[slice.range()]
    }
}
