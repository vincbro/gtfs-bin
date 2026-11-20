use bytemuck::{Pod, Zeroable};

use crate::models::{Date, ServiceBinarySlice, ServiceIdSlice, ServiceIndex, Weekday};

/// A single GTFS service.
///
/// Based on the GTFS standard: <https://gtfs.org/documentation/schedule/reference/#calendartxt>.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
pub struct Service {
    /// The string ID from the original GTFS file (e.g., `SERVICE_123`).
    pub id: ServiceIdSlice,

    /// One bit per day from `start_date` to `end_date`, set when the service runs that day.
    pub active_mask: ServiceBinarySlice,

    pub index: ServiceIndex,

    /// Start service day for the service interval.
    pub start_date: Date,

    /// End service day for the service interval. This service day is included in the interval.
    pub end_date: Date,

    pub weekday: Weekday,
    /// Always zero. Makes the layout free of implicit padding, which `Pod` requires.
    pub padding: [u8; 3],
}

impl Service {
    #[must_use]
    pub const fn new(
        id: ServiceIdSlice,
        index: ServiceIndex,
        active_mask: ServiceBinarySlice,
        start_date: Date,
        end_date: Date,
        weekday: Weekday,
    ) -> Self {
        Self {
            id,
            active_mask,
            index,
            start_date,
            end_date,
            weekday,
            padding: [0_u8; 3],
        }
    }
}
