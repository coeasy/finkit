//! Slot-addressed persistent state arena for planned and streaming execution.
//!
//! Unlike [`crate::buffer_arena::BufferArena`], which owns recyclable temporary
//! arrays, `StateArena` stores long-lived rolling/streaming state. Compute plans
//! address states through compact [`StateSlot`] ids so hot execution does not
//! require string-keyed lookups.

use std::any::{Any, TypeId};
use std::fmt;

/// Compact identifier for one persistent state entry in a [`StateArena`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateSlot(pub usize);

struct StateEntry {
    type_id: TypeId,
    value: Box<dyn Any + Send>,
}

impl fmt::Debug for StateEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateEntry")
            .field("type_id", &self.type_id)
            .finish_non_exhaustive()
    }
}

/// Errors returned when a slot is accessed with an incompatible state type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateTypeMismatch {
    /// Slot whose existing state has another concrete type.
    pub slot: StateSlot,
}

impl fmt::Display for StateTypeMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "state type mismatch at slot {}", self.slot.0)
    }
}

impl std::error::Error for StateTypeMismatch {}

/// Persistent state storage addressed by integer slots.
///
/// A plan can allocate slots once during compilation and reuse the same arena
/// for every incremental update. Values are type-checked only at the API
/// boundary; successful hot-path lookups are direct vector indexing followed by
/// an `Any` downcast, with no hashing or string comparisons.
#[derive(Default)]
pub struct StateArena {
    entries: Vec<Option<StateEntry>>,
    live: usize,
}

impl fmt::Debug for StateArena {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateArena")
            .field("slots", &self.entries.len())
            .field("live", &self.live)
            .finish()
    }
}

impl StateArena {
    /// Create an empty arena.
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            live: 0,
        }
    }

    /// Reserve addressable slots without constructing state values.
    pub fn reserve_slots(&mut self, slots: usize) {
        if self.entries.len() < slots {
            self.entries.resize_with(slots, || None);
        }
    }

    /// Insert or replace a state value at `slot`.
    pub fn insert<T: Any + Send>(&mut self, slot: StateSlot, value: T) {
        self.reserve_slots(slot.0 + 1);
        if self.entries[slot.0].is_none() {
            self.live += 1;
        }
        self.entries[slot.0] = Some(StateEntry {
            type_id: TypeId::of::<T>(),
            value: Box::new(value),
        });
    }

    /// Return an immutable typed state reference when the slot and type match.
    pub fn get<T: Any + Send>(&self, slot: StateSlot) -> Option<&T> {
        self.entries
            .get(slot.0)?
            .as_ref()?
            .value
            .downcast_ref::<T>()
    }

    /// Return a mutable typed state reference when the slot and type match.
    pub fn get_mut<T: Any + Send>(&mut self, slot: StateSlot) -> Option<&mut T> {
        self.entries
            .get_mut(slot.0)?
            .as_mut()?
            .value
            .downcast_mut::<T>()
    }

    /// Get an existing state or initialize the empty slot exactly once.
    ///
    /// A different pre-existing concrete type is an error instead of silently
    /// replacing state, which protects a precompiled plan from slot collisions.
    pub fn get_or_insert_with<T: Any + Send>(
        &mut self,
        slot: StateSlot,
        init: impl FnOnce() -> T,
    ) -> Result<&mut T, StateTypeMismatch> {
        self.reserve_slots(slot.0 + 1);
        match self.entries[slot.0].as_ref() {
            Some(entry) if entry.type_id != TypeId::of::<T>() => {
                return Err(StateTypeMismatch { slot });
            }
            Some(_) => {}
            None => {
                self.entries[slot.0] = Some(StateEntry {
                    type_id: TypeId::of::<T>(),
                    value: Box::new(init()),
                });
                self.live += 1;
            }
        }
        self.get_mut::<T>(slot).ok_or(StateTypeMismatch { slot })
    }

    /// Remove a state value and return ownership when its type matches.
    pub fn remove<T: Any + Send>(&mut self, slot: StateSlot) -> Option<T> {
        let entry = self.entries.get_mut(slot.0)?.take()?;
        match entry.value.downcast::<T>() {
            Ok(value) => {
                self.live -= 1;
                Some(*value)
            }
            Err(value) => {
                self.entries[slot.0] = Some(StateEntry {
                    type_id: entry.type_id,
                    value,
                });
                None
            }
        }
    }

    /// Drop all persistent state while retaining slot vector capacity.
    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            *entry = None;
        }
        self.live = 0;
    }

    /// Number of currently populated state slots.
    pub const fn len(&self) -> usize {
        self.live
    }

    /// Whether no state is currently stored.
    pub const fn is_empty(&self) -> bool {
        self.live == 0
    }

    /// Number of addressable slot positions currently provisioned.
    pub fn slot_capacity(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct RollingSum {
        sum: f64,
    }

    #[test]
    fn slot_access_reuses_persistent_state_without_string_lookup() {
        let mut arena = StateArena::new();
        let slot = StateSlot(3);
        let state = arena
            .get_or_insert_with(slot, || RollingSum { sum: 1.0 })
            .unwrap();
        state.sum += 2.0;

        assert_eq!(arena.get::<RollingSum>(slot).unwrap().sum, 3.0);
        assert_eq!(arena.len(), 1);
        assert_eq!(arena.slot_capacity(), 4);
    }

    #[test]
    fn incompatible_slot_type_is_rejected() {
        let mut arena = StateArena::new();
        let slot = StateSlot(0);
        arena.insert(slot, RollingSum { sum: 2.0 });

        let error = arena.get_or_insert_with(slot, || 42usize).unwrap_err();
        assert_eq!(error.slot, slot);
        assert_eq!(arena.get::<RollingSum>(slot).unwrap().sum, 2.0);
    }

    #[test]
    fn remove_preserves_value_on_type_mismatch() {
        let mut arena = StateArena::new();
        let slot = StateSlot(1);
        arena.insert(slot, RollingSum { sum: 7.0 });

        assert_eq!(arena.remove::<usize>(slot), None);
        assert_eq!(arena.get::<RollingSum>(slot).unwrap().sum, 7.0);
        assert_eq!(arena.len(), 1);
        assert_eq!(arena.remove::<RollingSum>(slot), Some(RollingSum { sum: 7.0 }));
        assert!(arena.is_empty());
    }
}
