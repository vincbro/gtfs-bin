use crate::{
    consumer::Consumer,
    models::{
        Route, RouteIndex, SearchStop, Service, ServiceIndex, Shape, Stop, StopIndex, StopTime,
        Transfer, TransferIndex, TransferSlice, Trip, TripIndex, TripPattern, TripPatternIndex,
        TripPatternSlice, TripSlice,
    },
};

impl<'a> Consumer<'a> {
    #[must_use]
    pub const fn stops(&self) -> &'a [Stop] {
        self.stops
    }

    #[must_use]
    pub const fn stops_id_lookup(&self) -> &'a [StopIndex] {
        self.stops_id_lookup
    }

    #[must_use]
    pub const fn stop_ids(&self) -> &'a [u8] {
        self.stop_ids
    }

    #[must_use]
    pub const fn routes(&self) -> &'a [Route] {
        self.routes
    }

    #[must_use]
    pub const fn routes_id_lookup(&self) -> &'a [RouteIndex] {
        self.routes_id_lookup
    }

    #[must_use]
    pub const fn route_ids(&self) -> &'a [u8] {
        self.route_ids
    }

    #[must_use]
    pub const fn trips(&self) -> &'a [Trip] {
        self.trips
    }

    #[must_use]
    pub const fn trips_id_lookup(&self) -> &'a [TripIndex] {
        self.trips_id_lookup
    }

    #[must_use]
    pub const fn trip_ids(&self) -> &'a [u8] {
        self.trip_ids
    }

    #[must_use]
    pub const fn services(&self) -> &'a [Service] {
        self.services
    }

    #[must_use]
    pub const fn services_id_lookup(&self) -> &'a [ServiceIndex] {
        self.services_id_lookup
    }

    #[must_use]
    pub const fn service_ids(&self) -> &'a [u8] {
        self.service_ids
    }

    #[must_use]
    pub const fn active_mask(&self) -> &'a [u8] {
        self.active_mask
    }

    #[must_use]
    pub const fn route_to_trips(&self) -> &'a [TripIndex] {
        self.route_to_trips
    }

    #[must_use]
    pub const fn route_to_trips_lookup(&self) -> &'a [TripSlice] {
        self.route_to_trips_lookup
    }

    #[must_use]
    pub const fn stop_to_trips(&self) -> &'a [TripIndex] {
        self.stop_to_trips
    }

    #[must_use]
    pub const fn stop_to_trips_lookup(&self) -> &'a [TripSlice] {
        self.stop_to_trips_lookup
    }

    #[must_use]
    pub const fn search_stops(&self) -> &'a [SearchStop] {
        self.search_stops
    }

    #[must_use]
    pub const fn search_to_stops(&self) -> &'a [StopIndex] {
        self.search_to_stops
    }

    #[must_use]
    pub const fn stop_times(&self) -> &'a [StopTime] {
        self.stop_times
    }

    #[must_use]
    pub const fn shapes(&self) -> &'a [Shape] {
        self.shapes
    }

    #[must_use]
    pub const fn trip_patterns(&self) -> &'a [TripPattern] {
        self.trip_patterns
    }

    #[must_use]
    pub const fn trip_patterns_stop_seq(&self) -> &'a [StopIndex] {
        self.trip_patterns_stop_seq
    }

    #[must_use]
    pub const fn trip_patterns_trips(&self) -> &'a [TripIndex] {
        self.trip_patterns_trips
    }

    #[must_use]
    pub const fn trip_to_trip_pattern(&self) -> &'a [TripPatternIndex] {
        self.trip_to_trip_pattern
    }

    #[must_use]
    pub const fn stop_to_trip_pattern(&self) -> &'a [TripPatternIndex] {
        self.stop_to_trip_pattern
    }

    #[must_use]
    pub const fn stop_to_trip_pattern_lookup(&self) -> &'a [TripPatternSlice] {
        self.stop_to_trip_pattern_lookup
    }

    #[must_use]
    pub const fn transfers(&self) -> &'a [Transfer] {
        self.transfers
    }

    #[must_use]
    pub const fn stop_to_transfer_out(&self) -> &'a [TransferSlice] {
        self.stop_to_transfer_out
    }

    #[must_use]
    pub const fn transfers_in_indices(&self) -> &'a [TransferIndex] {
        self.transfers_in_indices
    }

    #[must_use]
    pub const fn stop_to_transfer_in(&self) -> &'a [TransferSlice] {
        self.stop_to_transfer_in
    }

    #[must_use]
    pub const fn strings(&self) -> &'a [u8] {
        self.strings
    }
}
