use crate::models::{Date, ServiceIdSlice, ServiceIdx};
use bytemuck::{Pod, Zeroable};

/// A single GTFS service.
///
/// Based on the GTFS standard: https://gtfs.org/documentation/schedule/reference/#calendartxt
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Service {
    // ------------------------------------------------------------------------
    // 1. LARGEST FIELDS FIRST (8 to 16 bytes)
    // ------------------------------------------------------------------------
    /// The string ID from the original GTFS file (e.g., "SERVICE_123").
    pub id: ServiceIdSlice,

    // ------------------------------------------------------------------------
    // 2. SMALLER FIELDS LAST (4 bytes)
    // ------------------------------------------------------------------------
    /// The internal array index of this service in the file.
    pub idx: ServiceIdx,

    /// Start service day for the service interval.
    pub start_day: Date,

    /// End service day for the service interval. This service day is included in the interval.
    pub end_day: Date,

    pub weekdays: u8,  // Bitmask: Bit 0 = Mon, Bit 1 = Tue, ..., Bit 6 = Sun
    pub _pad: [u8; 3], // Pad to ensure 4-byte alignment for bytemuck
}
