//! Places de calcul wasm partagées par tout le nœud : au plus `capacity` instances vivantes à la
//! fois. La mémoire wasm du nœud est donc bornée par `capacity` × la mémoire d'une instance,
//! quel que soit le nombre de tâches et de sous-bandes en vol.

use std::sync::{Condvar, Mutex, PoisonError};

pub(crate) struct Slots {
    free: Mutex<usize>,
    freed: Condvar,
}

/// Une place prise, rendue à la destruction.
pub(crate) struct Slot<'a>(&'a Slots);

impl Slots {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            free: Mutex::new(capacity.max(1)),
            freed: Condvar::new(),
        }
    }

    /// Attend qu'une place soit libre, puis la prend. Ne pas la garder pendant un autre `take`.
    pub(crate) fn take(&self) -> Slot<'_> {
        let mut free = self.free.lock().unwrap_or_else(PoisonError::into_inner);
        while *free == 0 {
            free = self
                .freed
                .wait(free)
                .unwrap_or_else(PoisonError::into_inner);
        }
        *free -= 1;
        Slot(self)
    }

    #[cfg(test)]
    pub(crate) fn available(&self) -> usize {
        *self.free.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Drop for Slot<'_> {
    fn drop(&mut self) {
        let mut free = self.0.free.lock().unwrap_or_else(PoisonError::into_inner);
        *free += 1;
        self.0.freed.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::Slots;

    #[test]
    fn a_slot_is_given_back_when_dropped() {
        let slots = Slots::new(2);
        let a = slots.take();
        let b = slots.take();
        assert_eq!(slots.available(), 0);
        drop(a);
        assert_eq!(slots.available(), 1);
        drop(b);
        assert_eq!(slots.available(), 2);
    }

    #[test]
    fn capacity_is_at_least_one() {
        assert_eq!(Slots::new(0).available(), 1);
    }
}
