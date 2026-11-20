use crate::{
    consumer::Consumer,
    models::{Slice, StringSlice},
};

impl<'a> Consumer<'a> {
    /// # Panics
    ///
    /// Panics if the slice is out of range.
    #[must_use]
    pub fn string(&self, slice: StringSlice) -> &'a str {
        super::text(&self.strings[slice.range()])
    }
}
