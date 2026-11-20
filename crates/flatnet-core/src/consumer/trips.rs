use crate::{
    consumer::Consumer,
    models::{
        RouteIndex, Sentinel, Slice, StopIndex, Time, Trip, TripIdSlice, TripIndex, TripInfo,
    },
};

impl<'a> Consumer<'a> {
    /// # Panics
    ///
    /// Panics if `index` is out of range, which includes `TripIndex::NONE`.
    #[must_use]
    pub fn trip(&self, index: TripIndex) -> &'a Trip {
        &self.trips[index.as_usize()]
    }

    /// The text and block of a trip.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range, which includes `TripIndex::NONE`.
    #[must_use]
    pub fn trip_info(&self, index: TripIndex) -> &'a TripInfo {
        &self.trip_infos[index.as_usize()]
    }

    #[must_use]
    pub fn trip_by_id(&self, id: &str) -> Option<&'a Trip> {
        self.trips_id_lookup
            .binary_search_by(|&index| {
                let trip = self.trip(index);
                self.trip_id(trip.id).cmp(id)
            })
            .ok()
            .map(|position| self.trip(self.trips_id_lookup[position]))
    }

    /// # Panics
    ///
    /// Panics if the slice is out of range.
    #[must_use]
    pub fn trip_id(&self, id: TripIdSlice) -> &'a str {
        super::text(&self.trip_ids[id.range()])
    }

    /// # Panics
    ///
    /// Panics if `index` is out of range.
    #[must_use]
    pub fn trips_by_route(&self, index: RouteIndex) -> &'a [TripIndex] {
        let slice = self.route_to_trips_lookup[index.as_usize()];
        &self.route_to_trips[slice.range()]
    }

    /// Returns the trips of a route that start at `start`, in index order: a run of the route's
    /// trips, found by binary search. Several trips share a start when the route has several
    /// directions or services, so a caller still tells them apart by direction and by service day.
    /// `Time::NONE` finds nothing, because a trip with no time at its first stop has no start.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range.
    #[must_use]
    pub fn trips_by_route_start(&self, index: RouteIndex, start: Time) -> &'a [TripIndex] {
        if start == Time::NONE {
            return &[];
        }
        let slice = self.route_to_trips_lookup[index.as_usize()];
        let starts = &self.route_trip_starts[slice.range()];
        let first = starts.partition_point(|&time| time < start);
        let last = starts.partition_point(|&time| time <= start);
        &self.route_to_trips[slice.range()][first..last]
    }

    /// Returns when the trip starts: its departure from the first stop, or its arrival there if it
    /// has no departure. `None` if the trip has no time at its first stop.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range, which includes `TripIndex::NONE`.
    #[must_use]
    pub fn trip_start(&self, index: TripIndex) -> Option<Time> {
        let first = self.timings_by_trip(index).first()?;
        first
            .departure
            .as_option()
            .or_else(|| first.arrival.as_option())
    }

    /// # Panics
    ///
    /// Panics if `index` is out of range.
    pub fn iter_trips_by_route(&self, index: RouteIndex) -> impl Iterator<Item = &'a Trip> {
        self.trips_by_route(index)
            .iter()
            .map(|&trip_index| self.trip(trip_index))
    }

    /// Returns the trips that call at the stop: those of each pattern that visits it, pattern by
    /// pattern, and each trip once even if it visits the stop twice.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of range.
    pub fn iter_trips_by_stop(&self, index: StopIndex) -> impl Iterator<Item = &'a Trip> {
        let patterns = self.trip_patterns_by_stop(index);
        // A pattern that visits the stop twice is listed twice in a row.
        let first_visits = patterns
            .iter()
            .enumerate()
            .filter(move |&(position, &pattern)| position == 0 || patterns[position - 1] != pattern)
            .map(|(_, &pattern)| pattern);
        first_visits.flat_map(|pattern| self.iter_trips_in_trip_pattern(pattern))
    }
}
