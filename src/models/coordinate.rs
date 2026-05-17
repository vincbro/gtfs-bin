use crate::models::sentinel::Sentinel;
use bytemuck::{Pod, Zeroable};

/// A memory-efficient geographic coordinate.
///
/// To save space in the binary file, `f64` coordinates (16 bytes)
/// are scaled by 1,000,000 and packed into 32-bit integers (8 bytes total).
/// This maintains 6 decimal places of precision, which equates to roughly
/// 11 centimeters of accuracy at the equator—perfect for GTFS routing.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable, PartialEq, Eq)]
pub struct Coordinate {
    /// The latitude, scaled by 10,000,000 and rounded to the nearest integer.
    packed_latitude: i32,
    /// The longitude, scaled by 10,000,000 and rounded to the nearest integer.
    packed_longitude: i32,
}

impl Coordinate {
    /// Creates a new `Coordinate` by compressing standard floating-point coordinates.
    ///
    /// The input values are multiplied by 1,000,000 and rounded to preserve
    /// precision before being stored as 32-bit integers.
    #[must_use]
    pub fn new(latitude: f64, longitude: f64) -> Self {
        // Explicitly allow truncation because max geographic bounds
        // (180 * 10^7) fit safely within i32::MAX.
        #[allow(clippy::cast_possible_truncation)]
        Self {
            packed_latitude: (latitude * 10_000_000.0).round() as i32,
            packed_longitude: (longitude * 10_000_000.0).round() as i32,
        }
    }

    /// Decompresses the internal integer back into a standard `f64` latitude.
    ///
    /// This restores the original value up to 6 decimal places.
    #[must_use]
    pub fn lat_f64(&self) -> f64 {
        f64::from(self.packed_latitude) / 10_000_000.0
    }

    /// Decompresses the internal integer back into a standard `f64` longitude.
    ///
    /// This restores the original value up to 6 decimal places.
    #[must_use]
    pub fn lon_f64(&self) -> f64 {
        f64::from(self.packed_longitude) / 10_000_000.0
    }

    /// Returns both the decompressed latitude and longitude as a tuple.
    ///
    /// Useful for passing coordinates directly into distance calculation algorithms
    /// like the Haversine formula.
    #[must_use]
    pub fn tuple_f64(&self) -> (f64, f64) {
        (self.lat_f64(), self.lon_f64())
    }
}

impl From<(f64, f64)> for Coordinate {
    fn from((lat, lon): (f64, f64)) -> Self {
        Self::new(lat, lon)
    }
}

impl Sentinel for Coordinate {
    const NONE: Self = Self {
        packed_latitude: i32::MAX,
        packed_longitude: i32::MAX,
    };
}
