//! The map in the middle: every stop, and the lines and numbered stops of the selected item.

use std::env;

use eframe::egui::{self, Align2, Color32, Context, FontId, Pos2, Rect, Response, Stroke, Ui};
use flatnet_core::{
    consumer::Consumer,
    models::{Coordinate, StopIndex},
};
use walkers::{
    HeaderValue, HttpOptions, HttpTiles, Map, MapMemory, Position, Projector,
    sources::OpenStreetMap,
};

use crate::{
    labels,
    overlay::{self, Bounds, Overlay},
    selection::Item,
    shapes::{self, ShapeBox},
};

/// How close, in pixels, the pointer has to be to a stop to hover or click it.
const PICK_RADIUS: f32 = 10.0;

/// How much of the background map shows through. Faint, so the network is what stands out.
const TILE_OPACITY: f32 = 0.3;

const STOP_COLOR: Color32 = Color32::from_rgb(90, 90, 110);
const SHAPE_COLOR: Color32 = Color32::from_rgb(30, 110, 230);
const STRAIGHT_COLOR: Color32 = Color32::from_rgb(235, 120, 20);
const ALL_SHAPES_COLOR: Color32 = Color32::from_rgba_premultiplied(20, 70, 150, 90);
const SELECTED_COLOR: Color32 = Color32::from_rgb(220, 30, 60);

/// What the map shows.
#[derive(Debug, Clone, Copy)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each field is an independent checkbox in the toolbar"
)]
struct Layers {
    tiles: bool,
    stops: bool,
    all_shapes: bool,
    vertices: bool,
}

impl Default for Layers {
    fn default() -> Self {
        Self {
            tiles: true,
            stops: true,
            all_shapes: true,
            vertices: false,
        }
    }
}

/// The map, what it shows and where it looks.
pub(crate) struct MapView {
    tiles: HttpTiles,
    memory: MapMemory,
    /// Where the map starts, and where it goes back to.
    home: Position,
    layers: Layers,
    /// Every distinct shape of the file, found once when the file is opened.
    all_shapes: Vec<ShapeBox>,
}

impl MapView {
    pub(crate) fn new(context: &Context, consumer: &Consumer<'_>) -> Self {
        let options = HttpOptions {
            cache: Some(env::temp_dir().join("flatnet-viewer-tiles")),
            user_agent: Some(HeaderValue::from_static(
                "flatnet-viewer (https://github.com/vincbro/flatnet)",
            )),
            ..HttpOptions::default()
        };
        let mut view = Self {
            tiles: HttpTiles::with_options(OpenStreetMap, options, context.clone()),
            memory: MapMemory::default(),
            home: walkers::lat_lon(0.0, 0.0),
            layers: Layers::default(),
            all_shapes: shapes::all(consumer),
        };
        let stops = consumer
            .stops()
            .iter()
            .filter_map(|stop| stop.coordinate.as_option().map(overlay::position));
        if let Some(bounds) = Bounds::of(stops) {
            view.home = bounds.center();
            view.fit(bounds);
        }
        view
    }

    /// Looks at the middle of `bounds`, zoomed so that all of it is in view.
    pub(crate) fn fit(&mut self, bounds: Bounds) {
        // A world of 256 pixels per tile is about 700 pixels across at the zoom that fits it.
        let span = bounds.span_degrees().max(0.0005);
        let zoom = (700.0 * 360.0 / (256.0 * span)).log2().clamp(2.0, 19.0);
        self.memory.center_at(bounds.center());
        // An out-of-range zoom is clamped above, so the map keeps its zoom only if it refuses.
        let _ = self.memory.set_zoom(zoom);
    }

    /// Draws the map, and returns the stop that was clicked.
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        consumer: &Consumer<'_>,
        overlay: &Overlay,
    ) -> Option<Item> {
        let all_shapes = &self.all_shapes;
        let layers = &mut self.layers;
        let map = Map::new(None, &mut self.memory, self.home);
        let map = if layers.tiles {
            map.with_layer(&mut self.tiles, TILE_OPACITY)
        } else {
            map
        };
        map.show(ui, |ui, response, projector, _memory| {
            toolbar(ui, layers, all_shapes.len());
            let painter = ui.painter().with_clip_rect(projector.clip_rect());
            let cursor = response.hover_pos();
            if layers.all_shapes {
                draw_all_shapes(&painter, projector, consumer, all_shapes);
            }
            let nearest = if layers.stops {
                draw_stops(&painter, projector, consumer, cursor)
            } else {
                None
            };
            draw_overlay(&painter, projector, consumer, overlay, layers.vertices);
            attribution(&painter, projector.clip_rect());
            pick(response, consumer, nearest)
        })
        .inner
    }
}

fn toolbar(ui: &mut Ui, layers: &mut Layers, shape_count: usize) {
    egui::Frame::popup(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.checkbox(&mut layers.tiles, "Background map");
            ui.checkbox(&mut layers.stops, "All stops");
            ui.checkbox(
                &mut layers.all_shapes,
                format!("All shapes ({shape_count})"),
            );
            ui.checkbox(&mut layers.vertices, "Shape points");
        });
    });
}

fn attribution(painter: &egui::Painter, area: Rect) {
    painter.text(
        area.right_bottom() - egui::vec2(4.0, 4.0),
        Align2::RIGHT_BOTTOM,
        "© OpenStreetMap contributors",
        FontId::proportional(11.0),
        Color32::DARK_GRAY,
    );
}

/// Shows a tooltip for the stop under the pointer, and returns it if the map was clicked.
fn pick(response: &Response, consumer: &Consumer<'_>, nearest: Option<StopIndex>) -> Option<Item> {
    let index = nearest?;
    if response.clicked() {
        return Some(Item::Stop(index));
    }
    response
        .clone()
        .on_hover_text(labels::stop(consumer, index));
    None
}

/// Draws the stops in view, and returns the one nearest the pointer if it is close enough.
fn draw_stops(
    painter: &egui::Painter,
    projector: &Projector<'_>,
    consumer: &Consumer<'_>,
    cursor: Option<Pos2>,
) -> Option<StopIndex> {
    // Far out, most stops land on the same pixels, so only some are drawn.
    let stride = match projector.zoom() {
        zoom if zoom < 9.0 => 16,
        zoom if zoom < 11.0 => 4,
        zoom if zoom < 12.5 => 2,
        _ => 1,
    };
    let view = projector.clip_rect();
    let mut nearest: Option<(StopIndex, f32)> = None;
    for stop in consumer.stops().iter().step_by(stride) {
        let Some(coordinate) = stop.coordinate.as_option() else {
            continue;
        };
        let screen = projector.project(overlay::position(coordinate)).to_pos2();
        if !view.contains(screen) {
            continue;
        }
        painter.circle_filled(screen, 2.5, STOP_COLOR);
        let distance = cursor.map_or(f32::MAX, |cursor| cursor.distance(screen));
        if distance <= PICK_RADIUS && nearest.is_none_or(|(_, best)| distance < best) {
            nearest = Some((stop.index, distance));
        }
    }
    nearest.map(|(index, _)| index)
}

fn draw_overlay(
    painter: &egui::Painter,
    projector: &Projector<'_>,
    consumer: &Consumer<'_>,
    overlay: &Overlay,
    show_vertices: bool,
) {
    for line in &overlay.lines {
        let color = if line.from_shape {
            SHAPE_COLOR
        } else {
            STRAIGHT_COLOR
        };
        let points: Vec<Pos2> = line
            .points
            .iter()
            .map(|&point| projector.project(point).to_pos2())
            .collect();
        if show_vertices && line.from_shape {
            for &point in &points {
                painter.circle_filled(point, 2.0, color);
            }
        }
        painter.add(egui::Shape::line(points, Stroke::new(3.0, color)));
    }
    let numbered = overlay.stops.len() > 1;
    for (number, &index) in overlay.stops.iter().enumerate() {
        let Some(coordinate) = consumer.stop(index).coordinate.as_option() else {
            continue;
        };
        let screen = projector.project(overlay::position(coordinate)).to_pos2();
        painter.circle_filled(screen, 6.0, SELECTED_COLOR);
        if numbered {
            painter.text(
                screen,
                Align2::CENTER_CENTER,
                (number + 1).to_string(),
                FontId::proportional(9.0),
                Color32::WHITE,
            );
        }
    }
}

/// How many points to step over between two drawn ones. Far out, neighbouring points land on the
/// same pixels, so most of them need not be projected.
fn point_stride(zoom: f64) -> usize {
    match zoom {
        zoom if zoom < 8.0 => 32,
        zoom if zoom < 10.0 => 8,
        zoom if zoom < 12.0 => 4,
        zoom if zoom < 14.0 => 2,
        _ => 1,
    }
}

/// Every `stride`th point, and always the last one. A shape ends where the next one starts, so
/// dropping its last point leaves a gap in the line.
fn thinned(points: &[Coordinate], stride: usize) -> impl Iterator<Item = Coordinate> {
    let last = points.last().copied();
    let ends_on_stride = points.len().saturating_sub(1).is_multiple_of(stride);
    points
        .iter()
        .copied()
        .step_by(stride)
        .chain(last.filter(|_| !ends_on_stride))
}

/// Draws every shape that is in view, faintly, under everything else.
fn draw_all_shapes(
    painter: &egui::Painter,
    projector: &Projector<'_>,
    consumer: &Consumer<'_>,
    all_shapes: &[ShapeBox],
) {
    let view = projector.clip_rect();
    let corners = [
        projector.unproject(view.left_top().to_vec2()),
        projector.unproject(view.right_bottom().to_vec2()),
    ];
    let Some(visible) = Bounds::of(corners.into_iter()) else {
        return;
    };
    let stride = point_stride(projector.zoom());
    let stroke = Stroke::new(1.5, ALL_SHAPES_COLOR);
    let lines = all_shapes
        .iter()
        .filter(|shape| shape.bounds.overlaps(visible))
        .map(|shape| {
            let points = thinned(consumer.shape_coordinates(shape.index), stride)
                .map(|point| projector.project(overlay::position(point)).to_pos2())
                .collect();
            egui::Shape::line(points, stroke)
        });
    painter.extend(lines);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(length: u32) -> Vec<Coordinate> {
        (0..length)
            .map(|step| Coordinate::new(59.0 + f64::from(step) * 0.001, 18.0))
            .collect()
    }

    #[test]
    fn thinning_keeps_both_ends() {
        let points = line(20);

        let kept: Vec<Coordinate> = thinned(&points, 8).collect();

        assert_eq!(kept.first(), points.first(), "the start stays");
        assert_eq!(
            kept.last(),
            points.last(),
            "the end stays, so the next shape joins it"
        );
    }

    #[test]
    fn a_stride_that_lands_on_the_end_does_not_repeat_it() {
        let points = line(17);

        assert_eq!(thinned(&points, 8).count(), 3, "points 0, 8 and 16");
    }
}
