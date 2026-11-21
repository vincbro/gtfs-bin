use std::collections::HashMap;

use flatnet_core::models::{
    Coordinate, Distance, Footpath, FootpathSlice, LocationType, Place, PlaceStopSlice, Settings,
    Slice, Stop, StopCell, StopIndex,
};
use rayon::prelude::*;

/// The places, their stops and the walks between them.
pub(super) struct BuiltFootpaths {
    pub footpaths: Vec<Footpath>,
    /// The stops of every place, back to back.
    pub place_stops: Vec<StopIndex>,
    /// One for each stop, in stop order. Empty unless the stop is a place.
    pub places: Vec<Place>,
}

impl BuiltFootpaths {
    /// What a file without the footpaths part holds.
    pub(super) const fn empty() -> Self {
        Self {
            footpaths: Vec::new(),
            place_stops: Vec::new(),
            places: Vec::new(),
        }
    }
}

/// Groups the stops into places and finds, for every place, the places within
/// `settings.footpath_max_meters` in a straight line between their nearest stops. A place keeps its
/// nearest `settings.footpath_max_per_place`, nearest first and the lower index first on a tie, so
/// the same network always gives the same bytes.
///
/// A place is the root of a parent chain (see [`Place::root_of`]), and it holds the stops a
/// passenger can board at. Stations, entrances and generic nodes are not among them. Walking
/// between the stops of one place is a given, so it has no footpath.
pub(super) fn build_footpaths(stops: &[Stop], settings: Settings) -> BuiltFootpaths {
    let roots: Vec<StopIndex> = stops
        .iter()
        .map(|stop| Place::root_of(stops, stop.index))
        .collect();

    // The stops of each place in index order, so the same input gives the same bytes.
    let mut members: Vec<Vec<StopIndex>> = vec![Vec::new(); stops.len()];
    for stop in stops.iter().filter(|stop| can_board(stop)) {
        members[roots[stop.index.as_usize()].as_usize()].push(stop.index);
    }

    let mut cells: HashMap<(u32, u32), Vec<StopIndex>> = HashMap::new();
    for stop in stops.iter().filter(|stop| can_board(stop)) {
        if let Some(coordinate) = stop.coordinate.as_option() {
            let cell = StopCell::cell_of(coordinate.latitude_f64(), coordinate.longitude_f64());
            cells.entry(cell).or_default().push(stop.index);
        }
    }
    let nearby = Nearby {
        stops,
        roots: &roots,
        cells: &cells,
        settings,
    };

    let walks: Vec<Vec<Footpath>> = members
        .par_iter()
        .map(|place_members| nearby.walks_from(place_members))
        .collect();

    let mut built = BuiltFootpaths::empty();
    for (place_members, place_walks) in members.iter().zip(walks) {
        built.places.push(Place {
            stops: PlaceStopSlice::from_usize(built.place_stops.len(), place_members.len()),
            footpaths: FootpathSlice::from_usize(built.footpaths.len(), place_walks.len()),
        });
        built.place_stops.extend_from_slice(place_members);
        built.footpaths.extend(place_walks);
    }
    built
}

/// Whether a passenger can board at the stop, so it belongs to a place and can start or end a walk.
fn can_board(stop: &Stop) -> bool {
    stop.location_type != LocationType::STOP_AREA
        && stop.location_type != LocationType::STATION_ENTRANCE
        && stop.location_type != LocationType::GENERIC_NODE
}

/// What finding the places near a place needs.
struct Nearby<'a> {
    stops: &'a [Stop],
    roots: &'a [StopIndex],
    cells: &'a HashMap<(u32, u32), Vec<StopIndex>>,
    settings: Settings,
}

impl Nearby<'_> {
    /// The walks that start at a place with these stops, one for each other place in reach, at the
    /// length of the shortest walk between any stop of one and any stop of the other.
    fn walks_from(&self, place_members: &[StopIndex]) -> Vec<Footpath> {
        let Some(first) = place_members.first() else {
            return Vec::new();
        };
        let own_root = self.roots[first.as_usize()];
        let mut found: Vec<(StopIndex, f32)> = place_members
            .iter()
            .filter_map(|&member| self.stops[member.as_usize()].coordinate.as_option())
            .flat_map(|here| self.within_reach(here, own_root))
            .collect();

        // The nearest walk to each place: sorted by place and length, then the first of each place.
        found.sort_unstable_by(|first, second| {
            first.0.cmp(&second.0).then(first.1.total_cmp(&second.1))
        });
        found.dedup_by_key(|&mut (place, _)| place);
        found.sort_unstable_by(|first, second| {
            first.1.total_cmp(&second.1).then(first.0.cmp(&second.0))
        });
        let keep = usize::try_from(self.settings.footpath_max_per_place).unwrap_or(usize::MAX);
        found.truncate(keep);
        found
            .into_iter()
            .map(|(to_place_index, meters)| Footpath {
                to_place_index,
                meters: Distance(meters),
            })
            .collect()
    }

    /// The places other than `own_root` with a stop within reach of `here`, one entry for each
    /// stop, as the place and the walk in meters.
    fn within_reach(
        &self,
        here: Coordinate,
        own_root: StopIndex,
    ) -> impl Iterator<Item = (StopIndex, f32)> {
        let max_meters = f64::from(self.settings.footpath_max_meters);

        let (latitude, longitude) = here.to_f64_pair();
        let span = StopCell::cells_around(latitude, longitude, max_meters);

        span.cells()
            .filter_map(|cell| self.cells.get(&cell))
            .flatten()
            .filter_map(move |&other| {
                let place = self.roots[other.as_usize()];
                if place == own_root {
                    return None;
                }
                let there = self.stops[other.as_usize()].coordinate.as_option()?;
                let meters = here.distance_meters(there);
                // Ordered by the `f32` the file stores, so the file's order is what it checks.
                (meters <= max_meters).then_some((place, Distance::from_meters(meters).0))
            })
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use flatnet_core::models::{Optional, Sentinel, StopIdSlice};

    use super::*;

    /// Meters in a degree of latitude on the sphere `Coordinate::distance_meters` uses.
    const METERS_PER_DEGREE: f64 = 6_371_008.8 * PI / 180.0;

    /// A stop point `meters` east of the origin on the equator, under `parent` if it has one.
    fn stop_at(index: usize, meters: f64, parent: Option<usize>) -> Stop {
        let degrees = meters / METERS_PER_DEGREE;
        Stop::new(
            StopIdSlice::default(),
            StopIndex::from(index),
            Optional::new(parent.map_or(StopIndex::NONE, StopIndex::from)),
            Optional::new(Coordinate::new(0.0, degrees)),
            LocationType::STOP_POINT,
        )
    }

    fn station(index: usize) -> Stop {
        let mut stop = stop_at(index, 0.0, None);
        stop.coordinate = Optional::default();
        stop.location_type = LocationType::STOP_AREA;
        stop
    }

    fn settings(max_meters: u32, per_place: u32) -> Settings {
        Settings {
            footpath_max_meters: max_meters,
            footpath_max_per_place: per_place,
        }
    }

    fn targets(built: &BuiltFootpaths, place: usize) -> Vec<usize> {
        built.footpaths[built.places[place].footpaths.range()]
            .iter()
            .map(|footpath| footpath.to_place_index.as_usize())
            .collect()
    }

    #[test]
    fn a_place_walks_to_the_ones_in_reach_nearest_first() {
        let stops = [
            stop_at(0, 0.0, None),
            stop_at(1, 300.0, None),
            stop_at(2, 100.0, None),
            stop_at(3, 900.0, None),
        ];

        let built = build_footpaths(&stops, settings(500, u32::MAX));

        assert_eq!(
            targets(&built, 0),
            [2, 1],
            "100 m before 300 m, and 900 m is out of reach"
        );
        assert_eq!(
            targets(&built, 3),
            Vec::<usize>::new(),
            "nothing near the far stop"
        );
    }

    #[test]
    fn a_place_keeps_only_its_nearest() {
        let stops = [
            stop_at(0, 0.0, None),
            stop_at(1, 300.0, None),
            stop_at(2, 100.0, None),
            stop_at(3, 200.0, None),
        ];

        let built = build_footpaths(&stops, settings(500, 2));

        assert_eq!(targets(&built, 0), [2, 3], "the two nearest");
    }

    #[test]
    fn a_tie_goes_to_the_lower_index() {
        let stops = [
            stop_at(0, 0.0, None),
            stop_at(1, 100.0, None),
            stop_at(2, 100.0, None),
        ];

        let built = build_footpaths(&stops, settings(500, u32::MAX));

        assert_eq!(targets(&built, 0), [1, 2], "equal walks go by index");
    }

    #[test]
    fn the_stops_under_a_station_are_one_place_with_no_footpath_between_them() {
        // The station 0 has the stops 1 and 2, 50 m apart, and the stop 3 stands alone 200 m away.
        let stops = [
            station(0),
            stop_at(1, 0.0, Some(0)),
            stop_at(2, 50.0, Some(0)),
            stop_at(3, 250.0, None),
        ];

        let built = build_footpaths(&stops, settings(500, u32::MAX));

        let station_stops = &built.place_stops[built.places[0].stops.range()];
        assert_eq!(
            station_stops,
            [StopIndex::from(1_usize), StopIndex::from(2_usize)],
            "the station holds the stops under it"
        );
        assert!(
            built.places[1].stops.range().is_empty(),
            "a stop under a station is not a place"
        );
        assert_eq!(
            targets(&built, 0),
            [3],
            "the station walks to the other place, once"
        );
        assert_eq!(targets(&built, 3), [0], "and back");
        let meters = built.footpaths[built.places[0].footpaths.range()][0]
            .meters
            .0;
        assert!(
            (meters - 200.0).abs() < 1.0,
            "from its nearest stop, 50 m to 250 m, got {meters}"
        );
    }

    #[test]
    fn a_station_with_no_stop_to_board_at_is_empty() {
        let stops = [station(0), station(1)];

        let built = build_footpaths(&stops, settings(500, u32::MAX));

        assert!(built.place_stops.is_empty(), "nothing to board");
        assert!(built.footpaths.is_empty(), "so nowhere to walk");
        assert_eq!(built.places.len(), 2, "every stop still has a place entry");
    }
}
