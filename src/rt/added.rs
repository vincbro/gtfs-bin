use crate::{
    models::{Date, Optional, RouteIndex, StopIndex, Time},
    rt::StopTimeStatus,
};

/// A trip the feed added that the static data doesn't have.
///
/// It has no index in the static tables, so it lives beside them in `Realtime`.
#[derive(Debug, Clone, PartialEq)]
pub struct AddedTrip {
    pub id: Box<str>,
    pub route: Optional<RouteIndex>,
    pub start_date: Option<Date>,
    pub stop_times: Box<[AddedStopTime]>,
    /// The POSIX time of the update, or 0 if it had none.
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddedStopTime {
    pub stop: StopIndex,
    pub sequence: Option<u32>,
    pub arrival_time: Optional<Time>,
    pub departure_time: Optional<Time>,
    pub status: StopTimeStatus,
}
