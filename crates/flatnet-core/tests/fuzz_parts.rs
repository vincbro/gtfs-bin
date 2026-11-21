//! Property tests that damage files compiled with every combination of optional parts, and files
//! from arbitrary networks, not only the one fixture.

mod common;

use common::arbitrary;
use flatnet_compiler::{Compiler, Network};
use flatnet_core::{
    consumer::Consumer,
    models::{Header, Parts},
};
use proptest::{collection, prelude::*};

fn compile(parts: u64, network: Network) -> Option<Vec<u8>> {
    Compiler::new().parts(Parts(parts)).compile(network).ok()
}

/// One piece of damage: where, how, and with what.
#[derive(Debug, Clone)]
struct Damage {
    section: usize,
    fraction: f64,
    kind: u8,
    random: u32,
}

fn damage() -> impl Strategy<Value = Damage> {
    (any::<usize>(), 0.0_f64..1.0, 0_u8..7, any::<u32>()).prop_map(
        |(section, fraction, kind, random)| Damage {
            section,
            fraction,
            kind,
            random,
        },
    )
}

/// Writes one piece of damage: a word of a section set to a small, huge or random value, a byte
/// flipped, or two words swapped.
fn apply(bytes: &mut [u8], sections: &[(usize, usize)], damage: &Damage) {
    let (start, end) = sections[damage.section % sections.len()];
    if end - start < 4 {
        return;
    }
    let position = start + common::scale(damage.fraction, end - start - 3) / 4 * 4;
    let value = match damage.kind {
        0 => 0,
        1 => 1,
        2 => u32::MAX,
        3 => u32::MAX - 1,
        4 => damage.random % 64,
        5 => damage.random,
        _ => {
            let other =
                start + common::scale(f64::from(damage.random) / 4.3e9, end - start - 3) / 4 * 4;
            for offset in 0..4 {
                bytes.swap(position + offset, other + offset);
            }
            return;
        }
    };
    bytes[position..position + 4].copy_from_slice(&value.to_le_bytes());
}

fn check(original: &[u8], damaged: &[u8]) -> Result<(), TestCaseError> {
    let Some((_file, mmap)) = common::map(damaged) else {
        return Ok(());
    };
    let Ok(consumer) = Consumer::new(&mmap) else {
        return Ok(());
    };
    common::exercise(&consumer);
    let body = size_of::<Header>();
    if damaged.len() == original.len() && damaged[body..] != original[body..] {
        prop_assert!(
            consumer.verify_checksum().is_err(),
            "a changed body must fail its checksum"
        );
    }
    Ok(())
}

proptest! {

    #[test]
    fn the_fixture_with_any_parts_opens_and_survives_damage(
        parts in 0_u64..32,
        edits in collection::vec(damage(), 0..6),
        cut in prop_oneof![Just(None), (0_usize..100_000).prop_map(Some)],
    ) {
        let Ok(original) = Compiler::new()
            .parts(Parts(parts))
            .compile_gtfs(&common::fixture_path())
        else {
            return Ok(());
        };
        let (_file, mmap) = common::map(&original).expect("the file maps");
        let consumer = Consumer::new(&mmap).expect("an undamaged file opens");
        common::exercise(&consumer);
        prop_assert!(consumer.verify_checksum().is_ok(), "an undamaged file has its checksum");

        let sections = common::section_ranges(&original);
        let mut damaged = original.clone();
        for edit in &edits {
            apply(&mut damaged, &sections, edit);
        }
        if let Some(length) = cut {
            damaged.truncate(length.min(damaged.len()));
        }
        check(&original, &damaged)?;
    }

    #[test]
    fn an_arbitrary_network_survives_damage(
        network in arbitrary::network(),
        parts in 0_u64..32,
        edits in collection::vec(damage(), 1..8),
    ) {
        let Some(original) = compile(parts, network) else {
            return Ok(());
        };
        let sections = common::section_ranges(&original);
        let mut damaged = original.clone();
        for edit in &edits {
            apply(&mut damaged, &sections, edit);
        }
        check(&original, &damaged)?;
    }
}
