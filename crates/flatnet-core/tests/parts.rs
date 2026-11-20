//! The optional parts of a file: what a compile without them leaves out, and that the consumer
//! refuses a file that says a part is absent and holds it anyway.

mod common;

use flatnet_compiler::{Compiler, Error};
use flatnet_core::{
    consumer::Consumer,
    models::{Header, Parts},
};

fn compile(parts: Parts) -> Vec<u8> {
    Compiler::new()
        .parts(parts)
        .compile_gtfs(&common::fixture_path())
        .expect("the fixture is valid")
}

fn open(bytes: &[u8], body: impl FnOnce(&Consumer<'_>)) {
    let (_file, mmap) = common::map(bytes).expect("a compiled file maps");
    body(&Consumer::new(&mmap).expect("the compiler wrote a valid file"));
}

#[test]
fn a_file_has_every_part_by_default() {
    open(&common::compile_fixture(), |consumer| {
        assert_eq!(
            consumer.parts(),
            Parts::ALL,
            "nothing is left out unless asked"
        );
    });
}

#[test]
fn a_file_without_any_part_opens_and_every_accessor_runs() {
    let bytes = compile(Parts::NONE);

    open(&bytes, |consumer| {
        assert_eq!(
            consumer.parts(),
            Parts::NONE,
            "the header says what was left out"
        );
        assert!(consumer.shapes().is_empty(), "no shapes");
        assert!(consumer.search_stops().is_empty(), "nothing to search");
        assert!(consumer.stops_id_lookup().is_empty(), "no lookup by id");
        assert!(consumer.stop_ids().is_empty(), "no ids");
        common::exercise(consumer);
    });
}

#[test]
fn a_file_without_any_part_is_smaller() {
    let full = compile(Parts::ALL);
    let routing = compile(Parts::NONE);

    assert!(
        routing.len() < full.len(),
        "{} bytes without the parts, {} with them",
        routing.len(),
        full.len()
    );
}

#[test]
fn ids_and_names_read_as_nothing_without_their_parts() {
    open(&compile(Parts::ALL.without(Parts::IDS)), |consumer| {
        assert!(consumer.stop_by_id("stop8").is_none(), "no lookup by id");
        assert_eq!(
            consumer.stop_id(consumer.stops()[0].id),
            "",
            "an id reads as empty"
        );
        assert!(!consumer.search_stops().is_empty(), "search is still there");
    });
    open(
        &compile(Parts::ALL.without(Parts::SEARCH.with(Parts::TEXT))),
        |consumer| {
            let info = consumer.stop_info(consumer.stops()[0].index);
            assert!(info.name.is_none(), "the name is left out");
            assert!(consumer.stop_by_id("stop8").is_some(), "ids stay");
        },
    );
}

#[test]
fn search_without_text_is_refused() {
    let result = Compiler::new()
        .parts(Parts::ALL.without(Parts::TEXT))
        .compile_gtfs(&common::fixture_path());

    assert!(
        matches!(result, Err(Error::SearchWithoutText)),
        "a place is found by its name"
    );
}

#[test]
fn a_header_that_leaves_out_a_part_the_file_holds_is_rejected() {
    for lie in [Parts::IDS, Parts::TEXT, Parts::SEARCH] {
        let mut bytes = common::compile_fixture();
        let mut header: Header = bytemuck::pod_read_unaligned(&bytes[..size_of::<Header>()]);
        header.parts = Parts::ALL.without(lie);
        bytes[..size_of::<Header>()].copy_from_slice(bytemuck::bytes_of(&header));

        assert!(
            common::is_rejected(&bytes),
            "{lie:?} is claimed absent but present"
        );
    }
}

#[test]
fn a_header_with_an_unknown_part_is_rejected() {
    let mut bytes = common::compile_fixture();
    let mut header: Header = bytemuck::pod_read_unaligned(&bytes[..size_of::<Header>()]);
    header.parts = Parts(1 << 40);
    bytes[..size_of::<Header>()].copy_from_slice(bytemuck::bytes_of(&header));

    assert!(common::is_rejected(&bytes), "no such part");
}
