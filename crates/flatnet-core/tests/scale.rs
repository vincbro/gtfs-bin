//! A network far bigger than the fixture, and real data when it is available.

mod common;

use std::{env, fs, path::PathBuf};

use common::synthetic::{self, Size};
use flatnet_compiler::Compiler;
use flatnet_core::{
    consumer::Consumer,
    rt::{self, Realtime, RealtimeBuilder},
};

#[test]
fn a_generated_network_compiles_validates_and_answers_queries() {
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    synthetic::write_feed(directory.path(), Size::SMALL);

    let bytes = Compiler::new(directory.path())
        .compile()
        .expect("a valid feed");

    let (_file, mmap) = common::map(&bytes).expect("the file maps");
    let consumer = Consumer::new(&mmap).expect("the compiler wrote a valid file");
    assert_eq!(
        consumer.stops().len(),
        Size::SMALL.stops,
        "every stop is kept"
    );
    assert_eq!(
        consumer.trips().len(),
        Size::SMALL.trips,
        "every trip is kept"
    );
    assert_eq!(
        consumer.timings().len(),
        Size::SMALL.trips * Size::SMALL.stops_per_trip,
        "every stop time is kept"
    );
    assert_eq!(
        consumer.timezone(),
        Some("Europe/Stockholm"),
        "the agency's zone"
    );
    assert!(consumer.verify_checksum().is_ok(), "the checksum matches");
    common::exercise(&consumer);

    let mut found = Vec::new();
    consumer.stops_near(59.35, 18.05, 3_000.0, &mut found);
    assert!(
        !found.is_empty(),
        "stops are spread over about 30 km, so some are within 3 km"
    );
}

#[test]
fn compiling_a_generated_network_twice_gives_the_same_bytes() {
    let directory = tempfile::tempdir().expect("a temp directory can be created");
    synthetic::write_feed(directory.path(), Size::SMALL);

    let first = Compiler::new(directory.path())
        .compile()
        .expect("a valid feed");
    let second = Compiler::new(directory.path())
        .compile()
        .expect("a valid feed");

    assert_eq!(first, second, "output must not depend on hash map order");
}

/// Runs against real data when `FLATNET_REAL_FEED` names a folder holding `static.zip` and any
/// number of `.pb` realtime files. Without it the test has nothing to do and passes.
#[test]
fn real_data_compiles_and_matches_its_realtime_feeds() {
    let Some(folder) = env::var_os("FLATNET_REAL_FEED").map(PathBuf::from) else {
        eprintln!("FLATNET_REAL_FEED isn't set, so there is no real feed to try");
        return;
    };

    let bytes = Compiler::new(&folder.join("static.zip"))
        .compile()
        .expect("the real feed compiles");
    let (_file, mmap) = common::map(&bytes).expect("the file maps");
    let consumer = Consumer::new(&mmap).expect("the compiler wrote a valid file");
    assert!(consumer.verify_checksum().is_ok(), "the checksum matches");

    let mut realtime = Realtime::new(&consumer);
    let builder = RealtimeBuilder::new(&consumer);
    let mut matched = 0;
    let mut all = 0;
    for entry in fs::read_dir(&folder).expect("the folder is readable") {
        let path = entry.expect("a readable entry").path();
        if path.extension().is_none_or(|extension| extension != "pb") {
            continue;
        }
        let message = rt::decode_feed(&fs::read(&path).expect("a readable file"))
            .expect("a real feed decodes");
        let report = builder.apply([message].into_iter(), &mut realtime);
        eprintln!("{}: {report:?}", path.display());
        let applied = report.trip_updates + report.vehicles + report.alerts;
        matched += applied;
        all += applied + report.trip_updates_unmatched + report.vehicles_unmatched;
        assert_eq!(
            report.stop_time_updates_unmatched, 0,
            "every stop update finds its stop time"
        );
    }
    assert!(
        all == 0 || matched * 10 >= all * 9,
        "at least nine in ten entities should match"
    );
}
