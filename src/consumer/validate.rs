use std::str;

use crate::{
    consumer::{Consumer, Error, Result},
    models::{Index, Optional, Sentinel, Slice},
};

impl Consumer<'_> {
    /// Checks everything the accessors rely on, so a corrupt file is an error here and not a panic
    /// later. It reads every table once.
    pub(super) fn validate(&self) -> Result<()> {
        self.validate_text()?;
        self.validate_lengths()?;
        self.validate_stops()?;
        self.validate_routes()?;
        self.validate_trips()?;
        self.validate_services()?;
        self.validate_stop_times()?;
        self.validate_lookups()?;
        self.validate_patterns()?;
        self.validate_transfers()?;
        self.validate_grid()?;
        self.validate_order()
    }

    fn validate_text(&self) -> Result<()> {
        let sections = [
            ("stop id", self.stop_ids),
            ("route id", self.route_ids),
            ("trip id", self.trip_ids),
            ("service id", self.service_ids),
            ("string", self.strings),
            ("time zone", self.timezone),
        ];
        for (section, bytes) in sections {
            str::from_utf8(bytes).map_err(|_| Error::InvalidUtf8 { section })?;
        }

        // The sections are valid text, so a slice is too as long as it doesn't cut a character.
        let strings = |bytes| str::from_utf8(bytes).unwrap_or_default();
        check_text(
            "stops",
            "id",
            self.stops.iter().map(|stop| stop.id),
            strings(self.stop_ids),
        )?;
        check_text(
            "routes",
            "id",
            self.routes.iter().map(|route| route.id),
            strings(self.route_ids),
        )?;
        check_text(
            "trips",
            "id",
            self.trips.iter().map(|trip| trip.id),
            strings(self.trip_ids),
        )?;
        check_text(
            "services",
            "id",
            self.services.iter().map(|service| service.id),
            strings(self.service_ids),
        )?;

        let text = strings(self.strings);
        let optional = |slices: Vec<_>| slices.into_iter().filter_map(Optional::as_option);
        check_text(
            "stops",
            "name",
            optional(self.stops.iter().map(|stop| stop.name).collect()),
            text,
        )?;
        check_text(
            "stops",
            "code",
            optional(self.stops.iter().map(|stop| stop.code).collect()),
            text,
        )?;
        check_text(
            "stops",
            "description",
            optional(self.stops.iter().map(|stop| stop.description).collect()),
            text,
        )?;
        check_text(
            "routes",
            "long_name",
            optional(self.routes.iter().map(|route| route.long_name).collect()),
            text,
        )?;
        check_text(
            "routes",
            "short_name",
            optional(self.routes.iter().map(|route| route.short_name).collect()),
            text,
        )?;
        check_text(
            "routes",
            "description",
            optional(self.routes.iter().map(|route| route.description).collect()),
            text,
        )?;
        check_text(
            "trips",
            "headsign",
            optional(self.trips.iter().map(|trip| trip.headsign).collect()),
            text,
        )?;
        check_text(
            "trips",
            "block",
            optional(self.trips.iter().map(|trip| trip.block).collect()),
            text,
        )?;
        check_text(
            "trips",
            "short_name",
            optional(self.trips.iter().map(|trip| trip.short_name).collect()),
            text,
        )?;
        check_text(
            "stop_times",
            "headsign",
            optional(
                self.stop_times
                    .iter()
                    .map(|stop_time| stop_time.headsign)
                    .collect(),
            ),
            text,
        )?;
        check_text(
            "search_stops",
            "name",
            self.search_stops.iter().map(|search| search.name),
            text,
        )
    }

    fn validate_lengths(&self) -> Result<()> {
        let pairs = [
            (
                "stops",
                self.stops.len(),
                "stops_id_lookup",
                self.stops_id_lookup.len(),
            ),
            (
                "routes",
                self.routes.len(),
                "routes_id_lookup",
                self.routes_id_lookup.len(),
            ),
            (
                "trips",
                self.trips.len(),
                "trips_id_lookup",
                self.trips_id_lookup.len(),
            ),
            (
                "services",
                self.services.len(),
                "services_id_lookup",
                self.services_id_lookup.len(),
            ),
            (
                "routes",
                self.routes.len(),
                "route_to_trips_lookup",
                self.route_to_trips_lookup.len(),
            ),
            (
                "stops",
                self.stops.len(),
                "stop_to_trips_lookup",
                self.stop_to_trips_lookup.len(),
            ),
            (
                "stops",
                self.stops.len(),
                "stop_to_transfer_out",
                self.stop_to_transfer_out.len(),
            ),
            (
                "stops",
                self.stops.len(),
                "stop_to_transfer_in",
                self.stop_to_transfer_in.len(),
            ),
            (
                "stops",
                self.stops.len(),
                "stop_to_trip_pattern_lookup",
                self.stop_to_trip_pattern_lookup.len(),
            ),
            (
                "trips",
                self.trips.len(),
                "trip_to_trip_pattern",
                self.trip_to_trip_pattern.len(),
            ),
            (
                "transfers",
                self.transfers.len(),
                "transfers_in_indices",
                self.transfers_in_indices.len(),
            ),
        ];
        for (table, table_length, lookup, lookup_length) in pairs {
            if table_length != lookup_length {
                return Err(Error::MismatchedSections { table, lookup });
            }
        }
        Ok(())
    }

    fn validate_stops(&self) -> Result<()> {
        let stops = self.stops;
        check_positions("stops", stops.iter().map(|stop| stop.index))?;
        check_indices(
            "stops",
            "parent_index",
            stops
                .iter()
                .filter_map(|stop| stop.parent_index.as_option()),
            stops.len(),
        )
    }

    fn validate_routes(&self) -> Result<()> {
        check_positions("routes", self.routes.iter().map(|route| route.index))
    }

    fn validate_trips(&self) -> Result<()> {
        let trips = self.trips;
        check_positions("trips", trips.iter().map(|trip| trip.index))?;
        check_indices(
            "trips",
            "route_index",
            trips.iter().map(|trip| trip.route_index),
            self.routes.len(),
        )?;
        check_indices(
            "trips",
            "service_index",
            trips
                .iter()
                .filter_map(|trip| trip.service_index.as_option()),
            self.services.len(),
        )?;
        check_slices(
            "trips",
            "stop_times",
            trips.iter().map(|trip| trip.stop_times),
            self.stop_times.len(),
        )?;
        check_slices(
            "trips",
            "shape",
            trips.iter().filter_map(|trip| trip.shape.as_option()),
            self.shapes.len(),
        )
    }

    fn validate_services(&self) -> Result<()> {
        let services = self.services;
        check_positions("services", services.iter().map(|service| service.index))?;
        let bit_count = self.active_mask.len() * 8;
        check_slices(
            "services",
            "active_mask",
            services.iter().map(|service| service.active_mask),
            bit_count,
        )?;
        // A service reads one bit for each of its days, so the mask must cover all of them.
        for service in services {
            let days = u64::from(service.end_date.0.saturating_sub(service.start_date.0)) + 1;
            let reversed = service.end_date < service.start_date;
            if reversed || days > u64::from(service.active_mask.count) {
                return Err(Error::InvalidReference {
                    table: "services",
                    field: "active_mask",
                });
            }
        }
        Ok(())
    }

    fn validate_stop_times(&self) -> Result<()> {
        let stop_times = self.stop_times;
        check_positions(
            "stop_times",
            stop_times.iter().map(|stop_time| stop_time.index),
        )?;
        check_indices(
            "stop_times",
            "stop_index",
            stop_times.iter().map(|stop_time| stop_time.stop_index),
            self.stops.len(),
        )?;
        check_indices(
            "stop_times",
            "trip_index",
            stop_times.iter().map(|stop_time| stop_time.trip_index),
            self.trips.len(),
        )
    }

    fn validate_lookups(&self) -> Result<()> {
        check_indices(
            "stops_id_lookup",
            "entry",
            self.stops_id_lookup.iter().copied(),
            self.stops.len(),
        )?;
        check_indices(
            "routes_id_lookup",
            "entry",
            self.routes_id_lookup.iter().copied(),
            self.routes.len(),
        )?;
        check_indices(
            "trips_id_lookup",
            "entry",
            self.trips_id_lookup.iter().copied(),
            self.trips.len(),
        )?;
        check_indices(
            "services_id_lookup",
            "entry",
            self.services_id_lookup.iter().copied(),
            self.services.len(),
        )?;
        check_indices(
            "route_to_trips",
            "entry",
            self.route_to_trips.iter().copied(),
            self.trips.len(),
        )?;
        check_slices(
            "route_to_trips_lookup",
            "entry",
            self.route_to_trips_lookup.iter().copied(),
            self.route_to_trips.len(),
        )?;
        check_indices(
            "stop_to_trips",
            "entry",
            self.stop_to_trips.iter().copied(),
            self.trips.len(),
        )?;
        check_slices(
            "stop_to_trips_lookup",
            "entry",
            self.stop_to_trips_lookup.iter().copied(),
            self.stop_to_trips.len(),
        )?;
        check_positions(
            "search_stops",
            self.search_stops.iter().map(|search| search.index),
        )?;
        check_slices(
            "search_stops",
            "stops",
            self.search_stops.iter().map(|search| search.stops),
            self.search_to_stops.len(),
        )?;
        check_indices(
            "search_to_stops",
            "entry",
            self.search_to_stops.iter().copied(),
            self.stops.len(),
        )
    }

    fn validate_patterns(&self) -> Result<()> {
        let patterns = self.trip_patterns;
        check_positions(
            "trip_patterns",
            patterns.iter().map(|pattern| pattern.index),
        )?;
        check_slices(
            "trip_patterns",
            "stops",
            patterns.iter().map(|pattern| pattern.stops),
            self.trip_patterns_stop_seq.len(),
        )?;
        check_slices(
            "trip_patterns",
            "trips",
            patterns.iter().map(|pattern| pattern.trips),
            self.trip_patterns_trips.len(),
        )?;
        check_indices(
            "trip_patterns_stop_seq",
            "entry",
            self.trip_patterns_stop_seq.iter().copied(),
            self.stops.len(),
        )?;
        check_indices(
            "trip_patterns_trips",
            "entry",
            self.trip_patterns_trips.iter().copied(),
            self.trips.len(),
        )?;
        check_indices(
            "trip_to_trip_pattern",
            "entry",
            self.trip_to_trip_pattern.iter().copied(),
            patterns.len(),
        )?;
        check_indices(
            "stop_to_trip_pattern",
            "entry",
            self.stop_to_trip_pattern.iter().copied(),
            patterns.len(),
        )?;
        check_slices(
            "stop_to_trip_pattern_lookup",
            "entry",
            self.stop_to_trip_pattern_lookup.iter().copied(),
            self.stop_to_trip_pattern.len(),
        )
    }

    fn validate_transfers(&self) -> Result<()> {
        let transfers = self.transfers;
        check_indices(
            "transfers",
            "from_stop_index",
            transfers.iter().map(|transfer| transfer.from_stop_index),
            self.stops.len(),
        )?;
        check_indices(
            "transfers",
            "to_stop_index",
            transfers.iter().map(|transfer| transfer.to_stop_index),
            self.stops.len(),
        )?;
        check_indices(
            "transfers",
            "from_trip_index",
            transfers
                .iter()
                .filter_map(|transfer| transfer.from_trip_index.as_option()),
            self.trips.len(),
        )?;
        check_indices(
            "transfers",
            "to_trip_index",
            transfers
                .iter()
                .filter_map(|transfer| transfer.to_trip_index.as_option()),
            self.trips.len(),
        )?;
        check_slices(
            "stop_to_transfer_out",
            "entry",
            self.stop_to_transfer_out.iter().copied(),
            transfers.len(),
        )?;
        check_indices(
            "transfers_in_indices",
            "entry",
            self.transfers_in_indices.iter().copied(),
            transfers.len(),
        )?;
        check_slices(
            "stop_to_transfer_in",
            "entry",
            self.stop_to_transfer_in.iter().copied(),
            self.transfers_in_indices.len(),
        )
    }
}

impl Consumer<'_> {
    fn validate_grid(&self) -> Result<()> {
        let cells = self.stop_grid;
        let key = |cell: &crate::models::StopCell| (cell.latitude_cell, cell.longitude_cell);
        if cells.windows(2).any(|pair| key(&pair[0]) >= key(&pair[1])) {
            return Err(Error::InvalidOrder { table: "stop_grid" });
        }
        check_slices(
            "stop_grid",
            "stops",
            cells.iter().map(|cell| cell.stops),
            self.grid_stops.len(),
        )?;
        check_indices(
            "grid_stops",
            "entry",
            self.grid_stops.iter().copied(),
            self.stops.len(),
        )
    }

    /// Checks the order the lookups and runs rely on: the id lookups sorted for binary search, and
    /// each trip's stop times its own and in sequence order.
    fn validate_order(&self) -> Result<()> {
        let text = |bytes| str::from_utf8(bytes).unwrap_or_default();
        check_sorted(
            "stops_id_lookup",
            text(self.stop_ids),
            self.stops_id_lookup,
            |index| self.stops[index.as_usize()].id,
        )?;
        check_sorted(
            "routes_id_lookup",
            text(self.route_ids),
            self.routes_id_lookup,
            |index| self.routes[index.as_usize()].id,
        )?;
        check_sorted(
            "trips_id_lookup",
            text(self.trip_ids),
            self.trips_id_lookup,
            |index| self.trips[index.as_usize()].id,
        )?;
        check_sorted(
            "services_id_lookup",
            text(self.service_ids),
            self.services_id_lookup,
            |index| self.services[index.as_usize()].id,
        )?;

        for trip in self.trips {
            let run = &self.stop_times[trip.stop_times.range()];
            if run
                .iter()
                .any(|stop_time| stop_time.trip_index != trip.index)
            {
                return Err(invalid("stop_times", "trip_index"));
            }
            if run
                .windows(2)
                .any(|pair| pair[0].sequence > pair[1].sequence)
            {
                return Err(Error::InvalidOrder {
                    table: "stop_times",
                });
            }
        }
        Ok(())
    }
}

/// Checks that the ids a lookup lists are in strictly increasing order, which is what makes a
/// binary search find them.
fn check_sorted<I: Index, S: Slice>(
    table: &'static str,
    text: &str,
    lookup: &[I],
    id_of: impl Fn(I) -> S,
) -> Result<()> {
    let mut previous: Option<&str> = None;
    for &index in lookup {
        let current = &text[id_of(index).range()];
        if previous.is_some_and(|before| before >= current) {
            return Err(Error::InvalidOrder { table });
        }
        previous = Some(current);
    }
    Ok(())
}

fn invalid(table: &'static str, field: &'static str) -> Error {
    Error::InvalidReference { table, field }
}

/// Checks that each slice lies inside a table of `length` items.
fn check_slices<S: Slice>(
    table: &'static str,
    field: &'static str,
    slices: impl Iterator<Item = S>,
    length: usize,
) -> Result<()> {
    for slice in slices {
        if slice.range().end > length {
            return Err(invalid(table, field));
        }
    }
    Ok(())
}

/// Checks that each slice lies inside the text and starts and ends on a character.
fn check_text<S: Slice>(
    table: &'static str,
    field: &'static str,
    slices: impl Iterator<Item = S>,
    text: &str,
) -> Result<()> {
    for slice in slices {
        let range = slice.range();
        let inside = range.end <= text.len();
        if !inside || !text.is_char_boundary(range.start) || !text.is_char_boundary(range.end) {
            return Err(invalid(table, field));
        }
    }
    Ok(())
}

/// Checks that each index points at an item of a table of `length` items.
fn check_indices<I: Index>(
    table: &'static str,
    field: &'static str,
    indices: impl Iterator<Item = I>,
    length: usize,
) -> Result<()> {
    for index in indices {
        if index.as_usize() >= length {
            return Err(invalid(table, field));
        }
    }
    Ok(())
}

/// Checks that each item knows its own position.
fn check_positions<I: Index>(table: &'static str, indices: impl Iterator<Item = I>) -> Result<()> {
    for (position, index) in indices.enumerate() {
        if index.as_usize() != position {
            return Err(invalid(table, "index"));
        }
    }
    Ok(())
}
