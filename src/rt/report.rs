use std::ops::AddAssign;

/// What happened to the entities of the feeds an `apply` call read.
///
/// An entity that is not applied is not an error, because feeds routinely mention trips the static
/// data doesn't have. The report is how a caller notices that a feed isn't matching.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ApplyReport {
    /// Trip updates that were matched to a trip and applied.
    pub trip_updates: usize,
    /// Trip updates for a trip the static data doesn't have.
    pub trip_updates_unmatched: usize,
    /// Trip updates that matched more than one trip, so none was chosen.
    pub trip_updates_ambiguous: usize,
    /// Trip updates older than one already applied.
    pub trip_updates_stale: usize,
    /// Trip updates for a service date other than the one the `Realtime` is for.
    pub wrong_service_date: usize,
    /// Stop time updates applied to a stop time.
    pub stop_time_updates: usize,
    /// Stop time updates that no stop time of their trip matched.
    pub stop_time_updates_unmatched: usize,
    /// Vehicle positions applied to a trip.
    pub vehicles: usize,
    /// Vehicle positions with no trip or a trip that wasn't found.
    pub vehicles_unmatched: usize,
    /// Trips the feed added that aren't in the static data.
    pub added_trips: usize,
    /// Alerts stored.
    pub alerts: usize,
    /// Entities marked as deleted.
    pub deleted: usize,
    /// Entities of a kind that isn't supported: shapes, stops, trip modifications, and trips that
    /// replace or duplicate another.
    pub unsupported: usize,
}

impl AddAssign for ApplyReport {
    fn add_assign(&mut self, other: Self) {
        self.trip_updates += other.trip_updates;
        self.trip_updates_unmatched += other.trip_updates_unmatched;
        self.trip_updates_ambiguous += other.trip_updates_ambiguous;
        self.trip_updates_stale += other.trip_updates_stale;
        self.wrong_service_date += other.wrong_service_date;
        self.stop_time_updates += other.stop_time_updates;
        self.stop_time_updates_unmatched += other.stop_time_updates_unmatched;
        self.vehicles += other.vehicles;
        self.vehicles_unmatched += other.vehicles_unmatched;
        self.added_trips += other.added_trips;
        self.alerts += other.alerts;
        self.deleted += other.deleted;
        self.unsupported += other.unsupported;
    }
}
