/// Hashes bytes to catch a damaged file. It is fast, not secure: it can't stop someone who edits a
/// file on purpose.
#[must_use]
pub fn checksum(bytes: &[u8]) -> u64 {
    const MULTIPLIER: u64 = 0x517c_c1b7_2722_0a95;

    let mut hash = bytes.len() as u64;
    let mut words = bytes.chunks_exact(8);
    for word in &mut words {
        let word = word
            .iter()
            .rev()
            .fold(0_u64, |value, &byte| value << 8 | u64::from(byte));
        hash = (hash.rotate_left(5) ^ word).wrapping_mul(MULTIPLIER);
    }
    for &byte in words.remainder() {
        hash = (hash.rotate_left(5) ^ u64::from(byte)).wrapping_mul(MULTIPLIER);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_changed_byte_changes_the_hash() {
        let original = vec![7_u8; 100];
        let mut changed = original.clone();
        changed[63] = 8;

        assert_ne!(
            checksum(&original),
            checksum(&changed),
            "a flipped byte must show"
        );
    }

    #[test]
    fn length_matters() {
        assert_ne!(
            checksum(&[0; 8]),
            checksum(&[0; 16]),
            "zeros of different lengths differ"
        );
        assert_ne!(
            checksum(&[]),
            checksum(&[0]),
            "an empty input differs from a zero byte"
        );
    }

    #[test]
    fn the_tail_is_hashed() {
        assert_ne!(
            checksum(&[1; 11]),
            checksum(&[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2]),
            "bytes past the last word count"
        );
    }
}
