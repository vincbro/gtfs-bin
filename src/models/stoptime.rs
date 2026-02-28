use crate::models::{Distance, Opt, StopIdx, StopTimeIdx, StringSlice, Time, TripIdx};
use bytemuck::{Pod, Zeroable};

/// A single GTFS stop_time.
///
/// Based on the GTFS standard: https://gtfs.org/documentation/schedule/reference/#stop_timestxt
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct StopTime {
    // ------------------------------------------------------------------------
    // 1. LARGEST FIELDS FIRST (8 to 16 bytes)
    // ------------------------------------------------------------------------
    /// Text that appears on signage identifying the trip's destination to riders.
    pub headsign: Opt<StringSlice>,

    // ------------------------------------------------------------------------
    // 2. SMALLER FIELDS LAST (4 bytes)
    // ------------------------------------------------------------------------
    /// The internal array index of this stop time in the file.
    pub idx: StopTimeIdx,

    /// The internal array index of the stop in the file.
    pub stop_idx: StopIdx,

    /// The internal array index of the trip in the file.
    pub trip_idx: TripIdx,

    /// Order of stops.
    pub sequence: u32,

    /// Arrival time at the stop.
    pub arrival_time: Opt<Time>,

    /// Departure time from the stop.
    pub departure_time: Opt<Time>,

    /// Actual distance traveled along the associated shape.
    pub distance_traveled: Opt<Distance>,
}
