//! What the map draws for the selected item: lines, and the stops they run through.

use std::collections::HashSet;

use flatnet_core::{
    consumer::Consumer,
    models::{Coordinate, RouteIndex, StopIndex, TripIndex, TripPatternIndex},
};
use walkers::{self, Position};

use crate::selection::Item;

/// The most lines a route draws, so that a route with thousands of trips stays quick.
const MAX_LINES: usize = 300;

/// A path on the map.
#[derive(Debug)]
pub(crate) struct Line {
    pub(crate) points: Vec<Position>,
    /// Whether the points are the file's shape, and not just the stops joined with straight lines.
    pub(crate) from_shape: bool,
}

/// The lines and stops of the selected item, ready to draw.
#[derive(Debug, Default)]
pub(crate) struct Overlay {
    pub(crate) lines: Vec<Line>,
    /// In the order a trip visits them, so a stop's place in the list is its number on the map.
    pub(crate) stops: Vec<StopIndex>,
}

/// The smallest box that holds some positions.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Bounds {
    pub(crate) south: f64,
    pub(crate) north: f64,
    pub(crate) west: f64,
    pub(crate) east: f64,
}

impl Bounds {
    pub(crate) fn of(positions: impl Iterator<Item = Position>) -> Option<Self> {
        positions.fold(None, |bounds, position| {
            let (longitude, latitude) = position.x_y();
            Some(bounds.map_or(
                Self {
                    south: latitude,
                    north: latitude,
                    west: longitude,
                    east: longitude,
                },
                |bounds| Self {
                    south: bounds.south.min(latitude),
                    north: bounds.north.max(latitude),
                    west: bounds.west.min(longitude),
                    east: bounds.east.max(longitude),
                },
            ))
        })
    }

    /// Whether the two boxes share any ground.
    pub(crate) fn overlaps(self, other: Self) -> bool {
        self.south <= other.north
            && other.south <= self.north
            && self.west <= other.east
            && other.west <= self.east
    }

    pub(crate) fn center(self) -> Position {
        walkers::lat_lon(
            f64::midpoint(self.south, self.north),
            f64::midpoint(self.west, self.east),
        )
    }

    /// The larger of the two sides, in degrees of latitude.
    pub(crate) fn span_degrees(self) -> f64 {
        let width = (self.east - self.west) * self.center().y().to_radians().cos();
        width.max(self.north - self.south)
    }
}

pub(crate) fn position(coordinate: Coordinate) -> Position {
    let (latitude, longitude) = coordinate.to_f64_pair();
    walkers::lat_lon(latitude, longitude)
}

impl Overlay {
    pub(crate) fn of(consumer: &Consumer<'_>, item: Item) -> Self {
        match item {
            Item::Stop(index) => Self {
                lines: Vec::new(),
                stops: vec![index],
            },
            Item::Trip(index) => Self::trip(consumer, index),
            Item::Route(index) => Self::route(consumer, index),
            Item::Pattern(index) => Self::pattern(consumer, index),
            Item::Service(_) => Self::default(),
        }
    }

    pub(crate) fn bounds(&self, consumer: &Consumer<'_>) -> Option<Bounds> {
        let stops = self
            .stops
            .iter()
            .filter_map(|&index| stop_position(consumer, index));
        let points = self
            .lines
            .iter()
            .flat_map(|line| line.points.iter().copied());
        Bounds::of(stops.chain(points))
    }

    fn trip(consumer: &Consumer<'_>, index: TripIndex) -> Self {
        let pattern = consumer.trip_pattern_by_trip(index).index;
        let stops = consumer.stop_sequence_by_trip_pattern(pattern).to_vec();
        Self {
            lines: vec![trip_line(consumer, index, &stops)],
            stops,
        }
    }

    fn pattern(consumer: &Consumer<'_>, index: TripPatternIndex) -> Self {
        let stops = consumer.stop_sequence_by_trip_pattern(index).to_vec();
        Self {
            lines: vec![stops_line(consumer, &stops)],
            stops,
        }
    }

    /// Draws each distinct shape of the route once, and the stops of a trip that has none.
    fn route(consumer: &Consumer<'_>, index: RouteIndex) -> Self {
        let mut seen = HashSet::new();
        let mut overlay = Self::default();
        for &trip_index in consumer.trips_by_route(index) {
            if overlay.lines.len() >= MAX_LINES {
                break;
            }
            let trip = consumer.trip(trip_index);
            let key = trip.shape.as_option().map_or_else(
                || (u32::MAX, trip_index.0),
                |slice| (slice.start, slice.count),
            );
            if !seen.insert(key) {
                continue;
            }
            let stops = consumer
                .stop_sequence_by_trip_pattern(consumer.trip_pattern_by_trip(trip_index).index);
            overlay.lines.push(trip_line(consumer, trip_index, stops));
        }
        overlay
    }
}

fn stop_position(consumer: &Consumer<'_>, index: StopIndex) -> Option<Position> {
    consumer.stop(index).coordinate.as_option().map(position)
}

fn stops_line(consumer: &Consumer<'_>, stops: &[StopIndex]) -> Line {
    Line {
        points: stops
            .iter()
            .filter_map(|&index| stop_position(consumer, index))
            .collect(),
        from_shape: false,
    }
}

/// The trip's shape, or its stops joined with straight lines if it has none.
fn trip_line(consumer: &Consumer<'_>, index: TripIndex, stops: &[StopIndex]) -> Line {
    let shapes = consumer.shapes_by_trip(index);
    if shapes.is_empty() {
        return stops_line(consumer, stops);
    }
    // Each shape ends where the next one starts, so that point is only drawn once.
    let points = shapes
        .iter()
        .enumerate()
        .flat_map(|(hop, &shape)| {
            let skipped = usize::from(hop > 0);
            consumer.shape_coordinates(shape).iter().skip(skipped)
        })
        .map(|&coordinate| position(coordinate))
        .collect();
    Line {
        points,
        from_shape: true,
    }
}
