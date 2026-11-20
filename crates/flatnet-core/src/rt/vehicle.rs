use crate::models::{Coordinate, Optional, StopIndex};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleStopStatus {
    IncomingAt,
    StoppedAt,
    #[default]
    InTransitTo,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Occupancy {
    #[default]
    Empty,
    ManySeatsAvailable,
    FewSeatsAvailable,
    StandingRoomOnly,
    CrushedStandingRoomOnly,
    Full,
    NotAcceptingPassengers,
    NoDataAvailable,
    NotBoardable,
}

/// What a source calls a vehicle, for a person to tell it apart.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct VehicleDescription {
    pub id: Option<Box<str>>,
    pub label: Option<Box<str>>,
    pub license_plate: Option<Box<str>>,
}

#[derive(Default, Debug, Clone)]
pub struct Vehicle {
    pub position: Coordinate,
    pub current_status: VehicleStopStatus,
    pub occupancy_status: Occupancy,
    pub occupancy_percentage: Option<u32>,
    pub current_stop_sequence: Option<u32>,
    /// The stop the vehicle is at or heading to, if the feed names one the static data has.
    pub current_stop: Optional<StopIndex>,
    /// Degrees clockwise from north.
    pub bearing: Option<f32>,
    /// Meters per second.
    pub speed: Option<f32>,
    /// When the position was measured, as a POSIX time.
    pub timestamp: Option<u64>,
    /// Boxed, because most feeds name no vehicle and the position is read far more than this.
    pub description: Option<Box<VehicleDescription>>,
}
