use bytemuck::{Pod, Zeroable};

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

    pub stops: Section,
    pub stop_ids: Section,
    pub stops_id_lookup: Section,

    pub routes: Section,
    pub route_ids: Section,
    pub route_id_lookup: Section,

    pub trips: Section,
    pub trip_ids: Section,
    pub trip_id_lookup: Section,

    pub services: Section,
    pub service_ids: Section,
    pub service_id_lookup: Section,
    pub active_mask: Section,

    pub stop_to_trips: Section,
    pub stop_to_trips_lookup: Section,

    pub route_to_trips: Section,
    pub route_to_trips_lookup: Section,

    pub stop_times: Section,

    pub shapes: Section,

    pub trip_patterns: Section,
    pub trip_patterns_stop_seq: Section,
    pub trip_patterns_trip_seq: Section,
    pub trip_to_trip_pattern: Section,

    pub transfers: Section,
    pub stop_to_transfers_out: Section,
    pub transfers_in_indencies: Section,
    pub stop_to_transfers_in: Section,

    pub strings: Section,
}

impl Default for Header {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}
