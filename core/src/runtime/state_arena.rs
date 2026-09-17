//! Typed reusable state arena for streaming, replay and incremental execution.
//!
//! The arena owns heterogeneous kernel states behind generational handles. A
//! stale handle can never silently address a newly reused slot, and a complete
//! checkpoint can be restored without rebuilding the execution plan.

use std::any::Any;
use std::fmt;

type CloneStateFn = fn(&dyn Any) -> Box<dyn Any + Send + Sync>;

fn clone_state<T>(value: &dyn Any) -> Box<dyn Any + Send + Sync>
where
    T: Any + Clone + Send + Sync,
{
    let concrete = value
        .downcast_ref::<T>()
        .expect("state clone function received an unexpected concrete type")
        .clone();
    let concrete: Box<T> = Box::new(concrete);
    concrete
}

fn erase_any<T>(value: T) -> Box<dyn Any + Send + Sync>
where
    T: Any + Clone + Send + Sync,
{
    let concrete: Box<T> = Box::new(value);
    concrete
}

struct SlotEntry {
    generation: u64,
    value: Option<Box<dyn Any + Send + Sync>>,
    clone_fn: Option<CloneStateFn>,
    type_name: &'static str,
}

impl Clone for SlotEntry {
    fn clone(&self) -> Self {
        let value = match (&self.value, self.clone_fn) {
            (Some(value), Some(clone_fn)) => Some(clone_fn(value.as_ref())),
            (None, None) => None,
            _ => unreachable!("state slot value and clone function must agree"),
        };
        Self {
            generation: self.generation,
            value,
            clone_fn: self.clone_fn,
            type_name: self.type_name,
        }
    }
}

/// Stable generational handle to one state slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateHandle {
    index: usize,
    generation: u64,
}

impl StateHandle {
    /// Zero-based slot index. Exposed for diagnostics only.
    #[must_use]
    pub const fn index(self) -> usize {
        self.index
    }

    /// Slot generation used to reject stale handles.
    #[must_use]
    pub const fn generation(self) -> u64 {
        self.generation
    }
}

/// State-arena access failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateArenaError {
    /// The slot does not exist, is vacant, or has been reused since the handle
    /// was issued.
    InvalidHandle(StateHandle),
    /// The handle exists but contains a different concrete state type.
    TypeMismatch {
        /// Requested Rust type name.
        expected: &'static str,
        /// Concrete Rust type stored in the slot.
        actual: &'static str,
    },
}

impl fmt::Display for StateArenaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHandle(handle) => write!(
                f,
                "invalid or stale state handle {}:{}",
                handle.index, handle.generation
            ),
            Self::TypeMismatch { expected, actual } => {
                write!(
                    f,
                    "state slot type mismatch; expected {expected}, got {actual}"
                )
            }
        }
    }
}

impl std::error::Error for StateArenaError {}

/// Immutable snapshot of the complete arena.
#[derive(Clone)]
pub struct StateArenaCheckpoint {
    slots: Vec<SlotEntry>,
    free: Vec<usize>,
    live: usize,
}

impl fmt::Debug for StateArenaCheckpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateArenaCheckpoint")
            .field("slots", &self.slots.len())
            .field("live", &self.live)
            .finish()
    }
}

/// Heterogeneous kernel-state store with slot reuse and checkpoint/restore.
#[derive(Default)]
pub struct StateArena {
    slots: Vec<SlotEntry>,
    free: Vec<usize>,
    live: usize,
}

impl fmt::Debug for StateArena {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateArena")
            .field("slots", &self.slots.len())
            .field("live", &self.live)
            .field("free", &self.free.len())
            .finish()
    }
}

impl StateArena {
    /// Create an empty arena.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            live: 0,
        }
    }

    /// Number of live states.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.live
    }

    /// Whether the arena contains no live states.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.live == 0
    }

    /// Insert a typed state and return a generational handle.
    pub fn insert<T>(&mut self, value: T) -> StateHandle
    where
        T: Any + Clone + Send + Sync,
    {
        self.live += 1;
        if let Some(index) = self.free.pop() {
            let entry = &mut self.slots[index];
            entry.generation = entry.generation.wrapping_add(1).max(1);
            entry.value = Some(erase_any(value));
            entry.clone_fn = Some(clone_state::<T>);
            entry.type_name = std::any::type_name::<T>();
            StateHandle {
                index,
                generation: entry.generation,
            }
        } else {
            let index = self.slots.len();
            let generation = 1;
            self.slots.push(SlotEntry {
                generation,
                value: Some(erase_any(value)),
                clone_fn: Some(clone_state::<T>),
                type_name: std::any::type_name::<T>(),
            });
            StateHandle { index, generation }
        }
    }

    /// Borrow a typed state.
    pub fn get<T>(&self, handle: StateHandle) -> Result<&T, StateArenaError>
    where
        T: Any + Clone + Send + Sync,
    {
        let entry = self.entry(handle)?;
        entry
            .value
            .as_ref()
            .and_then(|value| value.downcast_ref::<T>())
            .ok_or_else(|| StateArenaError::TypeMismatch {
                expected: std::any::type_name::<T>(),
                actual: entry.type_name,
            })
    }

    /// Mutably borrow a typed state.
    pub fn get_mut<T>(&mut self, handle: StateHandle) -> Result<&mut T, StateArenaError>
    where
        T: Any + Clone + Send + Sync,
    {
        let entry = self.entry_mut(handle)?;
        let actual = entry.type_name;
        if let Some(value) = entry
            .value
            .as_mut()
            .and_then(|value| value.downcast_mut::<T>())
        {
            return Ok(value);
        }
        Err(StateArenaError::TypeMismatch {
            expected: std::any::type_name::<T>(),
            actual,
        })
    }

    /// Remove a state. Reusing the slot later increments its generation, so
    /// the old handle remains invalid forever.
    pub fn remove(&mut self, handle: StateHandle) -> Result<(), StateArenaError> {
        let entry = self.entry_mut(handle)?;
        entry.value = None;
        entry.clone_fn = None;
        entry.type_name = "<vacant>";
        self.free.push(handle.index);
        self.live -= 1;
        Ok(())
    }

    /// Capture every live state and slot generation.
    #[must_use]
    pub fn checkpoint(&self) -> StateArenaCheckpoint {
        StateArenaCheckpoint {
            slots: self.slots.clone(),
            free: self.free.clone(),
            live: self.live,
        }
    }

    /// Restore an earlier complete state image.
    pub fn restore(&mut self, checkpoint: &StateArenaCheckpoint) {
        self.slots.clone_from(&checkpoint.slots);
        self.free.clone_from(&checkpoint.free);
        self.live = checkpoint.live;
    }

    fn entry(&self, handle: StateHandle) -> Result<&SlotEntry, StateArenaError> {
        let entry = self
            .slots
            .get(handle.index)
            .ok_or(StateArenaError::InvalidHandle(handle))?;
        if entry.generation != handle.generation || entry.value.is_none() {
            return Err(StateArenaError::InvalidHandle(handle));
        }
        Ok(entry)
    }

    fn entry_mut(&mut self, handle: StateHandle) -> Result<&mut SlotEntry, StateArenaError> {
        let entry = self
            .slots
            .get_mut(handle.index)
            .ok_or(StateArenaError::InvalidHandle(handle))?;
        if entry.generation != handle.generation || entry.value.is_none() {
            return Err(StateArenaError::InvalidHandle(handle));
        }
        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_access_and_checkpoint_restore_round_trip() {
        let mut arena = StateArena::new();
        let handle = arena.insert(vec![1_u64, 2, 3]);
        arena.get_mut::<Vec<u64>>(handle).unwrap().push(4);
        let checkpoint = arena.checkpoint();
        arena.get_mut::<Vec<u64>>(handle).unwrap().push(5);
        assert_eq!(arena.get::<Vec<u64>>(handle).unwrap().len(), 5);

        arena.restore(&checkpoint);
        assert_eq!(
            arena.get::<Vec<u64>>(handle).unwrap().as_slice(),
            &[1, 2, 3, 4]
        );
    }

    #[test]
    fn reused_slot_rejects_stale_handle() {
        let mut arena = StateArena::new();
        let first = arena.insert(10_u64);
        arena.remove(first).unwrap();
        let second = arena.insert(20_u64);
        assert_eq!(first.index(), second.index());
        assert_ne!(first.generation(), second.generation());
        assert!(matches!(
            arena.get::<u64>(first),
            Err(StateArenaError::InvalidHandle(_))
        ));
        assert_eq!(*arena.get::<u64>(second).unwrap(), 20);
    }

    #[test]
    fn wrong_type_is_reported() {
        let mut arena = StateArena::new();
        let handle = arena.insert(7_u64);
        assert!(matches!(
            arena.get::<String>(handle),
            Err(StateArenaError::TypeMismatch { .. })
        ));
    }
}
