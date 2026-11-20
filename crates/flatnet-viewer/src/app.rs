//! The window: tabs and a list on the left, the map in the middle, details on the right.

use eframe::{
    App, Frame,
    egui::{self, CentralPanel, Context, Grid, Panel, ProgressBar, ScrollArea, Ui},
};
use flatnet_core::consumer::Consumer;

use crate::{
    details,
    file_info::{self, FileInfo},
    labels,
    list::{ListState, Tab},
    map::MapView,
    overlay::Overlay,
    picker,
    selection::{History, Item},
};

/// The state of the window, over the file it shows.
pub(crate) struct Viewer<'a> {
    consumer: Consumer<'a>,
    info: FileInfo,
    tab: Tab,
    list: ListState,
    history: History,
    overlay: Overlay,
    map: MapView,
    /// What the last check of the checksum said.
    checksum: Option<String>,
    /// Why the last file could not be opened.
    open_error: Option<String>,
}

impl<'a> Viewer<'a> {
    pub(crate) fn new(consumer: Consumer<'a>, info: FileInfo, context: &Context) -> Self {
        let map = MapView::new(context, &consumer);
        Self {
            consumer,
            info,
            tab: Tab::Stops,
            list: ListState::default(),
            history: History::default(),
            overlay: Overlay::default(),
            map,
            checksum: None,
            open_error: None,
        }
    }

    /// Selects an item, and moves the map to it.
    fn select(&mut self, item: Item) {
        self.history.push(item);
        self.show(item);
    }

    fn show(&mut self, item: Item) {
        self.tab = Tab::of(item);
        self.overlay = Overlay::of(&self.consumer, item);
        if let Some(bounds) = self.overlay.bounds(&self.consumer) {
            self.map.fit(bounds);
        }
    }

    /// Asks for another file, and opens it in a new window.
    fn open(&mut self) {
        let Some(path) = picker::pick() else {
            return;
        };
        self.open_error = picker::open_in_new_window(&path)
            .err()
            .map(|err| format!("could not open {}: {err}", path.display()));
    }

    fn left(&mut self, ui: &mut Ui) -> Option<Item> {
        if ui.button("Open another file...").clicked() {
            self.open();
        }
        if let Some(error) = &self.open_error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        ui.horizontal_wrapped(|ui| {
            for tab in Tab::ALL {
                ui.selectable_value(&mut self.tab, tab, tab.title());
            }
        });
        ui.separator();
        if self.tab == Tab::File {
            self.file(ui);
            return None;
        }
        self.list
            .show(ui, &self.consumer, self.tab, self.history.current())
    }

    fn right(&mut self, ui: &mut Ui) -> Option<Item> {
        ui.horizontal(|ui| {
            let back = egui::Button::new("Back");
            if ui.add_enabled(self.history.can_go_back(), back).clicked() {
                self.history.back().inspect(|&item| self.show(item));
            }
            let forward = egui::Button::new("Forward");
            if ui
                .add_enabled(self.history.can_go_forward(), forward)
                .clicked()
            {
                self.history.forward().inspect(|&item| self.show(item));
            }
        });
        let Some(item) = self.history.current() else {
            ui.label("Pick a row in a table, or a stop on the map.");
            return None;
        };
        details::show(ui, &self.consumer, item)
    }

    fn file(&mut self, ui: &mut Ui) {
        let header = &self.info.header;
        Grid::new("header")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                let mut row = |name: &str, value: String| {
                    ui.label(name);
                    ui.label(value);
                    ui.end_row();
                };
                row("path", self.info.path.clone());
                row("size", format!("{} bytes", self.info.len));
                row("version", header.version.to_string());
                row("magic", String::from_utf8_lossy(&header.magic).into_owned());
                row(
                    "time zone",
                    self.consumer.timezone().unwrap_or("none").to_owned(),
                );
                row(
                    "feed start",
                    self.consumer
                        .feed_start_date()
                        .map_or_else(|| "none".to_owned(), |date| date.to_string()),
                );
                row(
                    "feed end",
                    self.consumer
                        .feed_end_date()
                        .map_or_else(|| "none".to_owned(), |date| date.to_string()),
                );
                row(
                    "active",
                    file_info::activity(&self.consumer, file_info::today()),
                );
                let metadata = self.consumer.metadata();
                row("compiled at", file_info::compiled_at(metadata.compiled_at));
                for (name, slice) in metadata.texts() {
                    let value = labels::text(&self.consumer, slice).unwrap_or("none");
                    row(name, value.to_owned());
                }
                let settings = metadata.settings;
                row(
                    "footpaths",
                    format!(
                        "up to {} m and {} a place",
                        settings.footpath_max_meters,
                        if settings.footpath_max_per_place == u32::MAX {
                            "all".to_owned()
                        } else {
                            settings.footpath_max_per_place.to_string()
                        },
                    ),
                );
                row("parts", format!("{:?}", self.consumer.parts()));
                row(
                    "metadata json",
                    self.consumer.metadata_json().unwrap_or("none").to_owned(),
                );
                row("checksum", format!("{:#018x}", header.checksum));
            });
        if ui.button("Verify checksum").clicked() {
            self.checksum = Some(match self.consumer.verify_checksum() {
                Ok(()) => "matches".to_owned(),
                Err(err) => err.to_string(),
            });
        }
        if let Some(result) = &self.checksum {
            ui.label(result);
        }
        ui.separator();
        sections(ui, &self.info);
    }
}

/// A row per section, in file order, with its share of the file.
fn sections(ui: &mut Ui, info: &FileInfo) {
    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        Grid::new("sections").striped(true).show(ui, |ui| {
            for header in ["section", "offset", "items", "bytes", "share"] {
                ui.strong(header);
            }
            ui.end_row();
            for row in info.sections() {
                ui.label(row.name);
                ui.label(row.offset.to_string());
                ui.label(row.count.to_string());
                ui.label(row.bytes.to_string());
                let share = share_of(row.bytes, info.len);
                ui.add(
                    ProgressBar::new(share)
                        .desired_width(80.0)
                        .text(format!("{:.1}%", share * 100.0)),
                );
                ui.end_row();
            }
        });
    });
}

impl App for Viewer<'_> {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        let mut picked = None;
        Panel::left("tables")
            .default_size(380.0)
            .show(ui, |ui| picked = self.left(ui));
        Panel::right("details")
            .default_size(420.0)
            .show(ui, |ui| picked = self.right(ui).or(picked));
        CentralPanel::default().show(ui, |ui| {
            picked = self.map.show(ui, &self.consumer, &self.overlay).or(picked);
        });
        if let Some(item) = picked {
            self.select(item);
        }
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "a share of the file only has to be right to a pixel of a bar"
)]
fn share_of(bytes: u64, total: u64) -> f32 {
    bytes as f32 / total.max(1) as f32
}
