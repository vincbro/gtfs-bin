use std::collections::BTreeMap;

use flatnet_core::models::{
    PatternStop, Sentinel, Slice, Stop, StopIndex, StopSlice, StopTimeSlice, Timing, Trip,
    TripIndex, TripPattern, TripPatternIndex, TripPatternSlice, TripSlice,
};

use crate::stop_times::StopTimeRow;

pub(super) struct TripPatterns {
    pub patterns: Vec<TripPattern>,
    /// The stops of every pattern, back to back.
    pub stop_sequences: Vec<StopIndex>,
    /// What every pattern says at each of its stops, at the positions of `stop_sequences`.
    pub pattern_stops: Vec<PatternStop>,
    /// The trips of every pattern, back to back, each pattern's in the order they leave.
    pub trips_in_patterns: Vec<TripIndex>,
    /// By trip: the pattern it follows.
    pub trip_to_pattern: Vec<TripPatternIndex>,
    /// The patterns that visit each stop, back to back.
    pub stop_to_pattern: Vec<TripPatternIndex>,
    /// By stop: the run of `stop_to_pattern` that visits it.
    pub stop_to_pattern_lookup: Vec<TripPatternSlice>,
    /// The timings of every trip, pattern by pattern and trip by trip.
    pub timings: Vec<Timing>,
}

/// What a pattern says at one stop, as numbers that sort, so trips that agree are grouped: the
/// stop, the sequence, pickup, drop off, the headsign's start and length, and the distance's bits.
type Position = [u32; 7];

fn position_of(row: &StopTimeRow) -> Position {
    let headsign = row.headsign.as_option();
    [
        row.stop_index.0,
        row.sequence,
        u32::from(row.pickup.0),
        u32::from(row.drop_off.0),
        headsign.map_or(u32::MAX, |slice| slice.start),
        headsign.map_or(0, |slice| slice.count),
        row.distance_traveled
            .as_option()
            .map_or(u32::MAX, |distance| distance.0.to_bits()),
    ]
}

/// Groups the trips that agree on everything but their times, splits a group where trips overtake
/// each other, and lays the timings out pattern by pattern.
///
/// Each trip is given the run of timings that is its own.
pub(super) fn build_trip_patterns(
    trips: &mut [Trip],
    stops: &[Stop],
    rows: &[StopTimeRow],
) -> TripPatterns {
    // Ordered maps so the same input always gives the same bytes.
    let mut trips_by_position: BTreeMap<Vec<Position>, Vec<TripIndex>> = BTreeMap::new();
    for trip in trips.iter() {
        let key = rows[trip.stop_times.range()]
            .iter()
            .map(position_of)
            .collect();
        trips_by_position.entry(key).or_default().push(trip.index);
    }

    let mut built = TripPatterns {
        patterns: Vec::new(),
        stop_sequences: Vec::with_capacity(rows.len()),
        pattern_stops: Vec::with_capacity(rows.len()),
        trips_in_patterns: Vec::with_capacity(trips.len()),
        trip_to_pattern: vec![TripPatternIndex::NONE; trips.len()],
        stop_to_pattern: Vec::new(),
        stop_to_pattern_lookup: vec![TripPatternSlice::NONE; stops.len()],
        timings: Vec::with_capacity(rows.len()),
    };
    let mut patterns_by_stop: BTreeMap<StopIndex, Vec<TripPatternIndex>> = BTreeMap::new();

    for group in trips_by_position.into_values() {
        for chain in chains(&group, trips, rows) {
            let index = TripPatternIndex::from(built.patterns.len());
            add_pattern(
                &mut built,
                &mut patterns_by_stop,
                index,
                &chain,
                trips,
                rows,
            );
        }
    }

    built.stop_to_pattern = Vec::with_capacity(built.stop_sequences.len());
    for (stop_index, stop_patterns) in patterns_by_stop {
        built.stop_to_pattern_lookup[stop_index.as_usize()] =
            TripPatternSlice::from_usize(built.stop_to_pattern.len(), stop_patterns.len());
        built.stop_to_pattern.extend_from_slice(&stop_patterns);
    }
    built
}

/// Adds a pattern for `chain`, whose trips agree on everything but their times, and lays their
/// timings out after those of the patterns before it.
fn add_pattern(
    built: &mut TripPatterns,
    patterns_by_stop: &mut BTreeMap<StopIndex, Vec<TripPatternIndex>>,
    index: TripPatternIndex,
    chain: &[TripIndex],
    trips: &mut [Trip],
    rows: &[StopTimeRow],
) {
    let first_rows = &rows[trips[chain[0].as_usize()].stop_times.range()];
    built.patterns.push(TripPattern {
        stops: StopSlice::from_usize(built.stop_sequences.len(), first_rows.len()),
        trips: TripSlice::from_usize(built.trips_in_patterns.len(), chain.len()),
        index,
    });
    for row in first_rows {
        built.stop_sequences.push(row.stop_index);
        built.pattern_stops.push(PatternStop {
            headsign: row.headsign,
            sequence: row.sequence,
            distance_traveled: row.distance_traveled,
            pickup: row.pickup,
            drop_off: row.drop_off,
            padding: [0; 2],
        });
        patterns_by_stop
            .entry(row.stop_index)
            .or_default()
            .push(index);
    }

    for &trip_index in chain {
        let trip = &mut trips[trip_index.as_usize()];
        let trip_rows = &rows[trip.stop_times.range()];
        trip.stop_times = StopTimeSlice::from_usize(built.timings.len(), trip_rows.len());
        built.timings.extend(trip_rows.iter().map(|row| Timing {
            arrival: row.arrival_time,
            departure: row.departure_time,
        }));
        built.trip_to_pattern[trip_index.as_usize()] = index;
        built.trips_in_patterns.push(trip_index);
    }
}

/// Splits a group of trips that agree on everything but their times into runs in which no trip
/// overtakes the one before it, each in the order the trips leave.
///
/// A trip joins the first run it fits after, so a group with no overtaking is one run.
fn chains(group: &[TripIndex], trips: &[Trip], rows: &[StopTimeRow]) -> Vec<Vec<TripIndex>> {
    let run_of = |trip: TripIndex| &rows[trips[trip.as_usize()].stop_times.range()];
    let first_time = |trip: TripIndex| {
        run_of(trip)
            .first()
            .and_then(|row| {
                row.departure_time
                    .as_option()
                    .or_else(|| row.arrival_time.as_option())
            })
            .map_or(u32::MAX, |time| time.0)
    };
    let mut leaving = group.to_vec();
    leaving.sort_by_key(|&trip| (first_time(trip), trip));

    let mut chains: Vec<Vec<TripIndex>> = Vec::new();
    for trip in leaving {
        let fits = chains.iter_mut().find(|chain| {
            let last = chain.last().copied().expect("a chain is never empty");
            !overtakes(run_of(last), run_of(trip))
        });
        match fits {
            Some(chain) => chain.push(trip),
            None => chains.push(vec![trip]),
        }
    }
    chains
}

/// Whether `after` is earlier than `before` at any stop where both have a time.
fn overtakes(before: &[StopTimeRow], after: &[StopTimeRow]) -> bool {
    before.iter().zip(after).any(|(first, second)| {
        let earlier = |first: Option<u32>, second: Option<u32>| {
            first
                .zip(second)
                .is_some_and(|(first, second)| second < first)
        };
        earlier(
            first.arrival_time.as_option().map(|time| time.0),
            second.arrival_time.as_option().map(|time| time.0),
        ) || earlier(
            first.departure_time.as_option().map(|time| time.0),
            second.departure_time.as_option().map(|time| time.0),
        )
    })
}
