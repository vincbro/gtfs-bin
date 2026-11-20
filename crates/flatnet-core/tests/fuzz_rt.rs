//! Property tests that feed the realtime builder arbitrary feeds.

mod common;

use flatnet_core::{
    consumer::Consumer,
    models::Date,
    rt::{Realtime, RealtimeBuilder},
};
use gtfs_rt::{
    Alert, EntitySelector, FeedEntity, FeedHeader, FeedMessage, Position, TimeRange,
    TripDescriptor, TripUpdate, VehiclePosition,
    trip_update::{StopTimeEvent, StopTimeUpdate},
};
use proptest::{collection, option, prelude::*};

const TRIPS: [&str; 8] = [
    "trip1", "trip2", "trip3", "trip4", "trip5", "trip6", "trip7", "ghost",
];
const STOPS: [&str; 6] = ["stop1", "stop2", "stop6", "stop8", "stop15", "nowhere"];
const ROUTES: [&str; 5] = ["route1", "route2", "route3", "route4", "ghost"];

fn name(options: &'static [&'static str]) -> impl Strategy<Value = String> {
    (0..options.len()).prop_map(move |index| options[index].to_owned())
}

fn event() -> impl Strategy<Value = StopTimeEvent> {
    (
        option::of(any::<i32>()),
        option::of(prop_oneof![
            any::<i64>(),
            Just(i64::MIN),
            Just(i64::MAX),
            Just(0),
            1_483_000_000_i64..1_484_000_000,
        ]),
    )
        .prop_map(|(delay, time)| StopTimeEvent {
            delay,
            time,
            ..Default::default()
        })
}

fn stop_time_update() -> impl Strategy<Value = StopTimeUpdate> {
    (
        option::of(0_u32..5),
        option::of(name(&STOPS)),
        option::of(event()),
        option::of(event()),
        option::of(0_i32..5),
    )
        .prop_map(
            |(stop_sequence, stop_id, arrival, departure, schedule_relationship)| StopTimeUpdate {
                stop_sequence,
                stop_id,
                arrival,
                departure,
                schedule_relationship,
                ..Default::default()
            },
        )
}

fn descriptor() -> impl Strategy<Value = TripDescriptor> {
    (
        option::of(name(&TRIPS)),
        option::of(name(&ROUTES)),
        option::of(prop_oneof![
            Just("09:00:00".to_owned()),
            Just("14:00:00".to_owned()),
            "[0-9:]{0,10}",
        ]),
        option::of(prop_oneof![
            Just("20170107".to_owned()),
            Just("20170108".to_owned()),
            "[0-9]{0,9}",
        ]),
        option::of(0_i32..9),
    )
        .prop_map(
            |(trip_id, route_id, start_time, start_date, schedule_relationship)| TripDescriptor {
                trip_id,
                route_id,
                start_time,
                start_date,
                schedule_relationship,
                ..Default::default()
            },
        )
}

fn entity() -> impl Strategy<Value = FeedEntity> {
    let trip_update = (
        descriptor(),
        collection::vec(stop_time_update(), 0..5),
        option::of(any::<i32>()),
        option::of(0_u64..5000),
    )
        .prop_map(|(trip, stop_time_update, delay, timestamp)| TripUpdate {
            trip,
            stop_time_update,
            delay,
            timestamp,
            ..Default::default()
        });
    let vehicle = (
        option::of(descriptor()),
        option::of((any::<f32>(), any::<f32>())),
        option::of(name(&STOPS)),
        option::of(0_u64..5000),
    )
        .prop_map(|(trip, position, stop_id, timestamp)| VehiclePosition {
            trip,
            position: position.map(|(latitude, longitude)| Position {
                latitude,
                longitude,
                ..Default::default()
            }),
            stop_id,
            timestamp,
            ..Default::default()
        });
    let alert = (
        collection::vec(
            (
                option::of(name(&STOPS)),
                option::of(name(&ROUTES)),
                option::of(any::<i32>()),
            ),
            0..3,
        ),
        collection::vec((option::of(any::<u64>()), option::of(any::<u64>())), 0..3),
    )
        .prop_map(|(selectors, periods)| Alert {
            informed_entity: selectors
                .into_iter()
                .map(|(stop_id, route_id, route_type)| EntitySelector {
                    stop_id,
                    route_id,
                    route_type,
                    ..Default::default()
                })
                .collect(),
            active_period: periods
                .into_iter()
                .map(|(start, end)| TimeRange { start, end })
                .collect(),
            ..Default::default()
        });

    (
        "[a-c]",
        option::of(trip_update),
        option::of(vehicle),
        option::of(alert),
        option::of(any::<bool>()),
    )
        .prop_map(|(id, trip_update, vehicle, alert, is_deleted)| FeedEntity {
            id,
            trip_update,
            vehicle,
            alert,
            is_deleted,
            ..Default::default()
        })
}

fn message() -> impl Strategy<Value = FeedMessage> {
    (
        collection::vec(entity(), 0..6),
        option::of(0_i32..2),
        option::of(0_u64..5000),
    )
        .prop_map(|(entity, incrementality, timestamp)| FeedMessage {
            header: FeedHeader {
                gtfs_realtime_version: "2.0".to_owned(),
                incrementality,
                timestamp,
            },
            entity,
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn arbitrary_feeds_never_panic_or_resize_the_state(
        messages in collection::vec(message(), 1..4),
        cascade in any::<bool>(),
        replace in any::<bool>(),
        offset in -200_000_i32..200_000,
        for_day in any::<bool>(),
    ) {
        let bytes = common::compile_fixture();
        let (_file, mmap) = common::map(&bytes).expect("the fixture maps");
        let consumer = Consumer::new(&mmap).expect("the fixture is valid");
        let mut realtime = if for_day {
            Realtime::for_service_date(&consumer, Date::parse_gtfs("20170107").expect("a valid date"))
        } else {
            Realtime::new(&consumer)
        };
        let builder = RealtimeBuilder::new(&consumer)
            .with_cascading_delays(cascade)
            .with_replace_on_full_dataset(replace)
            .with_utc_offset_seconds(offset);

        for message in messages {
            let _ = builder.apply([message].into_iter(), &mut realtime);
        }

        prop_assert_eq!(realtime.stop_time_status().len(), consumer.stop_times().len());
        prop_assert_eq!(realtime.trip_status().len(), consumer.trips().len());
        for stop_time in consumer.stop_times() {
            let _ = realtime.arrival_time(stop_time);
            let _ = realtime.departure_time(stop_time);
            let _ = realtime.is_stop_time_skipped(stop_time);
        }
        for alert in realtime.alerts() {
            let _ = alert.is_active_at(100);
        }
        let _ = realtime.active_alerts(1000).count();
    }
}
