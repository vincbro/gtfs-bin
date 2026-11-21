//! Property tests that feed the realtime builder the feeds a source-neutral decoder could make,
//! with values no GTFS-realtime message holds: any date, any time zone, any timestamp.

mod common;

use flatnet_core::{
    consumer::Consumer,
    models::{Date, StopTimeIndex, Time},
    rt::{
        ActivePeriod, AlertUpdate, CallUpdate, Cause, Effect, Event, Feed, Kinds, Occupancy,
        Position, Realtime, RealtimeBuilder, Removal, Selector, Severity, StopTimeStatus, Text,
        Translation, TripRef, TripStatus, TripUpdate, Update, VehicleStopStatus, VehicleUpdate,
    },
};
use proptest::{collection, option, prelude::*};

const TRIPS: [&str; 8] = [
    "trip1", "trip2", "trip3", "trip4", "trip5", "trip6", "trip7", "ghost",
];
const STOPS: [&str; 6] = ["stop1", "stop2", "stop6", "stop8", "stop15", "nowhere"];
const ROUTES: [&str; 5] = ["route1", "route2", "route3", "route4", "ghost"];
const ZONES: [&str; 4] = [
    "Europe/Paris",
    "Pacific/Auckland",
    "America/St_Johns",
    "Asia/Kolkata",
];

fn name(options: &'static [&'static str]) -> impl Strategy<Value = String> {
    (0..options.len()).prop_map(move |index| options[index].to_owned())
}

fn date() -> impl Strategy<Value = Date> {
    prop_oneof![
        Just(Date(17_172)),
        Just(Date(17_173)),
        Just(Date(0)),
        Just(Date(2_932_896)),
        Just(Date(2_932_897)),
        Just(Date(u32::MAX)),
        any::<u32>().prop_map(Date),
    ]
}

fn seconds() -> impl Strategy<Value = i64> {
    prop_oneof![
        any::<i64>(),
        Just(i64::MIN),
        Just(i64::MAX),
        Just(0),
        1_483_000_000_i64..1_484_000_000,
    ]
}

fn event() -> impl Strategy<Value = Event> {
    (option::of(any::<i32>()), option::of(seconds()))
        .prop_map(|(delay, time)| Event { delay, time })
}

fn status() -> impl Strategy<Value = StopTimeStatus> {
    prop_oneof![
        Just(StopTimeStatus::Unchanged),
        Just(StopTimeStatus::Scheduled),
        Just(StopTimeStatus::Skipped),
        Just(StopTimeStatus::NoData),
        Just(StopTimeStatus::Unscheduled),
    ]
}

fn trip_status() -> impl Strategy<Value = TripStatus> {
    prop_oneof![
        Just(TripStatus::Unchanged),
        Just(TripStatus::Scheduled),
        Just(TripStatus::Added),
        Just(TripStatus::Canceled),
        Just(TripStatus::Deleted),
        Just(TripStatus::Unscheduled),
    ]
}

fn trip_ref() -> impl Strategy<Value = TripRef> {
    (
        option::of(name(&TRIPS)),
        option::of(name(&ROUTES)),
        option::of(prop_oneof![
            Just(Time(32_400)),
            Just(Time(50_400)),
            any::<u32>().prop_map(Time)
        ]),
        option::of(date()),
        option::of(0_u32..3),
    )
        .prop_map(
            |(trip_id, route_id, start_time, start_date, direction)| TripRef {
                trip_id,
                route_id,
                start_time,
                start_date,
                direction,
            },
        )
}

fn call() -> impl Strategy<Value = CallUpdate> {
    (
        option::of(prop_oneof![0_u32..6, any::<u32>()]),
        option::of(name(&STOPS)),
        option::of(event()),
        option::of(event()),
        status(),
    )
        .prop_map(
            |(sequence, stop_id, arrival, departure, status)| CallUpdate {
                sequence,
                stop_id,
                arrival,
                departure,
                status,
                occupancy: None,
            },
        )
}

fn trip_update() -> impl Strategy<Value = TripUpdate> {
    (
        trip_ref(),
        trip_status(),
        any::<bool>(),
        option::of(any::<i32>()),
        option::of(prop_oneof![0_u64..5000, any::<u64>()]),
        collection::vec(call(), 0..5),
    )
        .prop_map(
            |(trip, status, unsupported, delay, timestamp, calls)| TripUpdate {
                trip,
                status,
                unsupported,
                delay,
                timestamp,
                calls,
            },
        )
}

fn vehicle_update() -> impl Strategy<Value = VehicleUpdate> {
    let coordinate = prop_oneof![
        any::<f64>(),
        Just(f64::NAN),
        Just(f64::INFINITY),
        -90.0..90.0
    ];
    (
        option::of(trip_ref()),
        option::of((
            coordinate.clone(),
            coordinate,
            option::of(any::<f32>()),
            option::of(any::<f32>()),
        )),
        option::of(name(&STOPS)),
        option::of(prop_oneof![0_u64..5000, any::<u64>()]),
    )
        .prop_map(|(trip, position, stop_id, timestamp)| VehicleUpdate {
            trip,
            position: position.map(|(latitude, longitude, bearing, speed)| Position {
                latitude,
                longitude,
                bearing,
                speed,
            }),
            status: VehicleStopStatus::default(),
            occupancy: Occupancy::default(),
            occupancy_percentage: None,
            current_stop_sequence: None,
            stop_id,
            timestamp,
            description: None,
        })
}

fn alert_update() -> impl Strategy<Value = AlertUpdate> {
    let selector = (
        option::of(name(&STOPS)),
        option::of(name(&ROUTES)),
        option::of(name(&TRIPS)),
        option::of(any::<u16>()),
    )
        .prop_map(|(stop_id, route_id, trip_id, route_type)| Selector {
            agency_id: None,
            route_id,
            route_type,
            trip_id,
            stop_id,
        });
    let text = option::of(collection::vec(
        (
            option::of(prop_oneof![Just("sv".to_owned()), Just("en".to_owned())]),
            "[a-z ]{0,6}",
        ),
        0..3,
    ))
    .prop_map(|text| {
        text.map(|translations| Text {
            translations: translations
                .into_iter()
                .map(|(language, text)| Translation { language, text })
                .collect(),
        })
    });
    (
        "[a-c]",
        collection::vec((option::of(any::<u64>()), option::of(any::<u64>())), 0..3),
        collection::vec(selector, 0..3),
        text,
    )
        .prop_map(|(id, periods, selectors, header)| AlertUpdate {
            id,
            active_periods: periods
                .into_iter()
                .map(|(start, end)| ActivePeriod { start, end })
                .collect(),
            selectors,
            cause: Cause::default(),
            effect: Effect::default(),
            severity: Severity::default(),
            header,
            description: None,
            url: None,
        })
}

fn update() -> impl Strategy<Value = Update> {
    prop_oneof![
        4 => trip_update().prop_map(Update::Trip),
        2 => vehicle_update().prop_map(Update::Vehicle),
        2 => alert_update().prop_map(Update::Alert),
        1 => ("[a-c]", option::of(trip_ref()), option::of(trip_ref()))
            .prop_map(|(id, trip, vehicle_trip)| Update::Remove(Removal { id, trip, vehicle_trip })),
        1 => Just(Update::Unsupported),
    ]
}

fn feed() -> impl Strategy<Value = Feed> {
    (
        prop_oneof![0_u64..5000, any::<u64>()],
        any::<[bool; 3]>(),
        collection::vec(update(), 0..6),
    )
        .prop_map(
            |(timestamp, [trip_updates, vehicles, alerts], updates)| Feed {
                timestamp,
                replaces: Kinds {
                    trip_updates,
                    vehicles,
                    alerts,
                },
                updates,
            },
        )
}

proptest! {

    #[test]
    fn arbitrary_feeds_never_panic_or_resize_the_state(
        feeds in collection::vec(feed(), 1..4),
        cascade in any::<bool>(),
        replace in any::<bool>(),
        zone in prop_oneof![Just(None), Just(Some(None)), (0..ZONES.len()).prop_map(|index| Some(Some(ZONES[index])))],
        offset in prop_oneof![-200_000_i32..200_000, any::<i32>()],
        service_date in option::of(date()),
    ) {
        let bytes = common::compile_fixture();
        let (_file, mmap) = common::map(&bytes).expect("the fixture maps");
        let consumer = Consumer::new(&mmap).expect("the fixture is valid");
        let mut realtime = service_date.map_or_else(
            || Realtime::new(&consumer),
            |date| Realtime::for_service_date(&consumer, date),
        );
        let mut builder = RealtimeBuilder::new(&consumer)
            .with_cascading_delays(cascade)
            .with_replace_on_full_dataset(replace)
            .with_preferred_language("sv");
        builder = match zone {
            // The zone the file holds.
            None => builder,
            Some(None) => builder.with_utc_offset_seconds(offset),
            Some(Some(zone)) => builder.with_time_zone(zone).expect("a zone the system knows"),
        };

        for feed in feeds {
            let _ = builder.apply_feeds([feed], &mut realtime);
        }

        prop_assert_eq!(realtime.stop_time_status().len(), consumer.timings().len());
        prop_assert_eq!(realtime.trip_status().len(), consumer.trips().len());
        for trip in consumer.trips() {
            let first = trip.stop_times.start as usize;
            for (position, timing) in consumer.timings_by_trip(trip.index).iter().enumerate() {
                let index = StopTimeIndex::from(first + position);
                let _ = realtime.arrival_time(trip.index, index, *timing);
                let _ = realtime.departure_time(trip.index, index, *timing);
                let _ = realtime.is_stop_time_skipped(trip.index, index);
            }
        }
        for alert in realtime.alerts() {
            let _ = alert.is_active_at(100);
        }
        let _ = realtime.active_alerts(1000).count();
    }
}
