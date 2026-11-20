//! What is selected, and the trail of what was.

use flatnet_core::models::{RouteIndex, ServiceIndex, StopIndex, TripIndex, TripPatternIndex};

/// One row of one of the file's tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Item {
    Stop(StopIndex),
    Route(RouteIndex),
    Trip(TripIndex),
    Service(ServiceIndex),
    Pattern(TripPatternIndex),
}

/// The items selected so far, with a position so that the browser's back and forward work.
///
/// After every call, `position` is on an entry, or 0 while there are none, and no entry is the
/// same as the one before it.
#[derive(Debug, Default)]
pub(crate) struct History {
    entries: Vec<Item>,
    position: usize,
}

impl History {
    pub(crate) fn current(&self) -> Option<Item> {
        self.entries.get(self.position).copied()
    }

    /// Selects `item`, and forgets whatever was ahead of the current position.
    pub(crate) fn push(&mut self, item: Item) {
        if self.current() == Some(item) {
            return;
        }
        if !self.entries.is_empty() {
            self.entries.truncate(self.position + 1);
        }
        self.entries.push(item);
        self.position = self.entries.len() - 1;
        self.check_invariants();
    }

    pub(crate) fn can_go_back(&self) -> bool {
        self.position > 0
    }

    pub(crate) fn can_go_forward(&self) -> bool {
        self.position + 1 < self.entries.len()
    }

    /// Steps back and returns the item now selected.
    pub(crate) fn back(&mut self) -> Option<Item> {
        if !self.can_go_back() {
            return None;
        }
        self.position -= 1;
        self.check_invariants();
        self.current()
    }

    /// Steps forward and returns the item now selected.
    pub(crate) fn forward(&mut self) -> Option<Item> {
        if !self.can_go_forward() {
            return None;
        }
        self.position += 1;
        self.check_invariants();
        self.current()
    }

    fn check_invariants(&self) {
        if !cfg!(debug_assertions) {
            return;
        }
        assert!(
            self.position < self.entries.len().max(1),
            "the position is on an entry, or 0 while there are none"
        );
        assert!(
            self.entries.windows(2).all(|pair| pair[0] != pair[1]),
            "no entry is the same as the one before it"
        );
    }
}

#[cfg(test)]
mod fuzz;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_after_back_drops_what_was_ahead() {
        let mut history = History::default();
        history.push(Item::Stop(StopIndex(1)));
        history.push(Item::Stop(StopIndex(2)));
        history.back();

        history.push(Item::Stop(StopIndex(3)));

        assert!(!history.can_go_forward(), "the second stop is forgotten");
        assert_eq!(history.back(), Some(Item::Stop(StopIndex(1))));
    }

    #[test]
    fn pushing_the_current_item_adds_nothing() {
        let mut history = History::default();
        history.push(Item::Stop(StopIndex(1)));
        history.push(Item::Stop(StopIndex(1)));

        assert!(!history.can_go_back(), "one entry only");
    }
}
