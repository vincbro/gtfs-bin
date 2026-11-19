use crate::models::sentinel::Sentinel;
use bytemuck::{Pod, Zeroable};

/// A memory-efficient geographic coordinate.
///
/// To save space in the binary file, `f64` coordinates (16 bytes) are scaled by 10,000,000 and
/// packed into 32-bit integers (8 bytes total). That keeps 7 decimal places, which is roughly
/// 1 centimeter at the equator.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable, PartialEq, Eq)]
pub struct Coordinate {
    packed_latitude: i32,
    packed_longitude: i32,
}

impl Coordinate {
    /// Packs floating-point degrees into a `Coordinate`.
    #[must_use]
    pub fn new(latitude: f64, longitude: f64) -> Self {
        // Truncation can't happen for valid input because the largest value (180 * 10^7) fits in i32.
        #[allow(clippy::cast_possible_truncation)]
        Self {
            packed_latitude: (latitude * 10_000_000.0).round() as i32,
            packed_longitude: (longitude * 10_000_000.0).round() as i32,
        }
    }

    #[must_use]
    pub fn latitude_f64(self) -> f64 {
        f64::from(self.packed_latitude) / 10_000_000.0
    }

    #[must_use]
    pub fn longitude_f64(self) -> f64 {
        f64::from(self.packed_longitude) / 10_000_000.0
    }

    /// Returns the latitude and longitude, in that order.
    #[must_use]
    pub fn to_f64_pair(self) -> (f64, f64) {
        (self.latitude_f64(), self.longitude_f64())
    }
}

impl From<(f64, f64)> for Coordinate {
    fn from((latitude, longitude): (f64, f64)) -> Self {
        Self::new(latitude, longitude)
    }
}

impl Sentinel for Coordinate {
    const NONE: Self = Self {
        packed_latitude: i32::MAX,
        packed_longitude: i32::MAX,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_seven_decimal_places() {
        let coordinate = Coordinate::new(48.796_058_1, -2.449_386_4);

        assert_eq!(
            coordinate.latitude_f64(),
            48.796_058_1,
            "latitude round-trips"
        );
        assert_eq!(
            coordinate.longitude_f64(),
            -2.449_386_4,
            "longitude round-trips"
        );
    }

    #[test]
    fn extremes_fit() {
        let coordinate = Coordinate::new(90.0, 180.0);

        assert_eq!(
            coordinate.to_f64_pair(),
            (90.0, 180.0),
            "the largest valid input fits in i32"
        );
    }
}
