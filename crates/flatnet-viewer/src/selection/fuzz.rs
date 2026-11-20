//! Random back, forward and select calls against a plain list and a position.

use flatnet_core::models::StopIndex;
use proptest::prelude::*;

use super::{History, Item};

#[derive(Debug, Clone)]
enum Call {
    Push(u32),
    Back,
    Forward,
}

fn calls() -> impl Strategy<Value = Vec<Call>> {
    // A few distinct stops, so that the same one is often selected twice in a row.
    let call = prop_oneof![
        (0_u32..4).prop_map(Call::Push),
        Just(Call::Back),
        Just(Call::Forward),
    ];
    prop::collection::vec(call, 0..64)
}

/// What the history should do, written out the long way.
#[derive(Debug, Default)]
struct Model {
    entries: Vec<u32>,
    position: usize,
}

impl Model {
    fn current(&self) -> Option<Item> {
        self.entries
            .get(self.position)
            .map(|&stop| Item::Stop(StopIndex(stop)))
    }

    fn push(&mut self, stop: u32) {
        if self.entries.get(self.position) == Some(&stop) {
            return;
        }
        self.entries
            .truncate((self.position + 1).min(self.entries.len()));
        self.entries.push(stop);
        self.position = self.entries.len() - 1;
    }

    fn back(&mut self) -> Option<Item> {
        if self.position == 0 {
            return None;
        }
        self.position -= 1;
        self.current()
    }

    fn forward(&mut self) -> Option<Item> {
        if self.position + 1 >= self.entries.len() {
            return None;
        }
        self.position += 1;
        self.current()
    }
}

proptest! {
    #[test]
    fn the_history_follows_the_model_and_keeps_its_invariants(calls in calls()) {
        let mut history = History::default();
        let mut model = Model::default();
        for call in calls {
            match call {
                Call::Push(stop) => {
                    history.push(Item::Stop(StopIndex(stop)));
                    model.push(stop);
                }
                Call::Back => prop_assert_eq!(history.back(), model.back(), "back lands where the model does"),
                Call::Forward => prop_assert_eq!(history.forward(), model.forward(), "forward lands where the model does"),
            }
            // The invariants are checked inside every call, so a broken one panics there.
            prop_assert_eq!(history.current(), model.current(), "the selected item is the same");
        }
    }
}
