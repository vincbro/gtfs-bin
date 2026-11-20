//! The right panel: every field of the selected row, and links to the rows it points at.

use eframe::egui::{self, Grid, ScrollArea, Ui};
use flatnet_core::{
    consumer::Consumer,
    models::{
        Coordinate, Optional, PatternStop, RouteIndex, ServiceIndex, StopIndex, Time, Timing,
        TripIndex, TripPatternIndex,
    },
};

use crate::{labels, selection::Item};

/// The most links a list in the details shows. The rest are counted.
const MAX_LINKS: usize = 200;

/// Shows the row, and returns the row whose link was clicked.
pub(crate) fn show(ui: &mut Ui, consumer: &Consumer<'_>, item: Item) -> Option<Item> {
    let mut jump = None;
    ui.heading(labels::item(consumer, item));
    ui.separator();
    ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| match item {
            Item::Stop(index) => stop(ui, consumer, index, &mut jump),
            Item::Route(index) => route(ui, consumer, index, &mut jump),
            Item::Trip(index) => trip(ui, consumer, index, &mut jump),
            Item::Service(index) => service(ui, consumer, index),
            Item::Pattern(index) => pattern(ui, consumer, index, &mut jump),
        });
    jump
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "callers pass temporaries of mixed types"
)]
fn field(ui: &mut Ui, name: &str, value: impl ToString) {
    ui.label(name);
    ui.label(value.to_string());
    ui.end_row();
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "callers pass temporaries of mixed types"
)]
fn optional_field(ui: &mut Ui, name: &str, value: Option<impl ToString>) {
    field(
        ui,
        name,
        value.map_or_else(|| "none".to_owned(), |value| value.to_string()),
    );
}

fn link_field(ui: &mut Ui, name: &str, target: Option<(Item, String)>, jump: &mut Option<Item>) {
    ui.label(name);
    match target {
        Some((item, label)) => link(ui, item, &label, jump),
        None => {
            ui.label("none");
        }
    }
    ui.end_row();
}

fn link(ui: &mut Ui, item: Item, label: &str, jump: &mut Option<Item>) {
    if ui.link(label).clicked() {
        *jump = Some(item);
    }
}

/// A collapsible list of links, cut off after `MAX_LINKS`.
fn links(
    ui: &mut Ui,
    title: &str,
    items: impl ExactSizeIterator<Item = Item>,
    consumer: &Consumer<'_>,
    jump: &mut Option<Item>,
) {
    let total = items.len();
    egui::CollapsingHeader::new(format!("{title} ({total})"))
        .id_salt(title)
        .show(ui, |ui| {
            for item in items.take(MAX_LINKS) {
                link(ui, item, &labels::item(consumer, item), jump);
            }
            if total > MAX_LINKS {
                ui.label(format!("and {} more", total - MAX_LINKS));
            }
        });
}

fn coordinate_text(coordinate: Optional<Coordinate>) -> Option<String> {
    coordinate.as_option().map(|coordinate| {
        format!(
            "{:.6}, {:.6}",
            coordinate.latitude_f64(),
            coordinate.longitude_f64()
        )
    })
}

fn stop(ui: &mut Ui, consumer: &Consumer<'_>, index: StopIndex, jump: &mut Option<Item>) {
    let stop = consumer.stop(index);
    let info = consumer.stop_info(index);
    Grid::new("stop")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            field(ui, "index", index.0);
            field(ui, "id", consumer.stop_id(stop.id));
            optional_field(ui, "name", labels::text(consumer, info.name));
            optional_field(ui, "code", labels::text(consumer, info.code));
            optional_field(ui, "description", labels::text(consumer, info.description));
            optional_field(ui, "position", coordinate_text(stop.coordinate));
            field(ui, "location type", format!("{:?}", stop.location_type));
            let parent = stop
                .parent_index
                .as_option()
                .map(|parent| (Item::Stop(parent), labels::stop(consumer, parent)));
            link_field(ui, "parent", parent, jump);
        });
    ui.separator();
    let trips: Vec<Item> = consumer
        .iter_trips_by_stop(index)
        .map(|trip| Item::Trip(trip.index))
        .collect();
    links(ui, "Trips", trips.into_iter(), consumer, jump);
    let patterns = consumer
        .trip_patterns_by_stop(index)
        .iter()
        .map(|&pattern| Item::Pattern(pattern));
    links(ui, "Patterns", patterns, consumer, jump);
    let transfers = consumer.outbound_transfers_by_stop(index);
    let targets = transfers
        .iter()
        .map(|transfer| Item::Stop(transfer.to_stop_index));
    links(ui, "Transfers to", targets, consumer, jump);
}

fn route(ui: &mut Ui, consumer: &Consumer<'_>, index: RouteIndex, jump: &mut Option<Item>) {
    let route = consumer.route(index);
    Grid::new("route")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            field(ui, "index", index.0);
            field(ui, "id", consumer.route_id(route.id));
            optional_field(ui, "short name", labels::text(consumer, route.short_name));
            optional_field(ui, "long name", labels::text(consumer, route.long_name));
            optional_field(ui, "description", labels::text(consumer, route.description));
            field(ui, "type", format!("{:?}", route.route_type));
        });
    ui.separator();
    let trips = consumer
        .trips_by_route(index)
        .iter()
        .map(|&trip| Item::Trip(trip));
    links(ui, "Trips", trips, consumer, jump);
}

fn trip(ui: &mut Ui, consumer: &Consumer<'_>, index: TripIndex, jump: &mut Option<Item>) {
    let trip = consumer.trip(index);
    let info = consumer.trip_info(index);
    Grid::new("trip")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            field(ui, "index", index.0);
            field(ui, "id", consumer.trip_id(trip.id));
            optional_field(ui, "headsign", labels::text(consumer, info.headsign));
            optional_field(ui, "short name", labels::text(consumer, info.short_name));
            optional_field(ui, "block", labels::text(consumer, info.block));
            field(ui, "direction", format!("{:?}", trip.direction));
            let route = (
                Item::Route(trip.route_index),
                labels::route(consumer, trip.route_index),
            );
            link_field(ui, "route", Some(route), jump);
            let service = (
                Item::Service(trip.service_index),
                labels::service(consumer, trip.service_index),
            );
            link_field(ui, "service", Some(service), jump);
            let pattern = consumer.trip_pattern_by_trip(index).index;
            link_field(
                ui,
                "pattern",
                Some((Item::Pattern(pattern), labels::pattern(consumer, pattern))),
                jump,
            );
            optional_field(ui, "shape", shape_text(consumer, index));
        });
    ui.separator();
    stop_times(ui, consumer, index, jump);
}

/// Describes the shapes of a trip, or returns `None` if it has none.
fn shape_text(consumer: &Consumer<'_>, index: TripIndex) -> Option<String> {
    let shapes = consumer.shapes_by_trip(index);
    if shapes.is_empty() {
        return None;
    }
    let meters: f32 = shapes
        .iter()
        .map(|&shape| consumer.shape(shape).length.0)
        .sum();
    Some(format!("{} shapes, {meters:.0} m", shapes.len()))
}

fn stop_times(ui: &mut Ui, consumer: &Consumer<'_>, index: TripIndex, jump: &mut Option<Item>) {
    let pattern = consumer.trip_pattern_by_trip(index).index;
    let stops = consumer.stop_sequence_by_trip_pattern(pattern);
    let pattern_stops = consumer.pattern_stops_by_trip_pattern(pattern);
    let timings = consumer.timings_by_trip(index);
    ui.label(format!("Stop times ({})", timings.len()));
    Grid::new("stop times").striped(true).show(ui, |ui| {
        for header in ["seq", "stop", "arrives", "departs", "distance"] {
            ui.strong(header);
        }
        ui.end_row();
        for ((&stop, pattern_stop), timing) in stops.iter().zip(pattern_stops).zip(timings) {
            stop_time_row(ui, consumer, stop, pattern_stop, *timing, jump);
        }
    });
}

fn stop_time_row(
    ui: &mut Ui,
    consumer: &Consumer<'_>,
    stop: StopIndex,
    pattern_stop: &PatternStop,
    timing: Timing,
    jump: &mut Option<Item>,
) {
    ui.label(pattern_stop.sequence.to_string());
    link(ui, Item::Stop(stop), &labels::stop(consumer, stop), jump);
    ui.label(time_text(timing.arrival));
    ui.label(time_text(timing.departure));
    let distance = pattern_stop.distance_traveled.as_option();
    ui.label(distance.map_or_else(|| "-".to_owned(), |meters| format!("{:.0} m", meters.0)));
    ui.end_row();
}

fn time_text(time: Optional<Time>) -> String {
    time.as_option()
        .map_or_else(|| "-".to_owned(), |time| time.to_string())
}

fn service(ui: &mut Ui, consumer: &Consumer<'_>, index: ServiceIndex) {
    let service = consumer.service(index);
    Grid::new("service")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            field(ui, "index", index.0);
            field(ui, "id", consumer.service_id(service.id));
            field(ui, "first day", service.start_date);
            field(ui, "last day", service.end_date);
            field(ui, "weekdays", format!("{:?}", service.weekday));
        });
}

fn pattern(ui: &mut Ui, consumer: &Consumer<'_>, index: TripPatternIndex, jump: &mut Option<Item>) {
    ui.label("Trips that call at the same stops in the same order.");
    ui.separator();
    let stops = consumer
        .stop_sequence_by_trip_pattern(index)
        .iter()
        .map(|&stop| Item::Stop(stop));
    links(ui, "Stops", stops, consumer, jump);
    let trips = consumer
        .trips_in_trip_pattern(index)
        .iter()
        .map(|&trip| Item::Trip(trip));
    links(ui, "Trips", trips, consumer, jump);
}
