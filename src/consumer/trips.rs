use crate::{
    consumer::Consumer,
    models::{RouteIndex, Slice, StopIndex, Trip, TripIdSlice, TripIndex},
};

impl<'a> Consumer<'a> {
    /// # Panics
    ///
    /// Panics if `index` is out of range, which includes `TripIndex::NONE`.
    #[must_use]
    pub fn trip(&self, index: TripIndex) -> &'a Trip {
        &self.trips[index.as_usize()]
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

    /// # Panics
    ///
    /// Panics if `index` is out of range.
    pub fn iter_trips_by_route(&self, index: RouteIndex) -> impl Iterator<Item = &'a Trip> {
        self.trips_by_route(index)
            .iter()
            .map(|&trip_index| self.trip(trip_index))
    }

    /// # Panics
    ///
    /// Panics if `index` is out of range.
    #[must_use]
    pub fn trips_by_stop(&self, index: StopIndex) -> &'a [TripIndex] {
        let slice = self.stop_to_trips_lookup[index.as_usize()];
        &self.stop_to_trips[slice.range()]
    }

    /// # Panics
    ///
    /// Panics if `index` is out of range.
    pub fn iter_trips_by_stop(&self, index: StopIndex) -> impl Iterator<Item = &'a Trip> {
        self.trips_by_stop(index)
            .iter()
            .map(|&trip_index| self.trip(trip_index))
    }
}
