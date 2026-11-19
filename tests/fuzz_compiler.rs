//! Property tests that feed the compiler damaged feeds.
//!
//! Whatever the feed holds, compiling must return an error or a file the consumer accepts. It must
//! never panic, and it must never write a file that its own reader rejects.

mod common;

use std::fs;

use gtfs_bin::{compiler::Compiler, consumer::Consumer};
use proptest::prelude::*;

const FILES: [&str; 10] = [
    "stops.txt",
    "routes.txt",
    "trips.txt",
    "stop_times.txt",
    "calendar.txt",
    "calendar_dates.txt",
    "transfers.txt",
    "shapes.txt",
    "agency.txt",
    "feed_info.txt",
];

/// Characters that are easy to get wrong in a csv file or a date.
const AWKWARD: [char; 14] = [
    ',', '"', '\n', '\r', '9', '0', '-', ' ', 'x', '\u{feff}', 'é', '/', ':', '.',
];

#[derive(Clone, Debug)]
enum Edit {
    Replace(f64, usize),
    Insert(f64, usize),
    DeleteLine(f64),
    DuplicateLine(f64),
    Truncate(f64),
}

fn edit() -> impl Strategy<Value = Edit> {
    prop_oneof![
        (0.0_f64..1.0, 0..AWKWARD.len()).prop_map(|(at, c)| Edit::Replace(at, c)),
        (0.0_f64..1.0, 0..AWKWARD.len()).prop_map(|(at, c)| Edit::Insert(at, c)),
        (0.0_f64..1.0).prop_map(Edit::DeleteLine),
        (0.0_f64..1.0).prop_map(Edit::DuplicateLine),
        (0.0_f64..1.0).prop_map(Edit::Truncate),
    ]
}

fn apply(text: &str, edit: &Edit) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return String::new();
    }
    let at = |fraction: &f64, length: usize| ((fraction * length as f64) as usize).min(length - 1);
    match edit {
        Edit::Replace(fraction, character) => {
            let position = at(fraction, chars.len());
            chars[position] = AWKWARD[*character];
            chars.into_iter().collect()
        }
        Edit::Insert(fraction, character) => {
            chars.insert(at(fraction, chars.len()), AWKWARD[*character]);
            chars.into_iter().collect()
        }
        Edit::Truncate(fraction) => chars[..at(fraction, chars.len())].iter().collect(),
        Edit::DeleteLine(fraction) | Edit::DuplicateLine(fraction) => {
            let mut lines: Vec<&str> = text.split('\n').collect();
            let position = at(fraction, lines.len());
            if matches!(edit, Edit::DeleteLine(_)) {
                lines.remove(position);
            } else {
                lines.insert(position, lines[position]);
            }
            lines.join("\n")
        }
    }
}

fn compile_damaged(damage: &[(usize, Vec<Edit>)]) {
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    for name in FILES {
        let Ok(text) = fs::read_to_string(common::fixture_path().join(name)) else {
            continue;
        };
        fs::write(directory.path().join(name), text).expect("the temp directory is writable");
    }
    for (file, edits) in damage {
        let path = directory.path().join(FILES[file % FILES.len()]);
        let Ok(mut text) = fs::read_to_string(&path) else {
            continue;
        };
        for edit in edits {
            text = apply(&text, edit);
        }
        fs::write(&path, text).expect("the temp directory is writable");
    }

    let Ok(bytes) = Compiler::new(directory.path()).compile() else {
        return;
    };
    let (_file, mmap) = common::map(&bytes).expect("a compiled file is never empty");
    let consumer = Consumer::new(&mmap)
        .map_err(|error| error.to_string())
        .expect("the compiler wrote a file it can't read");
    common::exercise(&consumer);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn damaged_feeds_never_panic_and_never_miscompile(
        damage in proptest::collection::vec(
            (0..FILES.len(), proptest::collection::vec(edit(), 1..6)),
            1..4,
        ),
    ) {
        compile_damaged(&damage);
    }
}

#[test]
fn a_missing_required_file_is_an_error() {
    let directory = tempfile::tempdir().expect("a temp directory can be created");

    let result = Compiler::new(directory.path()).compile();

    assert!(result.is_err(), "an empty directory is not a feed");
}
