//! Helpers shared by the property tests.
#![allow(dead_code)]

use std::{io::Write, path::PathBuf};

pub(crate) mod synthetic;

use bytemuck::Pod;
use flatnet_compiler::Compiler;
use flatnet_core::{
    consumer::Consumer,
    models::{
        Date, Header, RouteIndex, SearchIndex, Section, ServiceIndex, StopIndex, TripIndex,
        TripPatternIndex,
    },
};
use memmap2::Mmap;
use tempfile::NamedTempFile;

/// Picks a position in `0..length` from a fraction between 0 and 1.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "the fraction is between 0 and 1 and the length is small"
)]
pub(crate) fn scale(fraction: f64, length: usize) -> usize {
    ((fraction * length as f64) as usize).min(length.saturating_sub(1))
}

pub(crate) fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gtfs")
}

pub(crate) fn compile_fixture() -> Vec<u8> {
    Compiler::new(&fixture_path())
        .compile()
        .expect("the fixture is valid")
}

#[allow(
    unsafe_code,
    reason = "mapping a file is unsafe in memmap2, and this file is not modified"
)]
pub(crate) fn map(bytes: &[u8]) -> Option<(NamedTempFile, Mmap)> {
    let mut file = NamedTempFile::new().expect("a temp file can be created");
    file.write_all(bytes).expect("the temp file is writable");
    // An empty file can't be mapped, and the consumer never sees one.
    // SAFETY: the file is private to this test and isn't modified while mapped.
    let mmap = unsafe { Mmap::map(file.as_file()) }.ok()?;
    Some((file, mmap))
}

/// Lets `damage` change the items of one section of a compiled file, and returns the bytes.
pub(crate) fn damage_section<T: Pod>(
    mut bytes: Vec<u8>,
    section_of: fn(&Header) -> Section,
    damage: impl FnOnce(&mut [T]),
) -> Vec<u8> {
    let header: Header = bytemuck::pod_read_unaligned(&bytes[..size_of::<Header>()]);
    let section = section_of(&header);
    let start = usize::try_from(section.offset).expect("an offset fits in usize");
    let length = usize::try_from(section.count).expect("a count fits in usize") * size_of::<T>();
    damage(bytemuck::cast_slice_mut(&mut bytes[start..start + length]));
    bytes
}

/// Whether the consumer refuses to open these bytes.
pub(crate) fn is_rejected(bytes: &[u8]) -> bool {
    let (_file, mmap) = map(bytes).expect("the file maps");
    Consumer::new(&mmap).is_err()
}

/// Calls every accessor with every index the file holds. Any panic fails the test.
pub(crate) fn exercise(consumer: &Consumer<'_>) {
    let date = Date::parse_gtfs("20170102").expect("a valid date");
    for index in 0..consumer.stops().len() {
        let stop = consumer.stop(StopIndex::from(index));
        if let Some(parent) = stop.parent_index.as_option() {
            let _ = consumer.stop(parent);
        }
        let _ = consumer.stop_id(stop.id);
        let _ = consumer.iter_trips_by_stop(stop.index).count();
        let _ = consumer.outbound_transfers_by_stop(stop.index);
        let _ = consumer.iter_inbound_transfers_by_stop(stop.index).count();
        let _ = consumer.iter_trip_patterns_by_stop(stop.index).count();
        if let Some(name) = stop.name.as_option() {
            let _ = consumer.string(name);
        }
    }
    for index in 0..consumer.routes().len() {
        let route = consumer.route(RouteIndex::from(index));
        let _ = consumer.route_id(route.id);
        let _ = consumer.iter_trips_by_route(route.index).count();
    }
    for index in 0..consumer.trips().len() {
        let trip = consumer.trip(TripIndex::from(index));
        let _ = consumer.trip_id(trip.id);
        let _ = consumer.route(trip.route_index);
        let pattern = consumer.trip_pattern_by_trip(trip.index);
        for &stop in consumer.stop_sequence_by_trip_pattern(pattern.index) {
            let _ = consumer.stop(stop);
        }
        for pattern_stop in consumer.pattern_stops_by_trip_pattern(pattern.index) {
            if let Some(headsign) = pattern_stop.headsign.as_option() {
                let _ = consumer.string(headsign);
            }
        }
        let _ = consumer.timings_by_trip(trip.index);
        let _ = consumer.is_service_active(trip.service_index, date);
        for &shape in consumer.shapes_by_trip(trip.index) {
            let _ = consumer.shape_coordinates(shape);
        }
    }
    for index in 0..consumer.services().len() {
        let service = consumer.service(ServiceIndex::from(index));
        let _ = consumer.service_id(service.id);
        for offset in 0..3 {
            let _ = consumer.is_service_active(service.index, Date(service.start_date.0 + offset));
        }
    }
    for index in 0..consumer.trip_patterns().len() {
        let index = TripPatternIndex::from(index);
        let _ = consumer.iter_stop_sequence_by_trip_pattern(index).count();
        let _ = consumer.iter_trips_in_trip_pattern(index).count();
        let _ = consumer.trip_pattern(index);
    }
    for index in 0..consumer.search_stops().len() {
        let search = consumer.search(SearchIndex::from(index));
        let _ = consumer.string(search.name);
        let _ = consumer.iter_stops_by_search(search.index).count();
    }
    for transfer in consumer.transfers() {
        let _ = consumer.stop(transfer.from_stop_index);
        let _ = consumer.stop(transfer.to_stop_index);
    }
    let mut near = Vec::new();
    consumer.stops_near(48.86, 2.35, 5000.0, &mut near);
    for &(stop, _) in &near {
        let _ = consumer.stop(stop);
    }
    let mut found = Vec::new();
    consumer.search_by_name("central", &mut found);
    for &index in &found {
        let _ = consumer.search(index);
    }
    let _ = consumer.timezone();
    let _ = consumer.service_range();
    let _ = consumer.feed_start_date();
    drop(consumer.verify_checksum());
    let _ = consumer.stop_by_id("stop1");
    let _ = consumer.trip_by_id("trip1");
    let _ = consumer.route_by_id("route1");
    let _ = consumer.service_by_id("service1");
}

pub(crate) fn open_and_exercise(bytes: &[u8]) {
    let Some((_file, mmap)) = map(bytes) else {
        return;
    };
    if let Ok(consumer) = Consumer::new(&mmap) {
        exercise(&consumer);
    }
}
