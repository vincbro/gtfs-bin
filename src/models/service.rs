use crate::models::{Date, ServiceBinarySlice, ServiceIdSlice, ServiceIdx, WeekdaySet};
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

    ///  A binary mask representing the days from start_date to end_date. 1 is running 0 is not running
    pub active_mask: ServiceBinarySlice,

    // ------------------------------------------------------------------------
    // 2. SMALLER FIELDS LAST (4 bytes)
    // ------------------------------------------------------------------------
    /// The internal array index of this service in the file.
    pub idx: ServiceIdx,

    /// Start service day for the service interval.
    pub start_date: Date,

    /// End service day for the service interval. This service day is included in the interval.
    pub end_date: Date,

    pub weekdays: WeekdaySet,
    pub _pad: [u8; 3], // Pad to ensure 4-byte alignment for bytemuck
}
