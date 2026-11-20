use std::sync::{Arc, Mutex, PoisonError, RwLock};

use crate::rt::Realtime;

/// A `Realtime` that many threads read while one applies feeds to it.
///
/// A reader takes the current state with `current` and keeps reading it for as long as it holds
/// the `Arc`, whatever is applied meanwhile. An update works on a copy and swaps it in whole, so a
/// reader never sees a feed half applied.
///
/// After every call the state is a complete `Realtime`, and updates run one at a time.
#[derive(Debug)]
pub struct SharedRealtime {
    current: RwLock<Arc<Realtime>>,
    /// Held for a whole update, so two updates don't start from the same copy and lose one.
    updating: Mutex<()>,
}

impl SharedRealtime {
    #[must_use]
    pub fn new(realtime: Realtime) -> Self {
        Self {
            current: RwLock::new(Arc::new(realtime)),
            updating: Mutex::new(()),
        }
    }

    /// Returns the state as it is now. It doesn't change while the caller holds it.
    #[must_use]
    pub fn current(&self) -> Arc<Realtime> {
        Arc::clone(&self.current.read().unwrap_or_else(PoisonError::into_inner))
    }

    /// Changes a copy of the state with `apply` and makes it the current one, and returns what
    /// `apply` returned. Readers go on using the old state until the swap.
    pub fn update<T>(&self, apply: impl FnOnce(&mut Realtime) -> T) -> T {
        let _updating = self.updating.lock().unwrap_or_else(PoisonError::into_inner);
        let mut next = (*self.current()).clone();
        let result = apply(&mut next);
        *self.current.write().unwrap_or_else(PoisonError::into_inner) = Arc::new(next);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_send_and_sync<T: Send + Sync>() {}

    #[test]
    fn it_can_be_shared_between_threads() {
        is_send_and_sync::<SharedRealtime>();
    }
}
