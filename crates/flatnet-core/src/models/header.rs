use bytemuck::{Pod, Zeroable};

use crate::models::{Date, Parts};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Section {
    pub offset: u64,
    pub count: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Header {
    pub magic: [u8; 4],
    pub version: u32,

    /// The optional parts the compiler wrote. A part left out has empty sections.
    pub parts: Parts,

    pub stops: Section,
    /// The `StopInfo` of every stop, at the stop's position.
    pub stop_infos: Section,
    pub stop_ids: Section,
    pub stops_id_lookup: Section,

    pub routes: Section,
    pub route_ids: Section,
    pub route_id_lookup: Section,

    pub trips: Section,
    /// The `TripInfo` of every trip, at the trip's position.
    pub trip_infos: Section,
    pub trip_ids: Section,
    pub trip_id_lookup: Section,

    pub services: Section,
    pub service_ids: Section,
    pub service_id_lookup: Section,
    pub active_mask: Section,

    pub search_stops: Section,
    pub search_to_stops: Section,

    pub route_to_trips: Section,
    pub route_to_trips_lookup: Section,

    /// The `Timing` of every trip at each stop of its pattern, pattern by pattern and trip by trip.
    pub timings: Section,

    /// The `Shape` of each hop between two stops.
    pub shapes: Section,
    /// The coordinates of every `Shape`, back to back.
    pub shape_points: Section,
    /// The `ShapeIndex` of each hop of every trip, back to back.
    pub trip_shapes: Section,

    pub trip_patterns: Section,
    pub trip_patterns_stop_seq: Section,
    /// The `PatternStop` of each stop of every pattern, at the same positions as `trip_patterns_stop_seq`.
    pub pattern_stops: Section,
    pub trip_patterns_trip_seq: Section,
    pub trip_to_trip_pattern: Section,
    pub stop_to_trip_pattern: Section,
    pub stop_to_trip_pattern_lookup: Section,

    pub transfers: Section,
    pub stop_to_transfers_out: Section,
    pub transfers_in_indices: Section,
    pub stop_to_transfers_in: Section,

    pub strings: Section,

    /// The agency time zone as text, such as `Europe/Stockholm`. Empty if the feed has none.
    pub timezone: Section,

    pub stop_grid: Section,
    pub grid_stops: Section,

    /// One `Metadata`.
    pub metadata: Section,
    /// A JSON object as UTF-8 text with whatever else the compiler was given, or empty.
    pub metadata_json: Section,

    /// The start of the feed's validity from `feed_info.txt`, or `Date::NONE`.
    pub feed_start_date: Date,
    /// The end of the feed's validity from `feed_info.txt`, or `Date::NONE`.
    pub feed_end_date: Date,

    /// The length of the whole file, to notice a truncated one.
    pub file_length: u64,
    /// The `checksum` of every byte after the header.
    pub checksum: u64,
}

impl Default for Header {
    fn default() -> Self {
        Self::zeroed()
    }
}
