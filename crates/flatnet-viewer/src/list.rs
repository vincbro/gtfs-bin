//! The left panel: one table at a time, with a filter.

use eframe::egui::{self, ScrollArea, TextStyle, Ui};
use flatnet_core::consumer::Consumer;

use crate::{labels, selection::Item};

/// The tables the left panel can list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    File,
    Stops,
    Routes,
    Trips,
    Services,
    Patterns,
}

impl Tab {
    pub(crate) const ALL: [Self; 6] = [
        Self::File,
        Self::Stops,
        Self::Routes,
        Self::Trips,
        Self::Services,
        Self::Patterns,
    ];

    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Stops => "Stops",
            Self::Routes => "Routes",
            Self::Trips => "Trips",
            Self::Services => "Services",
            Self::Patterns => "Patterns",
        }
    }

    pub(crate) fn len(self, consumer: &Consumer<'_>) -> usize {
        match self {
            Self::File => 0,
            Self::Stops => consumer.stops().len(),
            Self::Routes => consumer.routes().len(),
            Self::Trips => consumer.trips().len(),
            Self::Services => consumer.services().len(),
            Self::Patterns => consumer.trip_patterns().len(),
        }
    }

    /// The tab that lists `item`'s table.
    pub(crate) const fn of(item: Item) -> Self {
        match item {
            Item::Stop(_) => Self::Stops,
            Item::Route(_) => Self::Routes,
            Item::Trip(_) => Self::Trips,
            Item::Service(_) => Self::Services,
            Item::Pattern(_) => Self::Patterns,
        }
    }

    fn item(self, position: usize) -> Option<Item> {
        match self {
            Self::File => None,
            Self::Stops => Some(Item::Stop(position.into())),
            Self::Routes => Some(Item::Route(position.into())),
            Self::Trips => Some(Item::Trip(position.into())),
            Self::Services => Some(Item::Service(position.into())),
            Self::Patterns => Some(Item::Pattern(position.into())),
        }
    }
}

/// The filter text, and the rows it keeps.
///
/// After `refresh`, `built_for` is the tab and filter in use, and `matches` holds the positions in
/// that tab's table that the filter keeps, each inside the table and in ascending order.
#[derive(Debug, Default)]
pub(crate) struct ListState {
    filter: String,
    /// The tab and filter `matches` was built for.
    built_for: Option<(Tab, String)>,
    /// Positions in the table, only used while the filter isn't empty.
    matches: Vec<usize>,
}

impl ListState {
    /// Shows the list, and returns the row that was clicked.
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        consumer: &Consumer<'_>,
        tab: Tab,
        selected: Option<Item>,
    ) -> Option<Item> {
        ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Filter by name or id"));
        self.refresh(consumer, tab);
        let filtered = !self.filter.is_empty();
        let total = if filtered {
            self.matches.len()
        } else {
            tab.len(consumer)
        };
        ui.label(format!("{total} of {}", tab.len(consumer)));

        let row_height = ui.text_style_height(&TextStyle::Body) + ui.spacing().item_spacing.y;
        let mut clicked = None;
        ScrollArea::vertical()
            .auto_shrink(false)
            .show_rows(ui, row_height, total, |ui, rows| {
                for row in rows {
                    let position = self.position(row, filtered);
                    clicked = row_button(ui, consumer, tab, position, selected).or(clicked);
                }
            });
        clicked
    }

    /// The position in the table of a row of the list.
    fn position(&self, row: usize, filtered: bool) -> usize {
        if filtered { self.matches[row] } else { row }
    }

    fn refresh(&mut self, consumer: &Consumer<'_>, tab: Tab) {
        let key = (tab, self.filter.clone());
        if self.built_for.as_ref() == Some(&key) {
            return;
        }
        let needle = self.filter.to_lowercase();
        self.matches = (0..tab.len(consumer))
            .filter(|&position| {
                tab.item(position).is_some_and(|item| {
                    labels::item(consumer, item)
                        .to_lowercase()
                        .contains(&needle)
                })
            })
            .collect();
        self.built_for = Some(key);
        self.check_invariants(tab.len(consumer));
    }

    fn check_invariants(&self, table_len: usize) {
        if !cfg!(debug_assertions) {
            return;
        }
        assert!(
            self.matches.windows(2).all(|pair| pair[0] < pair[1]),
            "the kept positions are in ascending order"
        );
        assert!(
            self.matches.last().is_none_or(|&last| last < table_len),
            "every kept position is inside the table"
        );
    }
}

/// One row of the list, and the item it stands for if it was clicked.
fn row_button(
    ui: &mut Ui,
    consumer: &Consumer<'_>,
    tab: Tab,
    position: usize,
    selected: Option<Item>,
) -> Option<Item> {
    let item = tab.item(position)?;
    let label = labels::item(consumer, item);
    ui.selectable_label(selected == Some(item), label)
        .clicked()
        .then_some(item)
}
