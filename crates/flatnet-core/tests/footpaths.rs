//! Footpaths between stops, and the settings the file keeps so a reader knows how they were made.

mod common;

use std::ptr;

use flatnet_compiler::{Compiler, Error};
use flatnet_core::{
    consumer::Consumer,
    models::{Footpath, Header, Metadata, Parts, Settings, StopIndex},
};

fn compile(settings: Settings) -> Result<Vec<u8>, Error> {
    Compiler::new()
        .settings(settings)
        .compile_gtfs(&common::fixture_path())
}

fn open(bytes: &[u8], body: impl FnOnce(&Consumer<'_>)) {
    let (_file, mmap) = common::map(bytes).expect("a compiled file maps");
    body(&Consumer::new(&mmap).expect("the compiler wrote a valid file"));
}

fn reach(meters: u32) -> Settings {
    Settings {
        footpath_max_meters: meters,
        ..Settings::default()
    }
}

#[test]
fn the_file_keeps_the_settings_it_was_compiled_with() {
    let settings = Settings {
        footpath_max_meters: 1234,
        footpath_max_per_place: 7,
    };

    open(&compile(settings).expect("valid settings"), |consumer| {
        assert_eq!(
            *consumer.settings(),
            settings,
            "a reader sees what the compiler was told"
        );
    });
    open(&common::compile_fixture(), |consumer| {
        assert_eq!(
            *consumer.settings(),
            Settings::default(),
            "the default is what is kept when nobody chose"
        );
    });
}

#[test]
fn a_longer_reach_finds_the_stops_a_shorter_one_misses() {
    let near = compile(reach(100)).expect("valid settings");
    let far = compile(reach(50_000)).expect("valid settings");

    let count = |bytes: &[u8]| {
        let mut count = 0;
        open(bytes, |consumer| count = consumer.footpaths().len());
        count
    };
    assert!(
        count(&near) > 0,
        "four stop points share one position, so they are no distance apart"
    );
    assert!(
        count(&near) < count(&far),
        "the stops a few hundred meters apart are out of a short reach"
    );
    open(&far, |consumer| {
        assert!(
            !consumer.footpaths().is_empty(),
            "the fixture's stops are all within 50 km"
        );
        for stop in consumer.stops() {
            let walks = consumer.footpaths_by_stop(stop.index);
            assert!(
                walks
                    .windows(2)
                    .all(|pair| pair[0].meters.0 <= pair[1].meters.0),
                "nearest first"
            );
            assert!(
                walks
                    .iter()
                    .all(|walk| walk.to_place_index != consumer.place_of(stop.index)),
                "never to itself"
            );
        }
    });
}

#[test]
fn a_stop_keeps_no_more_footpaths_than_the_settings_allow() {
    let settings = Settings {
        footpath_max_per_place: 1,
        ..reach(50_000)
    };

    open(&compile(settings).expect("valid settings"), |consumer| {
        assert!(
            consumer
                .stops()
                .iter()
                .all(|stop| consumer.footpaths_by_stop(stop.index).len() <= 1),
            "at most one each"
        );
    });
}

#[test]
fn a_reach_a_compile_cannot_search_is_refused() {
    let settings = reach(Settings::MAX_FOOTPATH_METERS + 1);

    assert!(
        matches!(compile(settings), Err(Error::InvalidSettings)),
        "{settings:?} is too far to search"
    );
}

#[test]
fn a_file_with_settings_a_compile_would_refuse_is_rejected() {
    let bytes = common::compile_fixture();
    let damaged = common::damage_section(
        bytes,
        |header| header.metadata,
        |metadata: &mut [Metadata]| {
            metadata[0].settings.footpath_max_meters = u32::MAX;
        },
    );

    assert!(
        common::is_rejected(&damaged),
        "a reader can rely on the settings being usable"
    );
}

#[test]
fn a_file_without_the_footpaths_part_has_none_and_still_opens() {
    let bytes = Compiler::new()
        .settings(reach(50_000))
        .parts(Parts::ALL.without(Parts::FOOTPATHS))
        .compile_gtfs(&common::fixture_path())
        .expect("valid settings");

    open(&bytes, |consumer| {
        assert!(consumer.footpaths().is_empty(), "left out");
        assert!(
            consumer
                .stops()
                .iter()
                .all(|stop| consumer.footpaths_by_stop(stop.index).is_empty()),
            "every stop reads as having none"
        );
        assert_eq!(
            consumer.settings().footpath_max_meters,
            50_000,
            "the settings are kept anyway"
        );
        common::exercise(consumer);
    });
}

#[test]
fn a_header_that_leaves_out_footpaths_the_file_holds_is_rejected() {
    let mut bytes = compile(reach(50_000)).expect("valid settings");
    let mut header: Header = bytemuck::pod_read_unaligned(&bytes[..size_of::<Header>()]);
    header.parts = Parts::ALL.without(Parts::FOOTPATHS);
    bytes[..size_of::<Header>()].copy_from_slice(bytemuck::bytes_of(&header));

    assert!(
        common::is_rejected(&bytes),
        "footpaths are claimed absent but present"
    );
}

#[test]
fn the_stops_under_a_station_are_one_place() {
    open(&common::compile_fixture(), |consumer| {
        let index = |id: &str| consumer.stop_by_id(id).expect("the stop exists").index;
        let (station, first, second, alone) = (
            index("stop1"),
            index("stop3"),
            index("stop5"),
            index("stop2"),
        );

        assert_eq!(
            consumer.place_of(first),
            station,
            "a stop belongs to its station"
        );
        assert_eq!(
            consumer.place_of(alone),
            alone,
            "a stop with no parent is its own place"
        );
        assert_eq!(
            consumer.stops_by_place(station),
            [first, second],
            "the station holds the stops under it, not itself"
        );
        assert!(
            consumer.footpaths_by_place(first).is_empty(),
            "a stop under a station is not a place"
        );
        assert!(
            ptr::eq(
                consumer.footpaths_by_stop(first),
                consumer.footpaths_by_place(station)
            ),
            "it uses its station's walks"
        );
        let walks = consumer.footpaths_by_place(station);
        assert!(
            walks.iter().all(|walk| walk.to_place_index != station),
            "no walk inside a place"
        );
        assert!(
            walks
                .iter()
                .any(|walk| walk.to_place_index == alone && walk.meters.0 == 0.0),
            "the stop at the same position is a walk of nothing"
        );
    });
}

#[test]
fn a_place_that_lists_a_stop_of_another_place_is_rejected() {
    let bytes = common::damage_section(
        common::compile_fixture(),
        |header| header.place_stops,
        |stops: &mut [StopIndex]| stops.fill(StopIndex::from(0_usize)),
    );

    assert!(
        common::is_rejected(&bytes),
        "members are checked against the place that lists them"
    );
}

/// The shortest walk between any stop of one place and any stop of another, worked out from the
/// stops' positions.
fn nearest_walk(consumer: &Consumer<'_>, from: StopIndex, to: StopIndex) -> f64 {
    let mut nearest = f64::MAX;
    for &first in consumer.stops_by_place(from) {
        for &second in consumer.stops_by_place(to) {
            let positions = (
                consumer.stop(first).coordinate.as_option(),
                consumer.stop(second).coordinate.as_option(),
            );
            if let (Some(here), Some(there)) = positions {
                nearest = nearest.min(here.distance_meters(there));
            }
        }
    }
    nearest
}

#[test]
fn a_walk_between_two_positions_agrees_with_the_footpath_the_file_stored() {
    open(
        &compile(reach(50_000)).expect("valid settings"),
        |consumer| {
            let mut checked = 0;
            for place in consumer.stops().iter().map(|stop| stop.index) {
                for walk in consumer.footpaths_by_place(place) {
                    let nearest = nearest_walk(consumer, place, walk.to_place_index);
                    assert!(
                        (nearest - f64::from(walk.meters.0)).abs() < 0.01,
                        "stored {} but computed {nearest}",
                        walk.meters.0
                    );
                    checked += 1;
                }
            }
            assert!(checked > 0, "the fixture has footpaths to compare");
        },
    );
}

#[test]
fn footpaths_out_of_order_are_rejected() {
    let bytes = common::damage_section(
        compile(reach(50_000)).expect("valid settings"),
        |header| header.footpaths,
        |footpaths: &mut [Footpath]| footpaths.reverse(),
    );

    assert!(common::is_rejected(&bytes), "nearest first is promised");
}
