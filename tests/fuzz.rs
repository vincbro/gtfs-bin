//! Property tests that feed the consumer arbitrary and damaged files.

mod common;

use gtfs_bin::{consumer::Consumer, models::Header};
use proptest::{collection, prelude::*};

/// The byte range of every non-empty section, read from the header.
fn section_ranges(bytes: &[u8]) -> Vec<(usize, usize)> {
    // The header ends with two dates, the file length and the checksum, which aren't sections.
    let section_bytes = size_of::<Header>() - 8 - 24;
    let words: &[u64] = bytemuck::cast_slice(&bytes[8..8 + section_bytes]);
    let mut starts: Vec<usize> = words
        .chunks(2)
        .map(|pair| usize::try_from(pair[0]).expect("an offset in a file fits in usize"))
        .collect();
    starts.push(bytes.len());
    starts.sort_unstable();
    starts.dedup();
    starts
        .windows(2)
        .map(|pair| (pair[0], pair[1]))
        .filter(|&(start, end)| end > start)
        .collect()
}

#[test]
fn the_untouched_fixture_passes_the_exercise() {
    let bytes = common::compile_fixture();
    let (_file, mmap) = common::map(&bytes).expect("the fixture maps");
    common::exercise(&Consumer::new(&mmap).expect("the fixture is valid"));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn arbitrary_bytes_never_panic(bytes in collection::vec(any::<u8>(), 0..2048)) {
        common::open_and_exercise(&bytes);
    }

    #[test]
    fn damaged_files_never_panic(
        edits in collection::vec((0.0_f64..1.0, any::<u8>()), 1..24),
        header_bias in any::<bool>(),
    ) {
        let mut bytes = common::compile_fixture();
        let limit = if header_bias { 400.min(bytes.len()) } else { bytes.len() };
        for (fraction, value) in edits {
            let position = common::scale(fraction, limit);
            bytes[position] = value;
        }
        common::open_and_exercise(&bytes);
    }

    #[test]
    fn truncated_files_never_panic(length in 0_usize..100_000) {
        let bytes = common::compile_fixture();
        common::open_and_exercise(&bytes[..length.min(bytes.len())]);
    }

    #[test]
    fn damaged_table_entries_never_panic(
        edits in collection::vec(
            (any::<usize>(), 0.0_f64..1.0, 0_u8..6, any::<u32>()),
            1..8,
        ),
    ) {
        // Random bytes mostly miss the tables, so aim at a word inside a random section and write
        // a value that is small, huge or just past a plausible end.
        let mut bytes = common::compile_fixture();
        let sections = section_ranges(&bytes);
        for (section, fraction, kind, random) in edits {
            let (start, end) = sections[section % sections.len()];
            if end - start < 4 {
                continue;
            }
            let position = start + common::scale(fraction, end - start - 3) / 4 * 4;
            let value = match kind {
                0 => 0,
                1 => 1,
                2 => 40,
                3 => u32::MAX,
                4 => u32::MAX - 1,
                _ => random,
            };
            bytes[position..position + 4].copy_from_slice(&value.to_le_bytes());
        }
        common::open_and_exercise(&bytes);
    }

    #[test]
    fn swapped_words_never_panic(
        swaps in collection::vec((0.0_f64..1.0, 0.0_f64..1.0), 1..16),
    ) {
        // Moving whole words around keeps the values plausible, which gets past the cheap checks
        // and into the index validation.
        let mut bytes = common::compile_fixture();
        let words = bytes.len() / 4;
        for (first, second) in swaps {
            let first = common::scale(first, words) * 4;
            let second = common::scale(second, words) * 4;
            for offset in 0..4 {
                bytes.swap(first + offset, second + offset);
            }
        }
        common::open_and_exercise(&bytes);
    }
}
