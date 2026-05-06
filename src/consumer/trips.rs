use crate::{
    consumer::Consumer,
    models::{RouteIdx, Slice, StopIdx, Trip, TripIdSlice, TripIdx},
};

impl<'a> Consumer<'a> {
    #[inline]
    #[must_use]
    pub fn trip(&self, idx: TripIdx) -> &'a Trip {
        &self.trips[idx.as_usize()]
    }

    #[must_use]
    pub fn trip_by_id(&self, id: &str) -> Option<&'a Trip> {
        self.trips_id_lookup
            .binary_search_by(|&idx| {
                let trip = self.trip(idx);
                self.trip_id(trip.id).cmp(id)
            })
            .ok()
            .map(|idx| self.trip(self.trips_id_lookup[idx]))
    }

    #[inline]
    #[must_use]
    pub fn trip_id(&self, id: TripIdSlice) -> &'a str {
        unsafe { str::from_utf8_unchecked(&self.trip_ids[id.range()]) }
    }

    #[inline]
    #[must_use]
    pub fn trips_by_route(&self, idx: RouteIdx) -> &[TripIdx] {
        let slice = self.route_to_trips_lookup[idx.as_usize()];
        &self.route_to_trips[slice.range()]
    }

    pub fn iter_trips_by_route(&self, idx: RouteIdx) -> impl Iterator<Item = &'a Trip> {
        self.trips_by_route(idx)
            .iter()
            .copied()
            .map(|idx| self.trip(idx))
    }

    #[inline]
    #[must_use]
    pub fn trips_by_stop(&self, idx: StopIdx) -> &[TripIdx] {
        let slice = self.stop_to_trips_lookup[idx.as_usize()];
        &self.stop_to_trips[slice.range()]
    }

    pub fn iter_trips_by_stop(&self, idx: StopIdx) -> impl Iterator<Item = &'a Trip> {
        self.trips_by_stop(idx)
            .iter()
            .copied()
            .map(|idx| self.trip(idx))
    }
}
