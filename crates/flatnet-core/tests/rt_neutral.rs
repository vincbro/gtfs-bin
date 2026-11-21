//! Realtime updates that no wire format sent: the `Feed` a source-neutral decoder would make.

mod common;

use flatnet_core::{
    consumer::Consumer,
    models::{Date, Delay},
    rt::{
        CallUpdate, Event, Feed, Kinds, Realtime, RealtimeBuilder, StopTimeStatus, TripRef,
        TripStatus, TripUpdate, Update,
    },
};

fn with_consumer(body: impl FnOnce(&Consumer<'_>)) {
    let bytes = common::compile_fixture();
    let (_file, mmap) = common::map(&bytes).expect("the fixture maps");
    body(&Consumer::new(&mmap).expect("the fixture is valid"));
}

fn trip_ref(trip_id: &str) -> TripRef {
    TripRef {
        trip_id: Some(trip_id.to_owned()),
        ..TripRef::default()
    }
}

#[test]
fn a_hand_built_feed_delays_a_stop_time() {
    with_consumer(|consumer| {
        let late = Event {
            delay: Some(120),
            time: None,
        };
        let feed = Feed {
            updates: vec![Update::Trip(TripUpdate {
                trip: trip_ref("trip4"),
                status: TripStatus::Scheduled,
                unsupported: false,
                delay: None,
                timestamp: None,
                calls: vec![CallUpdate {
                    sequence: None,
                    stop_id: Some("stop9".to_owned()),
                    arrival: Some(late),
                    departure: Some(late),
                    status: StopTimeStatus::Scheduled,
                    occupancy: None,
                }],
            })],
            ..Feed::default()
        };

        let (realtime, report) = RealtimeBuilder::new(consumer).build_feeds_with_report([feed]);

        let first = consumer.trip_by_id("trip4").expect("the trip exists");
        let slot = first.stop_times.start as usize + 1;
        assert_eq!(
            realtime.stop_time_arrival_delays()[slot].as_option(),
            Some(Delay(120)),
            "stop9 is the second stop of trip4"
        );
        assert_eq!(report.trip_updates, 1, "the trip update was applied");
    });
}

#[test]
fn a_full_dataset_forgets_the_updates_it_leaves_out() {
    with_consumer(|consumer| {
        let builder = RealtimeBuilder::new(consumer);
        let delayed = Feed {
            updates: vec![Update::Trip(TripUpdate {
                trip: trip_ref("trip4"),
                status: TripStatus::Canceled,
                unsupported: false,
                delay: None,
                timestamp: None,
                calls: Vec::new(),
            })],
            ..Feed::default()
        };
        let mut realtime = Realtime::new(consumer);
        builder.apply_feeds([delayed], &mut realtime);
        let index = consumer.trip_by_id("trip4").expect("the trip exists").index;
        assert!(realtime.is_trip_canceled(index), "the update was applied");

        let nothing = Feed {
            replaces: Kinds {
                trip_updates: true,
                ..Kinds::default()
            },
            ..Feed::default()
        };
        builder.apply_feeds([nothing], &mut realtime);

        assert!(
            !realtime.is_trip_canceled(index),
            "a full dataset with no trip updates clears them"
        );
    });
}

#[test]
fn an_added_trip_is_found_by_the_stops_it_calls_at() {
    with_consumer(|consumer| {
        let stop = consumer.stop_by_id("stop9").expect("the stop exists").index;
        let other = consumer.stop_by_id("stop1").expect("the stop exists").index;
        let feed = Feed {
            updates: vec![Update::Trip(TripUpdate {
                trip: trip_ref("extra"),
                status: TripStatus::Added,
                unsupported: false,
                delay: None,
                timestamp: None,
                calls: vec![CallUpdate {
                    sequence: None,
                    stop_id: Some("stop9".to_owned()),
                    arrival: None,
                    departure: None,
                    status: StopTimeStatus::Scheduled,
                    occupancy: None,
                }],
            })],
            ..Feed::default()
        };

        let realtime = RealtimeBuilder::new(consumer).build_feeds([feed]);

        assert_eq!(
            realtime.added_trips_at_stop(stop).count(),
            1,
            "the added trip calls at stop9"
        );
        assert_eq!(
            realtime.added_trips_at_stop(other).count(),
            0,
            "and not at another stop"
        );
    });
}

#[test]
fn a_service_date_past_year_9999_is_ignored_not_a_panic() {
    with_consumer(|consumer| {
        for days in [2_932_897, 3_000_000, u32::MAX] {
            let feed = Feed {
                updates: vec![Update::Trip(TripUpdate {
                    trip: TripRef {
                        start_date: Some(Date(days)),
                        ..trip_ref("trip4")
                    },
                    status: TripStatus::Scheduled,
                    unsupported: false,
                    delay: None,
                    timestamp: None,
                    calls: vec![CallUpdate {
                        sequence: Some(1),
                        stop_id: None,
                        arrival: Some(Event {
                            delay: None,
                            time: Some(1_483_000_000),
                        }),
                        departure: None,
                        status: StopTimeStatus::Scheduled,
                        occupancy: None,
                    }],
                })],
                ..Feed::default()
            };

            let (_, report) = RealtimeBuilder::new(consumer).build_feeds_with_report([feed]);

            assert_eq!(report.trip_updates, 1, "{days} still updates the trip");
        }
    });
}
