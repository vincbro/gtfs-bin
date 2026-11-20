//! Realtime behavior beyond the basic delay and cancellation case.

mod common;

use flatnet_core::{
    consumer::Consumer,
    models::{Date, Delay, Sentinel, TripIndex},
    rt::{Realtime, RealtimeBuilder, StopTimeStatus, TripStatus},
};
use gtfs_rt::{
    FeedEntity, FeedHeader, FeedMessage, TripDescriptor, TripUpdate, VehiclePosition,
    alert::{Cause, Effect},
    feed_header::Incrementality,
    translated_string::Translation,
    trip_descriptor::ScheduleRelationship as TripRelationship,
    trip_update::{StopTimeEvent, StopTimeUpdate, stop_time_update::ScheduleRelationship},
    vehicle_position::VehicleStopStatus,
};

const SATURDAY: &str = "20170107";

fn saturday() -> Date {
    Date::parse_gtfs(SATURDAY).expect("a valid date")
}

fn message(entities: Vec<FeedEntity>) -> FeedMessage {
    FeedMessage {
        header: FeedHeader {
            gtfs_realtime_version: "2.0".to_owned(),
            ..Default::default()
        },
        entity: entities,
    }
}

fn entity(id: &str) -> FeedEntity {
    FeedEntity {
        id: id.to_owned(),
        ..Default::default()
    }
}

fn descriptor(trip_id: &str) -> TripDescriptor {
    TripDescriptor {
        trip_id: Some(trip_id.to_owned()),
        ..Default::default()
    }
}

fn delay_event(delay: i32) -> StopTimeEvent {
    StopTimeEvent {
        delay: Some(delay),
        ..Default::default()
    }
}

fn stop_update(sequence: u32, delay: i32) -> StopTimeUpdate {
    StopTimeUpdate {
        stop_sequence: Some(sequence),
        arrival: Some(delay_event(delay)),
        departure: Some(delay_event(delay)),
        ..Default::default()
    }
}

fn trip_update(trip: TripDescriptor, updates: Vec<StopTimeUpdate>) -> FeedEntity {
    FeedEntity {
        trip_update: Some(TripUpdate {
            trip,
            stop_time_update: updates,
            ..Default::default()
        }),
        ..entity("trip-update")
    }
}

fn first_slot(consumer: &Consumer<'_>, trip_id: &str) -> usize {
    consumer
        .trip_by_id(trip_id)
        .expect("the fixture has the trip")
        .stop_times
        .start as usize
}

fn trip_index(consumer: &Consumer<'_>, trip_id: &str) -> TripIndex {
    consumer
        .trip_by_id(trip_id)
        .expect("the fixture has the trip")
        .index
}

/// Runs a test body against the compiled fixture.
fn with_consumer(body: impl FnOnce(&Consumer<'_>)) {
    let bytes = common::compile_fixture();
    let (_file, mmap) = common::map(&bytes).expect("the fixture maps");
    body(&Consumer::new(&mmap).expect("the fixture is valid"));
}

#[test]
fn stop_times_are_matched_by_stop_id_when_there_is_no_sequence() {
    with_consumer(|consumer| {
        let update = StopTimeUpdate {
            stop_id: Some("stop9".to_owned()),
            arrival: Some(delay_event(90)),
            departure: Some(delay_event(90)),
            ..Default::default()
        };

        let (realtime, report) = RealtimeBuilder::new(consumer).build_with_report(
            [message(vec![trip_update(
                descriptor("trip4"),
                vec![update],
            )])]
            .into_iter(),
        );

        let slot = first_slot(consumer, "trip4") + 1;
        assert_eq!(
            realtime.stop_time_arrival_delays()[slot].as_option(),
            Some(Delay(90)),
            "stop9 is the second stop of trip4"
        );
        assert_eq!(report.stop_time_updates, 1, "the update was applied");
    });
}

#[test]
fn an_unknown_stop_is_counted_and_not_applied() {
    with_consumer(|consumer| {
        let update = StopTimeUpdate {
            stop_id: Some("nowhere".to_owned()),
            arrival: Some(delay_event(90)),
            ..Default::default()
        };

        let (_, report) = RealtimeBuilder::new(consumer).build_with_report(
            [message(vec![trip_update(
                descriptor("trip4"),
                vec![update],
            )])]
            .into_iter(),
        );

        assert_eq!(
            report.stop_time_updates_unmatched, 1,
            "no stop time matches it"
        );
        assert_eq!(report.stop_time_updates, 0, "nothing was applied");
    });
}

#[test]
fn a_trip_is_found_by_route_and_start_time_without_an_id() {
    with_consumer(|consumer| {
        let by_start = TripDescriptor {
            route_id: Some("route1".to_owned()),
            start_time: Some("14:00:00".to_owned()),
            schedule_relationship: Some(TripRelationship::Canceled as i32),
            ..Default::default()
        };

        let realtime = RealtimeBuilder::new(consumer)
            .build([message(vec![trip_update(by_start, vec![])])].into_iter());

        assert_eq!(
            realtime.trip_status()[trip_index(consumer, "trip1").as_usize()],
            TripStatus::Canceled,
            "trip1 is the only route1 trip starting at 14:00"
        );
    });
}

#[test]
fn a_start_time_that_fits_two_trips_is_ambiguous() {
    with_consumer(|consumer| {
        let ambiguous = TripDescriptor {
            route_id: Some("route2".to_owned()),
            start_time: Some("09:00:00".to_owned()),
            schedule_relationship: Some(TripRelationship::Canceled as i32),
            ..Default::default()
        };

        let (realtime, report) = RealtimeBuilder::new(consumer)
            .build_with_report([message(vec![trip_update(ambiguous, vec![])])].into_iter());

        assert_eq!(
            report.trip_updates_ambiguous, 1,
            "trip4 and trip7 both start at 09:00"
        );
        assert!(
            realtime
                .trip_status()
                .iter()
                .all(|&status| status == TripStatus::Unchanged),
            "neither trip is touched"
        );
    });
}

#[test]
fn updates_for_another_service_date_are_ignored() {
    with_consumer(|consumer| {
        let mut other_day = descriptor("trip4");
        other_day.start_date = Some("20170108".to_owned());
        other_day.schedule_relationship = Some(TripRelationship::Canceled as i32);
        let mut realtime = Realtime::for_service_date(consumer, saturday());

        let report = RealtimeBuilder::new(consumer).apply(
            [message(vec![trip_update(other_day, vec![])])].into_iter(),
            &mut realtime,
        );

        assert_eq!(report.wrong_service_date, 1, "the update names Sunday");
        assert_eq!(
            realtime.trip_status()[trip_index(consumer, "trip4").as_usize()],
            TripStatus::Unchanged,
            "a Saturday state is not changed by a Sunday update"
        );
    });
}

#[test]
fn a_timestamp_gives_an_exact_delay_past_midnight() {
    with_consumer(|consumer| {
        // trip4 leaves stop8 at 09:00:00. The vehicle leaves at 09:02:30 in a zone 2 hours ahead
        // of UTC, which is 07:02:30 UTC.
        let midnight_utc = i64::from(saturday().0) * 86_400;
        let update = StopTimeUpdate {
            stop_sequence: Some(0),
            departure: Some(StopTimeEvent {
                time: Some(midnight_utc + 7 * 3600 + 150),
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut trip = descriptor("trip4");
        trip.start_date = Some(SATURDAY.to_owned());

        let realtime = RealtimeBuilder::new(consumer)
            .with_utc_offset_seconds(2 * 3600)
            .build([message(vec![trip_update(trip, vec![update])])].into_iter());

        assert_eq!(
            realtime.stop_time_departure_delays()[first_slot(consumer, "trip4")].as_option(),
            Some(Delay(150)),
            "two and a half minutes late"
        );
    });
}

#[test]
fn a_timestamp_without_a_date_wraps_to_the_nearest_day() {
    with_consumer(|consumer| {
        // trip4 leaves at 09:00:00, and the vehicle leaves at 08:59:00 UTC the next day.
        let next_day = 86_400 * 20_000 + 8 * 3600 + 59 * 60;
        let update = StopTimeUpdate {
            stop_sequence: Some(0),
            departure: Some(StopTimeEvent {
                time: Some(next_day),
                ..Default::default()
            }),
            ..Default::default()
        };

        let realtime = RealtimeBuilder::new(consumer)
            .with_utc_offset_seconds(0)
            .build(
                [message(vec![trip_update(
                    descriptor("trip4"),
                    vec![update],
                )])]
                .into_iter(),
            );

        assert_eq!(
            realtime.stop_time_departure_delays()[first_slot(consumer, "trip4")].as_option(),
            Some(Delay(-60)),
            "one minute early, not a day off"
        );
    });
}

#[test]
fn a_new_update_replaces_the_old_stop_delays_of_the_trip() {
    with_consumer(|consumer| {
        let mut realtime = Realtime::new(consumer);
        let builder = RealtimeBuilder::new(consumer).with_cascading_delays(false);

        builder.apply(
            [message(vec![trip_update(
                descriptor("trip4"),
                vec![stop_update(0, 60)],
            )])]
            .into_iter(),
            &mut realtime,
        );
        builder.apply(
            [message(vec![trip_update(
                descriptor("trip4"),
                vec![stop_update(2, 30)],
            )])]
            .into_iter(),
            &mut realtime,
        );

        let slot = first_slot(consumer, "trip4");
        assert!(
            realtime.stop_time_departure_delays()[slot].is_none(),
            "the first update's delay is gone, because the trip update is the whole truth"
        );
        assert_eq!(
            realtime.stop_time_departure_delays()[slot + 2].as_option(),
            Some(Delay(30)),
            "the second update is applied"
        );
    });
}

#[test]
fn an_older_trip_update_is_ignored() {
    with_consumer(|consumer| {
        let mut realtime = Realtime::new(consumer);
        // A full dataset would clear the first update, so the second feed must not replace it.
        let builder = RealtimeBuilder::new(consumer).with_replace_on_full_dataset(false);
        let at = |timestamp: u64, delay: i32| {
            let mut entity = trip_update(descriptor("trip4"), vec![stop_update(0, delay)]);
            if let Some(update) = entity.trip_update.as_mut() {
                update.timestamp = Some(timestamp);
            }
            message(vec![entity])
        };

        builder.apply([at(2000, 120)].into_iter(), &mut realtime);
        let report = builder.apply([at(1000, 999)].into_iter(), &mut realtime);

        assert_eq!(report.trip_updates_stale, 1, "the second feed is older");
        assert_eq!(
            realtime.stop_time_departure_delays()[first_slot(consumer, "trip4")].as_option(),
            Some(Delay(120)),
            "the newer delay stays"
        );
    });
}

#[test]
fn skipped_stops_pass_the_delay_on_and_no_data_ends_it() {
    with_consumer(|consumer| {
        let skipped = StopTimeUpdate {
            stop_sequence: Some(1),
            schedule_relationship: Some(ScheduleRelationship::Skipped as i32),
            ..Default::default()
        };
        let no_data = StopTimeUpdate {
            stop_sequence: Some(1),
            schedule_relationship: Some(ScheduleRelationship::NoData as i32),
            ..Default::default()
        };
        let slot = first_slot(consumer, "trip4");

        let after_skip = RealtimeBuilder::new(consumer).build(
            [message(vec![trip_update(
                descriptor("trip4"),
                vec![stop_update(0, 60), skipped],
            )])]
            .into_iter(),
        );
        let after_no_data = RealtimeBuilder::new(consumer).build(
            [message(vec![trip_update(
                descriptor("trip4"),
                vec![stop_update(0, 60), no_data],
            )])]
            .into_iter(),
        );

        assert_eq!(
            after_skip.stop_time_status()[slot + 1],
            StopTimeStatus::Skipped,
            "the middle stop is skipped"
        );
        assert_eq!(
            after_skip.stop_time_departure_delays()[slot + 2].as_option(),
            Some(Delay(60)),
            "the last stop still carries the first stop's delay"
        );
        assert!(
            after_no_data.stop_time_departure_delays()[slot + 2].is_none(),
            "no data for a stop means nothing is known after it"
        );
    });
}

#[test]
fn a_trip_level_delay_spreads_when_there_are_no_stop_updates() {
    with_consumer(|consumer| {
        let mut entity = trip_update(descriptor("trip4"), vec![]);
        if let Some(update) = entity.trip_update.as_mut() {
            update.delay = Some(300);
        }

        let realtime = RealtimeBuilder::new(consumer).build([message(vec![entity])].into_iter());

        let slot = first_slot(consumer, "trip4");
        for offset in 0..3 {
            assert_eq!(
                realtime.stop_time_arrival_delays()[slot + offset].as_option(),
                Some(Delay(300)),
                "stop {offset} takes the trip delay"
            );
        }
    });
}

#[test]
fn a_missing_trip_delay_is_not_zero() {
    with_consumer(|consumer| {
        let realtime = RealtimeBuilder::new(consumer).build(
            [message(vec![trip_update(
                descriptor("trip4"),
                vec![stop_update(0, 5)],
            )])]
            .into_iter(),
        );

        assert!(
            realtime.trip_delay()[trip_index(consumer, "trip4").as_usize()].is_none(),
            "the feed gave no trip delay, which is not the same as on time"
        );
    });
}

#[test]
fn a_departure_inherits_the_arrival_delay_when_the_feed_gives_none() {
    with_consumer(|consumer| {
        let update = StopTimeUpdate {
            stop_sequence: Some(1),
            arrival: Some(delay_event(45)),
            ..Default::default()
        };

        let realtime = RealtimeBuilder::new(consumer)
            .with_cascading_delays(false)
            .build(
                [message(vec![trip_update(
                    descriptor("trip4"),
                    vec![update],
                )])]
                .into_iter(),
            );

        assert_eq!(
            realtime.stop_time_departure_delays()[first_slot(consumer, "trip4") + 1].as_option(),
            Some(Delay(45)),
            "a vehicle that arrives late leaves late"
        );
    });
}

#[test]
fn effective_times_apply_the_delay_and_hide_skipped_stops() {
    with_consumer(|consumer| {
        let skipped = StopTimeUpdate {
            stop_sequence: Some(1),
            schedule_relationship: Some(ScheduleRelationship::Skipped as i32),
            ..Default::default()
        };
        let realtime = RealtimeBuilder::new(consumer).build(
            [message(vec![trip_update(
                descriptor("trip4"),
                vec![stop_update(0, 120), skipped],
            )])]
            .into_iter(),
        );
        let stop_times = consumer.stop_times_by_trip(trip_index(consumer, "trip4"));

        assert_eq!(
            realtime
                .departure_time(&stop_times[0])
                .map(|time| time.to_string()),
            Some("09:02:00".to_owned()),
            "scheduled 09:00 plus two minutes"
        );
        assert_eq!(
            realtime.departure_time(&stop_times[1]),
            None,
            "a skipped stop has no time"
        );
    });
}

#[test]
fn a_new_trip_from_the_feed_is_kept_beside_the_static_ones() {
    with_consumer(|consumer| {
        let midnight_utc = i64::from(saturday().0) * 86_400;
        let mut trip = descriptor("extra-1");
        trip.route_id = Some("route1".to_owned());
        trip.start_date = Some(SATURDAY.to_owned());
        trip.schedule_relationship = Some(TripRelationship::Added as i32);
        let at = |stop: &str, seconds: i64| StopTimeUpdate {
            stop_id: Some(stop.to_owned()),
            arrival: Some(StopTimeEvent {
                time: Some(midnight_utc + seconds),
                ..Default::default()
            }),
            ..Default::default()
        };

        let (realtime, report) = RealtimeBuilder::new(consumer)
            .with_utc_offset_seconds(0)
            .build_with_report(
                [message(vec![trip_update(
                    trip,
                    vec![at("stop2", 10 * 3600), at("stop3", 10 * 3600 + 600)],
                )])]
                .into_iter(),
            );

        assert_eq!(report.added_trips, 1, "the trip is new");
        let added = &realtime.added_trips()[0];
        assert_eq!(&*added.id, "extra-1", "it keeps its own id");
        assert_eq!(added.stop_times.len(), 2, "both stops resolved");
        assert_eq!(
            added.stop_times[1]
                .arrival_time
                .as_option()
                .map(|time| time.0),
            Some(10 * 3600 + 600),
            "times are seconds since the start of the service day"
        );
        assert!(added.route.is_some(), "route1 exists");
    });
}

#[test]
fn an_unknown_trip_is_counted() {
    with_consumer(|consumer| {
        let (_, report) = RealtimeBuilder::new(consumer).build_with_report(
            [message(vec![trip_update(
                descriptor("ghost"),
                vec![stop_update(0, 1)],
            )])]
            .into_iter(),
        );

        assert_eq!(
            report.trip_updates_unmatched, 1,
            "nothing in the static data has that id"
        );
    });
}

#[test]
fn a_full_dataset_replaces_only_the_kinds_it_contains() {
    with_consumer(|consumer| {
        let mut realtime = Realtime::new(consumer);
        let builder = RealtimeBuilder::new(consumer);
        builder.apply(
            [message(vec![trip_update(
                descriptor("trip4"),
                vec![stop_update(0, 60)],
            )])]
            .into_iter(),
            &mut realtime,
        );

        let vehicle = FeedEntity {
            vehicle: Some(VehiclePosition {
                trip: Some(descriptor("trip3")),
                position: Some(gtfs_rt::Position {
                    latitude: 59.3,
                    longitude: 18.0,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..entity("vehicle")
        };
        builder.apply([message(vec![vehicle])].into_iter(), &mut realtime);

        assert_eq!(
            realtime.stop_time_departure_delays()[first_slot(consumer, "trip4")].as_option(),
            Some(Delay(60)),
            "a vehicle feed leaves trip updates alone"
        );

        builder.apply(
            [message(vec![trip_update(descriptor("trip5"), vec![])])].into_iter(),
            &mut realtime,
        );

        assert!(
            realtime.stop_time_departure_delays()[first_slot(consumer, "trip4")].is_none(),
            "a trip update feed that no longer mentions trip4 drops its delays"
        );
        assert!(
            realtime.trip_vehicles()[trip_index(consumer, "trip3").as_usize()].is_some(),
            "and still leaves the vehicle alone"
        );
    });
}

#[test]
fn an_incremental_feed_keeps_what_it_does_not_mention() {
    with_consumer(|consumer| {
        let mut realtime = Realtime::new(consumer);
        let builder = RealtimeBuilder::new(consumer);
        builder.apply(
            [message(vec![trip_update(
                descriptor("trip4"),
                vec![stop_update(0, 60)],
            )])]
            .into_iter(),
            &mut realtime,
        );
        let mut incremental = message(vec![trip_update(descriptor("trip5"), vec![])]);
        incremental.header.incrementality = Some(Incrementality::Differential as i32);

        builder.apply([incremental].into_iter(), &mut realtime);

        assert_eq!(
            realtime.stop_time_departure_delays()[first_slot(consumer, "trip4")].as_option(),
            Some(Delay(60)),
            "a differential feed only changes what it names"
        );
    });
}

#[test]
fn a_deleted_entity_resets_the_trip() {
    with_consumer(|consumer| {
        let mut realtime = Realtime::new(consumer);
        let builder = RealtimeBuilder::new(consumer).with_replace_on_full_dataset(false);
        builder.apply(
            [message(vec![trip_update(
                descriptor("trip4"),
                vec![stop_update(0, 60)],
            )])]
            .into_iter(),
            &mut realtime,
        );
        let mut deleted = trip_update(descriptor("trip4"), vec![]);
        deleted.is_deleted = Some(true);

        let report = builder.apply([message(vec![deleted])].into_iter(), &mut realtime);

        assert_eq!(report.deleted, 1, "one entity was deleted");
        assert!(
            realtime.stop_time_departure_delays()[first_slot(consumer, "trip4")].is_none(),
            "the trip is back to its schedule"
        );
    });
}

#[test]
fn vehicles_are_stored_with_their_stop_and_motion() {
    with_consumer(|consumer| {
        let vehicle = FeedEntity {
            vehicle: Some(VehiclePosition {
                trip: Some(descriptor("trip3")),
                position: Some(gtfs_rt::Position {
                    latitude: 59.25,
                    longitude: 18.5,
                    bearing: Some(90.0),
                    speed: Some(12.5),
                    ..Default::default()
                }),
                stop_id: Some("stop7".to_owned()),
                current_status: Some(VehicleStopStatus::StoppedAt as i32),
                timestamp: Some(500),
                ..Default::default()
            }),
            ..entity("vehicle")
        };

        let (realtime, report) =
            RealtimeBuilder::new(consumer).build_with_report([message(vec![vehicle])].into_iter());

        let stored = realtime.trip_vehicles()[trip_index(consumer, "trip3").as_usize()]
            .expect("the vehicle was stored");
        assert_eq!(report.vehicles, 1, "one vehicle applied");
        assert_eq!(
            stored.position.to_f64_pair(),
            (59.25, 18.5),
            "position round trips"
        );
        assert_eq!(stored.bearing, Some(90.0), "bearing is kept");
        assert_eq!(stored.speed, Some(12.5), "speed is kept");
        assert_eq!(
            stored
                .current_stop
                .as_option()
                .map(|stop| consumer.stop_id(consumer.stop(stop).id)),
            Some("stop7"),
            "the stop id is resolved to a stop"
        );
        assert_eq!(
            stored.current_status,
            VehicleStopStatus::StoppedAt,
            "status is kept"
        );
    });
}

#[test]
fn a_vehicle_without_a_known_trip_is_counted() {
    with_consumer(|consumer| {
        let vehicle = FeedEntity {
            vehicle: Some(VehiclePosition {
                trip: Some(descriptor("ghost")),
                position: Some(gtfs_rt::Position::default()),
                ..Default::default()
            }),
            ..entity("vehicle")
        };

        let (_, report) =
            RealtimeBuilder::new(consumer).build_with_report([message(vec![vehicle])].into_iter());

        assert_eq!(report.vehicles_unmatched, 1, "the trip is unknown");
    });
}

fn text(value: &str, language: &str) -> gtfs_rt::TranslatedString {
    gtfs_rt::TranslatedString {
        translation: vec![Translation {
            text: value.to_owned(),
            language: Some(language.to_owned()),
        }],
    }
}

fn alert_entity(id: &str, stop_id: &str) -> FeedEntity {
    FeedEntity {
        alert: Some(gtfs_rt::Alert {
            active_period: vec![gtfs_rt::TimeRange {
                start: Some(100),
                end: Some(200),
            }],
            informed_entity: vec![gtfs_rt::EntitySelector {
                stop_id: Some(stop_id.to_owned()),
                ..Default::default()
            }],
            cause: Some(Cause::Construction as i32),
            effect: Some(Effect::Detour as i32),
            header_text: Some(text("Roadworks", "en")),
            ..Default::default()
        }),
        ..entity(id)
    }
}

#[test]
fn alerts_are_resolved_and_filtered_by_time() {
    with_consumer(|consumer| {
        let realtime = RealtimeBuilder::new(consumer)
            .build([message(vec![alert_entity("a1", "stop2")])].into_iter());

        let stop = consumer
            .stop_by_id("stop2")
            .expect("the fixture has stop2")
            .index;
        let other = consumer
            .stop_by_id("stop3")
            .expect("the fixture has stop3")
            .index;
        let alert = &realtime.alerts()[0];
        assert_eq!(&*alert.header, "Roadworks", "the text is kept");
        assert_eq!(alert.effect, Effect::Detour, "the effect is kept");
        assert!(alert.affects_stop(stop), "it is about stop2");
        assert!(!alert.affects_stop(other), "and not stop3");
        assert_eq!(
            realtime.active_alerts(150).count(),
            1,
            "active inside its period"
        );
        assert_eq!(
            realtime.active_alerts(200).count(),
            0,
            "the end is exclusive"
        );
        assert_eq!(
            realtime.active_alerts(50).count(),
            0,
            "not active before it starts"
        );
    });
}

#[test]
fn an_alert_about_unknown_stops_is_dropped_and_not_widened() {
    with_consumer(|consumer| {
        let (realtime, report) = RealtimeBuilder::new(consumer)
            .build_with_report([message(vec![alert_entity("a1", "nowhere")])].into_iter());

        assert!(realtime.alerts().is_empty(), "it is about another network");
        assert_eq!(report.alerts, 0, "so it isn't counted");
    });
}

#[test]
fn an_alert_with_the_same_id_replaces_the_old_one_and_can_be_deleted() {
    with_consumer(|consumer| {
        let mut realtime = Realtime::new(consumer);
        let builder = RealtimeBuilder::new(consumer).with_replace_on_full_dataset(false);
        builder.apply(
            [message(vec![alert_entity("a1", "stop2")])].into_iter(),
            &mut realtime,
        );
        builder.apply(
            [message(vec![alert_entity("a1", "stop3")])].into_iter(),
            &mut realtime,
        );
        assert_eq!(realtime.alerts().len(), 1, "same id, one alert");

        let mut deleted = alert_entity("a1", "stop3");
        deleted.is_deleted = Some(true);
        builder.apply([message(vec![deleted])].into_iter(), &mut realtime);

        assert!(realtime.alerts().is_empty(), "the alert is gone");
    });
}

#[test]
fn unsupported_entities_are_counted() {
    with_consumer(|consumer| {
        let shape = FeedEntity {
            shape: Some(gtfs_rt::Shape::default()),
            ..entity("shape")
        };

        let (_, report) =
            RealtimeBuilder::new(consumer).build_with_report([message(vec![shape])].into_iter());

        assert_eq!(report.unsupported, 1, "shapes from a feed aren't supported");
    });
}

#[test]
fn a_canceled_trip_has_no_effective_times() {
    with_consumer(|consumer| {
        let mut trip = descriptor("trip4");
        trip.schedule_relationship = Some(TripRelationship::Canceled as i32);
        let realtime = RealtimeBuilder::new(consumer)
            .build([message(vec![trip_update(trip, vec![])])].into_iter());
        let stop_times = consumer.stop_times_by_trip(trip_index(consumer, "trip4"));

        assert!(
            realtime.is_trip_canceled(trip_index(consumer, "trip4")),
            "the trip is canceled"
        );
        assert_eq!(
            realtime.arrival_time(&stop_times[0]),
            None,
            "nothing arrives on a canceled trip"
        );
    });
}

#[test]
fn clearing_forgets_everything() {
    with_consumer(|consumer| {
        let mut realtime = Realtime::new(consumer);
        RealtimeBuilder::new(consumer).apply(
            [message(vec![
                trip_update(descriptor("trip4"), vec![stop_update(0, 60)]),
                alert_entity("a1", "stop2"),
            ])]
            .into_iter(),
            &mut realtime,
        );

        realtime.clear();

        assert!(realtime.alerts().is_empty(), "alerts are gone");
        assert!(
            realtime
                .stop_time_departure_delays()
                .iter()
                .all(|delay| delay.is_none()),
            "delays are gone"
        );
        assert!(
            realtime
                .trip_status()
                .iter()
                .all(|&status| status == TripStatus::Unchanged),
            "statuses are back to unchanged"
        );
        let _ = Delay::NONE;
    });
}

#[test]
fn the_agency_time_zone_makes_delays_exact_across_a_clock_change() {
    with_consumer(|consumer| {
        // The fixture's agency is in Europe/Paris, where the clocks went back on 2026-10-25. The
        // trip4 stop leaves at 09:00 local time, which that day is 08:00 UTC.
        let day = Date::parse_gtfs("20261025").expect("a valid date");
        let on_time = i64::from(day.0) * 86_400 + 8 * 3600;
        let mut trip = descriptor("trip4");
        trip.start_date = Some("20261025".to_owned());
        let departing = |time: i64| StopTimeUpdate {
            stop_sequence: Some(0),
            departure: Some(StopTimeEvent {
                time: Some(time),
                ..Default::default()
            }),
            ..Default::default()
        };
        let delay_of = |builder: RealtimeBuilder<'_>, time: i64| {
            let realtime = builder.build(
                [message(vec![trip_update(
                    trip.clone(),
                    vec![departing(time)],
                )])]
                .into_iter(),
            );
            realtime.stop_time_departure_delays()[first_slot(consumer, "trip4")].as_option()
        };

        assert_eq!(
            delay_of(RealtimeBuilder::new(consumer), on_time + 90),
            Some(Delay(90)),
            "the zone from the file puts 09:01:30 local at 90 seconds late"
        );
        assert_eq!(
            delay_of(
                RealtimeBuilder::new(consumer).with_utc_offset_seconds(7200),
                on_time + 90
            ),
            Some(Delay(3690)),
            "a fixed summer offset is an hour out on a day the clocks went back"
        );
        assert_eq!(
            delay_of(
                RealtimeBuilder::new(consumer)
                    .with_time_zone("Europe/Stockholm")
                    .expect("a known zone"),
                on_time + 90
            ),
            Some(Delay(90)),
            "a zone named by the caller works the same"
        );
    });
}

#[test]
fn an_unknown_time_zone_is_an_error() {
    with_consumer(|consumer| {
        assert!(
            RealtimeBuilder::new(consumer)
                .with_time_zone("Mars/Olympus")
                .is_err(),
            "no such zone"
        );
    });
}

#[test]
fn the_direction_tells_apart_trips_that_start_together() {
    with_consumer(|consumer| {
        // trip4 (direction 1) and trip7 (direction 0) are both route2 trips starting at 09:00.
        let mut by_start = TripDescriptor {
            route_id: Some("route2".to_owned()),
            start_time: Some("09:00:00".to_owned()),
            direction_id: Some(1),
            schedule_relationship: Some(TripRelationship::Canceled as i32),
            ..Default::default()
        };

        let realtime = RealtimeBuilder::new(consumer)
            .build([message(vec![trip_update(by_start.clone(), vec![])])].into_iter());
        assert_eq!(
            realtime.trip_status()[trip_index(consumer, "trip4").as_usize()],
            TripStatus::Canceled,
            "direction 1 is trip4"
        );
        assert_eq!(
            realtime.trip_status()[trip_index(consumer, "trip7").as_usize()],
            TripStatus::Unchanged,
            "and not trip7"
        );

        by_start.direction_id = Some(0);
        let realtime = RealtimeBuilder::new(consumer)
            .build([message(vec![trip_update(by_start, vec![])])].into_iter());
        assert_eq!(
            realtime.trip_status()[trip_index(consumer, "trip7").as_usize()],
            TripStatus::Canceled,
            "direction 0 is trip7"
        );
    });
}

#[test]
fn alerts_can_be_asked_for_by_stop_route_and_trip() {
    with_consumer(|consumer| {
        let realtime = RealtimeBuilder::new(consumer)
            .build([message(vec![alert_entity("a1", "stop2")])].into_iter());
        let stop2 = consumer
            .stop_by_id("stop2")
            .expect("the fixture has stop2")
            .index;
        let stop3 = consumer
            .stop_by_id("stop3")
            .expect("the fixture has stop3")
            .index;
        let route1 = consumer
            .route_by_id("route1")
            .expect("the fixture has route1")
            .index;

        assert_eq!(
            realtime.alerts_for_stop(stop2, 150).count(),
            1,
            "about stop2 while active"
        );
        assert_eq!(
            realtime.alerts_for_stop(stop2, 500).count(),
            0,
            "but not after it ends"
        );
        assert_eq!(
            realtime.alerts_for_stop(stop3, 150).count(),
            0,
            "and not about stop3"
        );
        assert_eq!(
            realtime.alerts_for_route(route1, 150).count(),
            0,
            "nor about a route"
        );
        assert_eq!(
            realtime
                .alerts_for_trip(trip_index(consumer, "trip1"), 150)
                .count(),
            0,
            "nor a trip"
        );
    });
}
