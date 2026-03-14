use bytemuck::{Pod, cast_slice};
use memmap2::Mmap;

use crate::models::Section;

pub struct Reader<'a> {
    mmap: &'a Mmap,
}

impl<'a> Reader<'a> {
    pub fn new(mmap: &'a Mmap) -> Self {
        Self { mmap }
    }

    pub fn get_bytes<T>(&self, section: Section) -> &'a [u8] {
        let start = section.offset as usize;
        let end = start + (section.count as usize * size_of::<T>());
        &self.mmap[start..end]
    }

    pub fn cast_slice<B: Pod>(&self, section: Section) -> &'a [B] {
        cast_slice(self.get_bytes::<B>(section))
    }
}
