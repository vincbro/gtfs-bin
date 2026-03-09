use crate::models::Section;
use bytemuck::{NoUninit, checked::cast_slice};

#[derive(Clone, Debug, Default)]
pub struct BinaryWriter {
    buffer: Vec<u8>,
}

impl BinaryWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    pub fn resize(mut self, size: usize) -> Self {
        self.buffer.resize(self.buffer.len() + size, 0);
        self.align();
        self
    }

    pub fn write_section<T: NoUninit>(&mut self, slice: &[T]) -> Section {
        let offset = self.buffer.len() as u64;
        let count = slice.len() as u64;
        self.buffer.extend_from_slice(cast_slice(slice));
        self.align();
        Section { offset, count }
    }

    pub fn overwrite(&mut self, offset: usize, bytes: &[u8]) {
        self.buffer
            .splice(offset..(offset + bytes.len()), bytes.iter().copied());
    }

    fn align(&mut self) {
        let len = self.buffer.len();
        let rem = len % 8;
        if rem > 0 {
            self.buffer.resize(len + (8 - rem), 0);
        }
    }

    pub fn take(self) -> Vec<u8> {
        self.buffer
    }
}
