use crate::models::{Distance, Optional, StopIndex, StopTimeIndex, StringSlice, Time, TripIndex};
use bytemuck::{Pod, Zeroable};

/// A single GTFS ``stop_time``.
///
/// Based on the GTFS standard: <https://gtfs.org/documentation/schedule/reference/#stop_timestxt>
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct StopTime {
    /// Text that appears on signage identifying the trip's destination to riders.
    pub headsign: Optional<StringSlice>,

    pub index: StopTimeIndex,

    pub stop_index: StopIndex,

    pub trip_index: TripIndex,

    /// Order of stops.
    pub sequence: u32,

    /// Arrival time at the stop.
    pub arrival_time: Optional<Time>,

    /// Departure time from the stop.
    pub departure_time: Optional<Time>,

    /// Actual distance traveled along the associated shape.
    pub distance_traveled: Optional<Distance>,
}
