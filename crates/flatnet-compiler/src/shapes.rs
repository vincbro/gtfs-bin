use std::collections::HashMap;

use flatnet_core::models::{
    Coordinate, Distance, Shape, ShapeIndex, ShapePointSlice, Slice, Stop, StopIndex, Trip,
    TripShapeSlice,
};

use crate::network::Call;

/// What the file keeps of the shapes: the pieces between stops, their points, and which pieces
/// each trip uses.
#[derive(Debug, Default)]
pub(super) struct BuiltShapes {
    pub(super) shapes: Vec<Shape>,
    pub(super) points: Vec<Coordinate>,
    pub(super) trip_shapes: Vec<ShapeIndex>,
}

/// Cuts the line each trip follows at the trip's stops, and gives the trip a slice of the pieces.
///
/// `trip_lines` holds, for each trip, the position in `lines` of the shape it names. A trip gets
/// no shape if it has fewer than two stops or one of its stops has no position.
pub(super) fn build_trip_shapes(
    trips: &mut [Trip],
    trip_lines: &[Option<usize>],
    lines: &[Vec<Coordinate>],
    stops: &[Stop],
    stop_times: &[Call],
) -> BuiltShapes {
    let mut builder = Builder::default();
    for (trip, &line) in trips.iter_mut().zip(trip_lines) {
        let Some(line) = line else {
            continue;
        };
        let sequence: Vec<StopIndex> = stop_times[trip.stop_times.range()]
            .iter()
            .map(|call| StopIndex::from(call.stop))
            .collect();
        trip.shape = builder.path(lines, stops, line, sequence).into();
    }
    builder.built
}

/// Cuts lines at stops and keeps what comes out once, however many trips repeat it.
///
/// After every call each shape in `built` has one entry in `by_points`, and the points of the
/// last shape end where `built.points` ends, so no point is stored twice or left unclaimed.
/// `fuzz_shapes.rs` compiles random networks and checks what this builds from the outside.
#[derive(Debug, Default)]
struct Builder {
    built: BuiltShapes,
    /// The shape that has these points, so a stretch of road used by many routes is kept once.
    by_points: HashMap<Vec<u8>, ShapeIndex>,
    /// The trip shapes of a line cut at a run of stops, for the trips that repeat it.
    by_cut: HashMap<(usize, Vec<StopIndex>), Option<TripShapeSlice>>,
    /// The slice that holds these shapes, so two cuts that came out the same are kept once.
    by_shapes: HashMap<Vec<ShapeIndex>, TripShapeSlice>,
}

impl Builder {
    fn path(
        &mut self,
        lines: &[Vec<Coordinate>],
        stops: &[Stop],
        line: usize,
        sequence: Vec<StopIndex>,
    ) -> Option<TripShapeSlice> {
        let key = (line, sequence);
        if let Some(&slice) = self.by_cut.get(&key) {
            return slice;
        }
        let slice = self.cut(&lines[line], stops, &key.1);
        self.by_cut.insert(key, slice);
        slice
    }

    fn cut(
        &mut self,
        line: &[Coordinate],
        stops: &[Stop],
        sequence: &[StopIndex],
    ) -> Option<TripShapeSlice> {
        if sequence.len() < 2 || line.is_empty() {
            return None;
        }
        let mut cuts = Vec::with_capacity(sequence.len());
        let mut from = 0;
        for &stop_index in sequence {
            let at = stops[stop_index.as_usize()].coordinate.as_option()?;
            from = nearest_from(line, from, at);
            cuts.push(from);
        }
        let shapes: Vec<ShapeIndex> = cuts
            .windows(2)
            .map(|pair| self.shape_of(&line[pair[0]..=pair[1]]))
            .collect();
        Some(self.slice_of(shapes))
    }

    /// Returns the shape with these points, adding it if it is new.
    fn shape_of(&mut self, points: &[Coordinate]) -> ShapeIndex {
        let key = bytemuck::cast_slice::<Coordinate, u8>(points);
        if let Some(&index) = self.by_points.get(key) {
            return index;
        }
        let index = ShapeIndex::from(self.built.shapes.len());
        self.built.shapes.push(Shape {
            points: ShapePointSlice::from_usize(self.built.points.len(), points.len()),
            length: Distance::from_meters(length_meters(points)),
        });
        self.built.points.extend_from_slice(points);
        self.by_points.insert(key.to_vec(), index);
        self.check_invariants();
        index
    }

    fn slice_of(&mut self, shapes: Vec<ShapeIndex>) -> TripShapeSlice {
        if let Some(&slice) = self.by_shapes.get(&shapes) {
            return slice;
        }
        let slice = TripShapeSlice::from_usize(self.built.trip_shapes.len(), shapes.len());
        self.built.trip_shapes.extend_from_slice(&shapes);
        self.by_shapes.insert(shapes, slice);
        self.check_invariants();
        slice
    }

    fn check_invariants(&self) {
        if !cfg!(debug_assertions) {
            return;
        }
        assert_eq!(
            self.by_points.len(),
            self.built.shapes.len(),
            "every shape is kept once, under its points"
        );
        if let Some(last) = self.built.shapes.last() {
            assert_eq!(
                last.points.range().end,
                self.built.points.len(),
                "the points table ends where the last shape's points end"
            );
        }
    }
}

/// Returns the position of the point of `line` nearest `at`, looking only at `from` and after, so
/// the stops of a trip are cut in the order the trip visits them.
fn nearest_from(line: &[Coordinate], from: usize, at: Coordinate) -> usize {
    let (latitude, longitude) = at.to_f64_pair();
    // A degree of longitude is shorter away from the equator.
    let stretch = latitude.to_radians().cos();
    let squared_distance = |point: &Coordinate| {
        let (point_latitude, point_longitude) = point.to_f64_pair();
        (point_latitude - latitude).powi(2) + ((point_longitude - longitude) * stretch).powi(2)
    };
    line[from..]
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| squared_distance(left).total_cmp(&squared_distance(right)))
        .map_or(from, |(offset, _)| from + offset)
}

fn length_meters(points: &[Coordinate]) -> f64 {
    points
        .windows(2)
        .map(|pair| pair[0].distance_meters(pair[1]))
        .sum()
}
