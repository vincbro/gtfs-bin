//! The text that names a row, in the lists, the details and the map's tooltips.

use flatnet_core::{
    consumer::Consumer,
    models::{
        Optional, RouteIndex, ServiceIndex, StopIndex, StringSlice, TripIndex, TripPatternIndex,
    },
};

use crate::selection::Item;

/// Returns the text a string slice points at, if it points at any.
pub(crate) fn text<'a>(consumer: &Consumer<'a>, slice: Optional<StringSlice>) -> Option<&'a str> {
    slice.as_option().map(|slice| consumer.string(slice))
}

pub(crate) fn stop(consumer: &Consumer<'_>, index: StopIndex) -> String {
    let stop = consumer.stop(index);
    let id = consumer.stop_id(stop.id);
    match text(consumer, stop.name) {
        Some(name) => format!("{name} ({id})"),
        None => id.to_owned(),
    }
}

pub(crate) fn route(consumer: &Consumer<'_>, index: RouteIndex) -> String {
    let route = consumer.route(index);
    let id = consumer.route_id(route.id);
    let short = text(consumer, route.short_name).unwrap_or_default();
    let long = text(consumer, route.long_name).unwrap_or_default();
    format!("{short} {long} ({id})").trim().to_owned()
}

pub(crate) fn trip(consumer: &Consumer<'_>, index: TripIndex) -> String {
    let trip = consumer.trip(index);
    let id = consumer.trip_id(trip.id);
    match text(consumer, trip.headsign) {
        Some(headsign) => format!("{id} to {headsign}"),
        None => id.to_owned(),
    }
}

pub(crate) fn service(consumer: &Consumer<'_>, index: ServiceIndex) -> String {
    consumer.service_id(consumer.service(index).id).to_owned()
}

/// Names a pattern by its position and the first and last stop it calls at.
pub(crate) fn pattern(consumer: &Consumer<'_>, index: TripPatternIndex) -> String {
    let stops = consumer.stop_sequence_by_trip_pattern(index);
    let trips = consumer.trip_pattern(index).trips.count;
    let (Some(&first), Some(&last)) = (stops.first(), stops.last()) else {
        return format!("#{} (no stops, {trips} trips)", index.0);
    };
    let (first, last) = (stop(consumer, first), stop(consumer, last));
    format!(
        "#{}: {first} to {last} ({} stops, {trips} trips)",
        index.0,
        stops.len()
    )
}

pub(crate) fn item(consumer: &Consumer<'_>, item: Item) -> String {
    match item {
        Item::Stop(index) => stop(consumer, index),
        Item::Route(index) => route(consumer, index),
        Item::Trip(index) => trip(consumer, index),
        Item::Service(index) => service(consumer, index),
        Item::Pattern(index) => pattern(consumer, index),
    }
}
