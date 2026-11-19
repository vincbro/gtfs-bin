use bytemuck::NoUninit;

use crate::models::Section;

/// Appends sections to a byte buffer, keeping each one aligned to 8 bytes.
#[derive(Clone, Debug, Default)]
pub(super) struct BinaryWriter {
    buffer: Vec<u8>,
}

impl BinaryWriter {
    #[must_use]
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Reserves `size` zeroed bytes, such as room for a header written last.
    #[must_use]
    pub(super) fn with_reserved(mut self, size: usize) -> Self {
        self.buffer.resize(self.buffer.len() + size, 0);
        self.align();
        self
    }

    pub(super) fn write_section<T: NoUninit>(&mut self, slice: &[T]) -> Section {
        let offset = self.buffer.len() as u64;
        let count = slice.len() as u64;
        self.buffer
            .extend_from_slice(bytemuck::checked::cast_slice(slice));
        self.align();
        Section { offset, count }
    }

    /// Returns everything written so far.
    #[must_use]
    pub(super) fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }

    /// Replaces the bytes at `offset`.
    ///
    /// # Panics
    ///
    /// Panics if the bytes don't fit in what has been written.
    pub(super) fn overwrite(&mut self, offset: usize, bytes: &[u8]) {
        self.buffer[offset..offset + bytes.len()].copy_from_slice(bytes);
    }

    fn align(&mut self) {
        let remainder = self.buffer.len() % 8;
        if remainder > 0 {
            self.buffer.resize(self.buffer.len() + (8 - remainder), 0);
        }
    }

    #[must_use]
    pub(super) fn into_bytes(self) -> Vec<u8> {
        self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_start_on_eight_byte_boundaries() {
        let mut writer = BinaryWriter::new();

        let first = writer.write_section(&[1_u8, 2, 3]);
        let second = writer.write_section(&[4_u8]);

        assert_eq!(first.offset, 0, "the first section starts at the beginning");
        assert_eq!(first.count, 3, "count is items, not bytes");
        assert_eq!(second.offset, 8, "the three bytes were padded up to eight");
    }

    #[test]
    fn overwrite_replaces_reserved_bytes() {
        let mut writer = BinaryWriter::new().with_reserved(8);

        writer.overwrite(0, &[7, 7]);

        assert_eq!(
            &writer.into_bytes()[..3],
            &[7, 7, 0],
            "only the given bytes change"
        );
    }

    #[test]
    #[should_panic(expected = "range end index")]
    fn overwrite_past_the_end_panics() {
        let mut writer = BinaryWriter::new();

        writer.overwrite(0, &[1]);
    }
}
