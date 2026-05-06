use bytemuck::{Pod, cast_slice};
use memmap2::Mmap;

use crate::models::Section;

pub struct Reader<'a> {
    mmap: &'a Mmap,
}

impl<'a> Reader<'a> {
    pub const fn new(mmap: &'a Mmap) -> Self {
        Self { mmap }
    }

    pub fn get_bytes<T>(&self, section: Section) -> Result<&'a [u8], crate::Error> {
        let offset =
            usize::try_from(section.offset).map_err(|_| crate::Error::SectionOutOfBound)?;
        let count = usize::try_from(section.count).map_err(|_| crate::Error::SectionOutOfBound)?;
        let end = offset + (count * size_of::<T>());
        if end > self.mmap.len() {
            Err(crate::Error::SectionOutOfBound)
        } else {
            Ok(&self.mmap[offset..end])
        }
    }

    pub fn cast_slice<B: Pod>(&self, section: Section) -> Result<&'a [B], crate::Error> {
        Ok(cast_slice(self.get_bytes::<B>(section)?))
    }
}
