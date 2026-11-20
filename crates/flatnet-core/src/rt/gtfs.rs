//! The decoder for GTFS-realtime: protobuf bytes to a `FeedMessage`, and a `FeedMessage` to the
//! source-neutral `Feed` that `RealtimeBuilder` applies.

use gtfs_rt::{
    EntitySelector, FeedEntity, FeedMessage, TranslatedString, TripDescriptor,
    alert::{Cause as GtfsCause, Effect as GtfsEffect, SeverityLevel},
    feed_header,
    trip_descriptor::ScheduleRelationship as TripRelationship,
    trip_update::{
        StopTimeEvent, StopTimeUpdate, stop_time_update::ScheduleRelationship as StopRelationship,
    },
    vehicle_position::{OccupancyStatus, VehicleStopStatus as GtfsVehicleStopStatus},
};
use prost::Message;

use crate::{
    models::{Date, Time},
    rt::{
        ActivePeriod, AlertUpdate, ApplyReport, CallUpdate, Cause, Effect, Event, Feed, Kinds,
        Occupancy, Position, Realtime, RealtimeBuilder, Removal, Result, Selector, Severity,
        StopTimeStatus, Text, Translation, TripRef, TripStatus, TripUpdate, Update,
        VehicleDescription, VehicleStopStatus, VehicleUpdate,
    },
};

/// Decodes the bytes of a GTFS-realtime protobuf feed, as publishers serve it.
///
/// # Errors
///
/// Returns an error if the bytes aren't a feed. The bytes must already be decompressed.
pub fn decode_feed(bytes: &[u8]) -> Result<FeedMessage> {
    Ok(FeedMessage::decode(bytes)?)
}

/// Turns a GTFS-realtime message into the updates it holds.
///
/// A message that says it is a full dataset replaces the kinds of entity it contains.
#[must_use]
pub fn to_feed(message: &FeedMessage) -> Feed {
    let replaces = if message.header.incrementality() == feed_header::Incrementality::FullDataset {
        Kinds {
            trip_updates: message
                .entity
                .iter()
                .any(|entity| entity.trip_update.is_some()),
            vehicles: message.entity.iter().any(|entity| entity.vehicle.is_some()),
            alerts: message.entity.iter().any(|entity| entity.alert.is_some()),
        }
    } else {
        Kinds::default()
    };

    let mut updates = Vec::with_capacity(message.entity.len());
    for entity in &message.entity {
        entity_updates(entity, &mut updates);
    }
    Feed {
        timestamp: message.header.timestamp.unwrap_or(0),
        replaces,
        updates,
    }
}

impl RealtimeBuilder<'_> {
    /// Applies every GTFS-realtime message to `realtime`. Later updates replace earlier ones.
    ///
    /// # Panics
    ///
    /// Panics if `realtime` was not built from the consumer this builder was created with.
    pub fn apply<I>(&self, messages: I, realtime: &mut Realtime) -> ApplyReport
    where
        I: Iterator<Item = FeedMessage>,
    {
        self.apply_feeds(messages.map(|message| to_feed(&message)), realtime)
    }

    pub fn build<I>(&self, messages: I) -> Realtime
    where
        I: Iterator<Item = FeedMessage>,
    {
        self.build_feeds(messages.map(|message| to_feed(&message)))
    }

    pub fn build_with_report<I>(&self, messages: I) -> (Realtime, ApplyReport)
    where
        I: Iterator<Item = FeedMessage>,
    {
        self.build_feeds_with_report(messages.map(|message| to_feed(&message)))
    }
}

fn entity_updates(entity: &FeedEntity, updates: &mut Vec<Update>) {
    if entity.is_deleted() {
        updates.push(Update::Remove(Removal {
            id: entity.id.clone(),
            trip: entity
                .trip_update
                .as_ref()
                .map(|trip_update| trip_ref(&trip_update.trip)),
            vehicle_trip: entity
                .vehicle
                .as_ref()
                .and_then(|vehicle| vehicle.trip.as_ref())
                .map(trip_ref),
        }));
        return;
    }

    if let Some(trip_update) = &entity.trip_update {
        updates.push(Update::Trip(TripUpdate {
            trip: trip_ref(&trip_update.trip),
            status: trip_status(trip_update.trip.schedule_relationship()),
            unsupported: matches!(
                trip_update.trip.schedule_relationship(),
                TripRelationship::Replacement | TripRelationship::Duplicated
            ),
            delay: trip_update.delay,
            timestamp: trip_update.timestamp,
            calls: trip_update.stop_time_update.iter().map(call).collect(),
        }));
    }
    if let Some(vehicle) = &entity.vehicle {
        updates.push(Update::Vehicle(VehicleUpdate {
            trip: vehicle.trip.as_ref().map(trip_ref),
            position: vehicle.position.as_ref().map(|position| Position {
                latitude: f64::from(position.latitude),
                longitude: f64::from(position.longitude),
                bearing: position.bearing,
                speed: position.speed,
            }),
            status: match vehicle.current_status() {
                GtfsVehicleStopStatus::IncomingAt => VehicleStopStatus::IncomingAt,
                GtfsVehicleStopStatus::StoppedAt => VehicleStopStatus::StoppedAt,
                GtfsVehicleStopStatus::InTransitTo => VehicleStopStatus::InTransitTo,
            },
            occupancy: occupancy(vehicle.occupancy_status()),
            occupancy_percentage: vehicle.occupancy_percentage,
            current_stop_sequence: vehicle.current_stop_sequence,
            stop_id: vehicle.stop_id.clone(),
            timestamp: vehicle.timestamp,
            description: vehicle
                .vehicle
                .as_ref()
                .map(|descriptor| VehicleDescription {
                    id: descriptor.id.as_deref().map(Box::from),
                    label: descriptor.label.as_deref().map(Box::from),
                    license_plate: descriptor.license_plate.as_deref().map(Box::from),
                }),
        }));
    }
    if let Some(alert) = &entity.alert {
        updates.push(Update::Alert(AlertUpdate {
            id: entity.id.clone(),
            active_periods: alert
                .active_period
                .iter()
                .map(|period| ActivePeriod {
                    start: period.start,
                    end: period.end,
                })
                .collect(),
            selectors: alert.informed_entity.iter().map(selector).collect(),
            cause: cause(alert.cause()),
            effect: effect(alert.effect()),
            severity: severity(alert.severity_level()),
            header: alert.header_text.as_ref().map(text),
            description: alert.description_text.as_ref().map(text),
            url: alert.url.as_ref().map(text),
        }));
    }
    if entity.shape.is_some() || entity.stop.is_some() || entity.trip_modifications.is_some() {
        updates.push(Update::Unsupported);
    }
}

fn trip_ref(descriptor: &TripDescriptor) -> TripRef {
    TripRef {
        trip_id: descriptor.trip_id.clone(),
        route_id: descriptor.route_id.clone(),
        start_time: descriptor.start_time.as_deref().and_then(Time::from_hms),
        start_date: descriptor.start_date.as_deref().and_then(Date::parse_gtfs),
        direction: descriptor.direction_id,
    }
}

const fn trip_status(relationship: TripRelationship) -> TripStatus {
    match relationship {
        TripRelationship::Scheduled => TripStatus::Scheduled,
        TripRelationship::Added => TripStatus::Added,
        TripRelationship::Unscheduled => TripStatus::Unscheduled,
        TripRelationship::Canceled => TripStatus::Canceled,
        TripRelationship::Deleted => TripStatus::Deleted,
        // Replaced and duplicated trips aren't supported, so they leave the trip as it was.
        _ => TripStatus::Unchanged,
    }
}

fn call(update: &StopTimeUpdate) -> CallUpdate {
    CallUpdate {
        sequence: update.stop_sequence,
        stop_id: update.stop_id.clone(),
        arrival: update.arrival.as_ref().map(event),
        departure: update.departure.as_ref().map(event),
        status: match update.schedule_relationship() {
            StopRelationship::Scheduled => StopTimeStatus::Scheduled,
            StopRelationship::Skipped => StopTimeStatus::Skipped,
            StopRelationship::NoData => StopTimeStatus::NoData,
            StopRelationship::Unscheduled => StopTimeStatus::Unscheduled,
        },
        occupancy: update
            .departure_occupancy_status
            .and_then(OccupancyStatus::from_i32)
            .map(occupancy),
    }
}

const fn event(event: &StopTimeEvent) -> Event {
    Event {
        delay: event.delay,
        time: event.time,
    }
}

fn selector(selector: &EntitySelector) -> Selector {
    Selector {
        agency_id: selector.agency_id.clone(),
        route_id: selector.route_id.clone(),
        route_type: selector
            .route_type
            .and_then(|code| u16::try_from(code).ok()),
        trip_id: selector
            .trip
            .as_ref()
            .and_then(|descriptor| descriptor.trip_id.clone()),
        stop_id: selector.stop_id.clone(),
    }
}

fn text(source: &TranslatedString) -> Text {
    Text {
        translations: source
            .translation
            .iter()
            .map(|translation| Translation {
                language: translation.language.clone(),
                text: translation.text.clone(),
            })
            .collect(),
    }
}

const fn occupancy(status: OccupancyStatus) -> Occupancy {
    match status {
        OccupancyStatus::Empty => Occupancy::Empty,
        OccupancyStatus::ManySeatsAvailable => Occupancy::ManySeatsAvailable,
        OccupancyStatus::FewSeatsAvailable => Occupancy::FewSeatsAvailable,
        OccupancyStatus::StandingRoomOnly => Occupancy::StandingRoomOnly,
        OccupancyStatus::CrushedStandingRoomOnly => Occupancy::CrushedStandingRoomOnly,
        OccupancyStatus::Full => Occupancy::Full,
        OccupancyStatus::NotAcceptingPassengers => Occupancy::NotAcceptingPassengers,
        OccupancyStatus::NoDataAvailable => Occupancy::NoDataAvailable,
        OccupancyStatus::NotBoardable => Occupancy::NotBoardable,
    }
}

const fn cause(cause: GtfsCause) -> Cause {
    use GtfsCause as Source;
    match cause {
        Source::UnknownCause => Cause::Unknown,
        Source::OtherCause => Cause::Other,
        Source::TechnicalProblem => Cause::TechnicalProblem,
        Source::Strike => Cause::Strike,
        Source::Demonstration => Cause::Demonstration,
        Source::Accident => Cause::Accident,
        Source::Holiday => Cause::Holiday,
        Source::Weather => Cause::Weather,
        Source::Maintenance => Cause::Maintenance,
        Source::Construction => Cause::Construction,
        Source::PoliceActivity => Cause::PoliceActivity,
        Source::MedicalEmergency => Cause::MedicalEmergency,
    }
}

const fn effect(effect: GtfsEffect) -> Effect {
    use GtfsEffect as Source;
    match effect {
        Source::NoService => Effect::NoService,
        Source::ReducedService => Effect::ReducedService,
        Source::SignificantDelays => Effect::SignificantDelays,
        Source::Detour => Effect::Detour,
        Source::AdditionalService => Effect::AdditionalService,
        Source::ModifiedService => Effect::ModifiedService,
        Source::OtherEffect => Effect::Other,
        Source::UnknownEffect => Effect::Unknown,
        Source::StopMoved => Effect::StopMoved,
        Source::NoEffect => Effect::NoEffect,
        Source::AccessibilityIssue => Effect::AccessibilityIssue,
    }
}

const fn severity(severity: SeverityLevel) -> Severity {
    use SeverityLevel as Source;
    match severity {
        Source::UnknownSeverity => Severity::Unknown,
        Source::Info => Severity::Info,
        Source::Warning => Severity::Warning,
        Source::Severe => Severity::Severe,
    }
}

#[cfg(test)]
mod tests {
    use gtfs_rt::FeedHeader;

    use super::*;

    #[test]
    fn a_feed_round_trips() {
        let feed = FeedMessage {
            header: FeedHeader {
                gtfs_realtime_version: "2.0".to_owned(),
                timestamp: Some(42),
                ..Default::default()
            },
            entity: vec![],
        };

        let decoded = decode_feed(&feed.encode_to_vec()).expect("a valid feed");

        assert_eq!(decoded.header.timestamp, Some(42), "the header survives");
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(
            decode_feed(&[0xff, 0xff, 0xff, 0xff]).is_err(),
            "not protobuf"
        );
    }

    #[test]
    fn a_full_dataset_replaces_the_kinds_it_holds() {
        let message = FeedMessage {
            header: FeedHeader {
                gtfs_realtime_version: "2.0".to_owned(),
                incrementality: Some(feed_header::Incrementality::FullDataset as i32),
                timestamp: Some(7),
            },
            entity: vec![FeedEntity {
                id: "a".to_owned(),
                vehicle: Some(gtfs_rt::VehiclePosition::default()),
                ..Default::default()
            }],
        };

        let feed = to_feed(&message);

        assert_eq!(feed.timestamp, 7, "the feed keeps its time");
        assert_eq!(
            feed.replaces,
            Kinds {
                vehicles: true,
                ..Kinds::default()
            },
            "only the kinds the message holds are replaced"
        );
    }
}
