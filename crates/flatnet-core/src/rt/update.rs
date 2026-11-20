//! What a realtime source says, in terms that belong to no wire format.
//!
//! A decoder (`rt::to_feed` for GTFS-realtime) turns a message into a [`Feed`], and
//! `RealtimeBuilder::apply_feeds` applies it. Ids are the source's own, so the applier resolves them
//! against the file.

use crate::{
    models::{Date, Time},
    rt::{
        ActivePeriod, Cause, Effect, Occupancy, Severity, StopTimeStatus, TripStatus,
        VehicleDescription, VehicleStopStatus,
    },
};

/// One message from a source: a batch of updates sent at one time.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Feed {
    /// When the source made the message, as a POSIX time, or 0 if it doesn't say. An update with no
    /// time of its own takes this one.
    pub timestamp: u64,
    /// The kinds of update this message replaces in full, so what it leaves out of them is gone.
    pub replaces: Kinds,
    pub updates: Vec<Update>,
}

/// A choice of the kinds of update a `Realtime` holds.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Kinds {
    pub trip_updates: bool,
    pub vehicles: bool,
    pub alerts: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Update {
    Trip(TripUpdate),
    Vehicle(VehicleUpdate),
    Alert(AlertUpdate),
    Remove(Removal),
    /// Something the source sent that this crate can't apply, counted in the report.
    Unsupported,
}

/// How a source names a trip: by id, or by route, start and date.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TripRef {
    pub trip_id: Option<String>,
    pub route_id: Option<String>,
    /// When the trip starts, as a time of day, for a source with no trip id.
    pub start_time: Option<Time>,
    /// The service day the trip runs on.
    pub start_date: Option<Date>,
    pub direction: Option<u32>,
}

/// When a vehicle was, or will be, at a stop. Either field can be missing.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    /// Seconds later than scheduled. Wins over `time` when both are given.
    pub delay: Option<i32>,
    /// A POSIX time.
    pub time: Option<i64>,
}

/// What a source says about one stop of a trip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallUpdate {
    /// The stop's `sequence` in the trip. Found first, when the pattern has it.
    pub sequence: Option<u32>,
    pub stop_id: Option<String>,
    pub arrival: Option<Event>,
    pub departure: Option<Event>,
    /// What the source says happens at the stop. `Unchanged` says nothing, so the call is ignored.
    pub status: StopTimeStatus,
    /// How full the vehicle is when it leaves the stop.
    pub occupancy: Option<Occupancy>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TripUpdate {
    pub trip: TripRef,
    pub status: TripStatus,
    /// Set when the trip replaces or duplicates another, which this crate doesn't apply.
    pub unsupported: bool,
    /// Seconds late for the whole trip, spread over its stops when there are no calls.
    pub delay: Option<i32>,
    /// When the update was made, as a POSIX time. Without it the feed's time is used.
    pub timestamp: Option<u64>,
    /// In trip order.
    pub calls: Vec<CallUpdate>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
    pub bearing: Option<f32>,
    pub speed: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VehicleUpdate {
    pub trip: Option<TripRef>,
    pub position: Option<Position>,
    pub status: VehicleStopStatus,
    pub occupancy: Occupancy,
    pub occupancy_percentage: Option<u32>,
    pub current_stop_sequence: Option<u32>,
    pub stop_id: Option<String>,
    pub timestamp: Option<u64>,
    pub description: Option<VehicleDescription>,
}

/// One thing an alert is about, by the source's ids.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Selector {
    pub agency_id: Option<String>,
    pub route_id: Option<String>,
    pub route_type: Option<u16>,
    pub trip_id: Option<String>,
    pub stop_id: Option<String>,
}

/// A text in some languages.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Text {
    pub translations: Vec<Translation>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Translation {
    pub language: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlertUpdate {
    /// Names the alert, so a later one with the same id replaces it.
    pub id: String,
    pub active_periods: Vec<ActivePeriod>,
    pub selectors: Vec<Selector>,
    pub cause: Cause,
    pub effect: Effect,
    pub severity: Severity,
    pub header: Option<Text>,
    pub description: Option<Text>,
    pub url: Option<Text>,
}

/// What a source took back: the parts of one entity it no longer has.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Removal {
    /// The id of the alert, which is the id of the whole entity.
    pub id: String,
    /// The trip whose update, and added trip, to forget.
    pub trip: Option<TripRef>,
    /// The trip whose vehicle to forget.
    pub vehicle_trip: Option<TripRef>,
}
