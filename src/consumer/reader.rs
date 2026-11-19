use bytemuck::Pod;
use memmap2::Mmap;

use crate::{
    consumer::{Error, Result},
    models::Section,
};

pub struct Reader<'a> {
    mmap: &'a Mmap,
}

impl<'a> Reader<'a> {
    pub const fn new(mmap: &'a Mmap) -> Self {
        Self { mmap }
    }

    /// Returns the bytes of `count` items of `T` at the section's offset.
    pub fn bytes<T>(&self, section: Section) -> Result<&'a [u8]> {
        // Overflow is a real case: the section comes from the file, which may be corrupt.
        let offset = usize::try_from(section.offset).map_err(|_| Error::SectionOutOfBound)?;
        let count = usize::try_from(section.count).map_err(|_| Error::SectionOutOfBound)?;
        let end = count
            .checked_mul(size_of::<T>())
            .and_then(|length| offset.checked_add(length))
            .ok_or(Error::SectionOutOfBound)?;
        self.mmap.get(offset..end).ok_or(Error::SectionOutOfBound)
    }

    pub fn cast_slice<B: Pod>(&self, section: Section) -> Result<&'a [B]> {
        bytemuck::try_cast_slice(self.bytes::<B>(section)?).map_err(|_| Error::InvalidSection)
    }
}
