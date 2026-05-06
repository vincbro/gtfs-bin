use crate::{
    consumer::Consumer,
    models::{Slice, StringSlice},
};

impl<'a> Consumer<'a> {
    #[inline]
    #[must_use]
    pub fn string(&self, slice: StringSlice) -> &'a str {
        unsafe { str::from_utf8_unchecked(&self.strings[slice.range()]) }
    }
}
