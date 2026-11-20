//! What the file says about itself: the fixed fields, the JSON and the stale check.

use std::{
    fs,
    path::{Path, PathBuf},
};

use flatnet_compiler::{Compiler, Error};
use flatnet_core::{
    consumer::Consumer,
    models::{Date, Header, Metadata, Optional, Section, StringSlice},
};
use memmap2::Mmap;

mod common;

fn write_feed(directory: &Path) {
    let files = [
        (
            "agency.txt",
            "agency_id,agency_name,agency_url,agency_timezone\nA,Dummy Transit,https://dummy.example,Europe/Stockholm\n",
        ),
        (
            "feed_info.txt",
            "feed_publisher_name,feed_publisher_url,feed_lang,feed_version,feed_contact_email,feed_start_date,feed_end_date\nDummy Org,https://org.example,sv,2026-11-01,hello@org.example,20260101,20261231\n",
        ),
        (
            "stops.txt",
            "stop_id,stop_name,stop_lat,stop_lon\nS0,S0,59.0,18.0\nS1,S1,59.01,18.0\n",
        ),
        (
            "routes.txt",
            "route_id,route_short_name,route_type\nR,R,3\n",
        ),
        (
            "calendar.txt",
            "service_id,monday,tuesday,wednesday,thursday,friday,saturday,sunday,start_date,end_date\nA,1,1,1,1,1,1,1,20260301,20260630\nB,1,1,1,1,1,1,1,20260201,20260501\n",
        ),
        ("trips.txt", "route_id,service_id,trip_id\nR,A,T1\n"),
        (
            "stop_times.txt",
            "trip_id,arrival_time,departure_time,stop_id,stop_sequence\nT1,08:00:00,08:00:00,S0,0\nT1,08:05:00,08:05:00,S1,1\n",
        ),
    ];
    for (name, contents) in files {
        fs::write(directory.join(name), contents).expect("a feed file can be written");
    }
}

/// Writes the feed into a folder called `network`, which is kept until the first value is dropped.
fn new_feed() -> (tempfile::TempDir, PathBuf) {
    let parent = tempfile::tempdir().expect("a temp directory can be created");
    let directory = parent.path().join("network");
    fs::create_dir(&directory).expect("the feed folder can be made");
    write_feed(&directory);
    (parent, directory)
}

/// Compiles the feed after `configure` has set the compiler up.
fn compile(configure: impl FnOnce(Compiler) -> Compiler) -> Result<Vec<u8>, Error> {
    let (_parent, directory) = new_feed();
    configure(Compiler::new()).compile_gtfs(&directory)
}

fn with_consumer(bytes: &[u8], check: impl FnOnce(&Consumer<'_>)) {
    let (_file, mmap): (_, Mmap) = common::map(bytes).expect("the file maps");
    check(&Consumer::new(&mmap).expect("the file is valid"));
}

fn text<'a>(consumer: &Consumer<'a>, slice: Optional<StringSlice>) -> Option<&'a str> {
    slice.as_option().map(|slice| consumer.string(slice))
}

fn day(year: u32, month: u32, date: u32) -> Date {
    Date::from_year_month_day(year, month, date).expect("a real day")
}

#[test]
fn the_fixed_fields_come_from_the_feed() {
    let bytes = compile(|compiler| {
        compiler
            .compiled_at(1_700_000_000)
            .source("network.zip".to_owned())
    })
    .expect("the feed compiles");

    with_consumer(&bytes, |consumer| {
        let metadata = consumer.metadata();
        assert_eq!(metadata.compiled_at, 1_700_000_000);
        assert_eq!(text(consumer, metadata.source), Some("network.zip"));
        assert_eq!(text(consumer, metadata.publisher_name), Some("Dummy Org"));
        assert_eq!(
            text(consumer, metadata.publisher_url),
            Some("https://org.example")
        );
        assert_eq!(text(consumer, metadata.language), Some("sv"));
        assert_eq!(text(consumer, metadata.feed_version), Some("2026-11-01"));
        assert_eq!(
            text(consumer, metadata.contact_email),
            Some("hello@org.example")
        );
        assert_eq!(text(consumer, metadata.agency_name), Some("Dummy Transit"));
        assert_eq!(
            text(consumer, metadata.agency_url),
            Some("https://dummy.example")
        );
        assert!(
            text(consumer, metadata.compiler_version).is_some_and(|version| !version.is_empty()),
            "the compiler says which version it is"
        );
    });
}

#[test]
fn the_time_and_the_source_are_not_known_unless_the_compiler_is_told() {
    let bytes = compile(|compiler| compiler).expect("the feed compiles");

    with_consumer(&bytes, |consumer| {
        assert_eq!(consumer.metadata().compiled_at, 0);
        assert_eq!(
            consumer.metadata().source.as_option(),
            None,
            "not taken from the path"
        );
    });
}

#[test]
fn the_active_range_covers_every_service() {
    let bytes = compile(|compiler| compiler).expect("the feed compiles");

    with_consumer(&bytes, |consumer| {
        assert_eq!(
            consumer.service_range(),
            Some((day(2026, 2, 1), day(2026, 6, 30))),
            "from the earliest start to the latest end"
        );
    });
}

#[test]
fn a_file_is_stale_the_day_after_its_last_service() {
    let bytes = compile(|compiler| compiler).expect("the feed compiles");

    with_consumer(&bytes, |consumer| {
        assert!(
            !consumer.is_stale(day(2026, 1, 1)),
            "not started is not stale"
        );
        assert!(
            !consumer.is_stale(day(2026, 6, 30)),
            "the last day still runs"
        );
        assert!(consumer.is_stale(day(2026, 7, 1)), "the day after does not");
    });
}

#[test]
fn json_and_entries_are_merged_with_the_entries_winning() {
    let bytes = compile(|compiler| {
        compiler
            .metadata_json(r#"{"owner": "someone", "tags": ["a", "b"]}"#.to_owned())
            .metadata_entry("owner".to_owned(), "vince".to_owned())
            .metadata_entry("env".to_owned(), "test".to_owned())
    })
    .expect("the feed compiles");

    with_consumer(&bytes, |consumer| {
        assert_eq!(
            consumer.metadata_json(),
            Some(r#"{"env":"test","owner":"vince","tags":["a","b"]}"#),
            "compact, keys sorted, the entry replaced the owner"
        );
    });
}

#[test]
fn no_json_is_none() {
    let bytes = compile(|compiler| compiler).expect("the feed compiles");

    with_consumer(&bytes, |consumer| {
        assert_eq!(consumer.metadata_json(), None);
    });
}

#[test]
fn json_that_is_not_json_is_an_error() {
    let result = compile(|compiler| compiler.metadata_json("{oops".to_owned()));

    assert!(matches!(result, Err(Error::InvalidMetadata(_))));
}

#[test]
fn json_that_is_not_an_object_is_an_error() {
    let result = compile(|compiler| compiler.metadata_json("[1, 2]".to_owned()));

    assert!(matches!(result, Err(Error::MetadataNotAnObject)));
}

#[test]
fn metadata_does_not_make_the_output_depend_on_anything_but_its_input() {
    let configure = |compiler: Compiler| {
        compiler
            .compiled_at(5)
            .metadata_entry("a".to_owned(), "1".to_owned())
    };

    assert_eq!(
        compile(configure).expect("the feed compiles"),
        compile(configure).expect("the feed compiles")
    );
}

fn json_feed() -> Vec<u8> {
    compile(|compiler| compiler.metadata_json(r#"{"a": 1}"#.to_owned())).expect("the feed compiles")
}

#[test]
fn a_text_field_past_the_strings_is_rejected() {
    let bytes = common::damage_section::<Metadata>(
        json_feed(),
        |header| header.metadata,
        |items| {
            items[0].source = Optional::new(StringSlice {
                start: 0,
                count: 100_000,
            });
        },
    );

    assert!(
        common::is_rejected(&bytes),
        "the slice ends outside the strings"
    );
}

#[test]
fn a_range_that_ends_before_it_starts_is_rejected() {
    let bytes = common::damage_section::<Metadata>(
        json_feed(),
        |header| header.metadata,
        |items| {
            items[0].active_until = Date(items[0].active_from.0 - 1);
        },
    );

    assert!(common::is_rejected(&bytes), "a range can't run backwards");
}

#[test]
fn a_range_with_only_one_end_is_rejected() {
    let bytes = common::damage_section::<Metadata>(
        json_feed(),
        |header| header.metadata,
        |items| {
            items[0].active_until = Date(u32::MAX);
        },
    );

    assert!(
        common::is_rejected(&bytes),
        "a range has both ends or neither"
    );
}

#[test]
fn a_file_with_no_metadata_item_is_rejected() {
    let mut bytes = json_feed();
    let mut header: Header = bytemuck::pod_read_unaligned(&bytes[..size_of::<Header>()]);
    header.metadata.count = 0;
    bytes[..size_of::<Header>()].copy_from_slice(bytemuck::bytes_of(&header));

    assert!(
        common::is_rejected(&bytes),
        "a file holds exactly one Metadata"
    );
}

#[test]
fn json_that_is_not_text_is_rejected() {
    let bytes = common::damage_section::<u8>(
        json_feed(),
        |header| header.metadata_json,
        |bytes| {
            bytes[0] = 0xff;
        },
    );

    assert!(common::is_rejected(&bytes), "0xff is never utf-8");
}

#[test]
fn the_header_names_both_sections() {
    let bytes = json_feed();
    let header: Header = bytemuck::pod_read_unaligned(&bytes[..size_of::<Header>()]);
    let sections: [Section; 2] = [header.metadata, header.metadata_json];

    assert_eq!(sections[0].count, 1, "one Metadata");
    assert!(sections[1].count > 0, "the json is there");
}
