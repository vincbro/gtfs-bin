use crate::{
    consumer::Consumer,
    models::{Shape, ShapeSlice, Slice},
};

impl<'a> Consumer<'a> {
    #[inline]
    pub fn shapes(&self, slice: ShapeSlice) -> &'a [Shape] {
        &self.shapes[slice.range()]
    }
}
