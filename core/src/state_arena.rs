//! Slot-addressed persistent state arena for planned and streaming execution.
//!
//! Unlike [`crate::buffer_arena::BufferArena`], which owns recyclable temporary
//! arrays, `StateArena` stores long-lived rolling/streaming state. Compute plans
//! address states through compact [`StateSlot`] ids so hot execution does not
//! require string-keyed lookups.

use crate::compute::{ComputeNode, ComputeNodeId, ComputePlan};
use std::any::{Any, TypeId};
use std::collections::BTreeMap;
use std::fmt;

/// Compact identifier for one persistent state entry in a [`StateArena`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateSlot(pub usize);

/// Compile-time identity used to intern semantically identical persistent state.
///
/// `Node` preserves the historical one-state-per-compute-node behavior. Shared
/// state must be requested explicitly with a family id, ordered input nodes and
/// exact parameter fingerprints. This prevents accidental state aliasing between
/// kernels such as EMA(12) and EMA(26) while allowing canonical families such as
/// DMI/OHLC or MACD/EMA to project several outputs from one state machine.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StateKey {
    /// Unique state owned by exactly one semantic node.
    Node(ComputeNodeId),
    /// State intentionally shared by nodes with identical family/input/parameter identity.
    Shared {
        /// Stable family identifier resolved by the compile-time planner.
        family: u64,
        /// Ordered semantic input nodes feeding the shared state.
        inputs: Vec<ComputeNodeId>,
        /// Exact parameter fingerprints; floats should be supplied as `to_bits()`.
        parameters: Vec<u64>,
    },
}

impl StateKey {
    /// Preserve one unique state slot for `node`.
    pub const fn unique(node: ComputeNodeId) -> Self {
        Self::Node(node)
    }

    /// Construct an explicit shared-state identity.
    pub fn shared(
        family: u64,
        inputs: impl IntoIterator<Item = ComputeNodeId>,
        parameters: impl IntoIterator<Item = u64>,
    ) -> Self {
        Self::Shared {
            family,
            inputs: inputs.into_iter().collect(),
            parameters: parameters.into_iter().collect(),
        }
    }
}

/// Precompiled mapping from stateful compute nodes to compact state slots.
///
/// The layout is compiled once from a [`ComputePlan`]. Executors can then keep
/// the layout beside the plan and access long-lived state by integer slot instead
/// of performing operation-name or variable-name lookup on every bar.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanStateLayout {
    slots: BTreeMap<ComputeNodeId, StateSlot>,
    slot_count: usize,
}

impl PlanStateLayout {
    /// Assign one stable slot to every stateful node in execution order.
    ///
    /// This compatibility path deliberately does not infer sharing from string
    /// operation names. Frontend planners must use [`Self::compile_with_keys`]
    /// once they have exact family and parameter identity available.
    pub fn compile(plan: &ComputePlan) -> Self {
        Self::compile_with_keys(plan, |node_id, _| StateKey::unique(node_id))
    }

    /// Compile a layout with explicit semantic state interning.
    ///
    /// The callback runs once at plan compilation and may inspect semantic node
    /// metadata. Hot execution receives only the resulting [`StateSlot`] values.
    /// Nodes that return equal [`StateKey`] values share one persistent state slot.
    pub fn compile_with_keys(
        plan: &ComputePlan,
        mut key_for: impl FnMut(ComputeNodeId, &ComputeNode) -> StateKey,
    ) -> Self {
        let mut slots = BTreeMap::new();
        let mut interned = BTreeMap::<StateKey, StateSlot>::new();
        for &node_id in plan.execution_order() {
            let node = plan
                .node(node_id)
                .expect("execution order only contains compiled nodes");
            if node.capabilities.stateful {
                let key = key_for(node_id, node);
                let slot = match interned.get(&key).copied() {
                    Some(slot) => slot,
                    None => {
                        let slot = StateSlot(interned.len());
                        interned.insert(key, slot);
                        slot
                    }
                };
                slots.insert(node_id, slot);
            }
        }
        let slot_count = interned.len();
        Self { slots, slot_count }
    }

    /// Return the state slot assigned to a compute node, if it is stateful.
    pub fn slot(&self, node: ComputeNodeId) -> Option<StateSlot> {
        self.slots.get(&node).copied()
    }

    /// Number of persistent slots required by the plan after interning.
    pub const fn slot_count(&self) -> usize {
        self.slot_count
    }

    /// Provision an arena for this layout without constructing any state.
    pub fn prepare(&self, arena: &mut StateArena) {
        arena.reserve_slots(self.slot_count);
    }
}

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
        let type_id = entry.type_id;
        match entry.value.downcast::<T>() {
            Ok(value) => {
                self.live -= 1;
                Some(*value)
            }
            Err(value) => {
                self.entries[slot.0] = Some(StateEntry { type_id, value });
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
    use crate::compute::{ComputeCapabilities, ComputeEffect, ComputeNode, LookbackRequirement};

    #[derive(Debug, PartialEq)]
    struct RollingSum {
        sum: f64,
    }

    fn capabilities(stateful: bool) -> ComputeCapabilities {
        ComputeCapabilities {
            deterministic: true,
            streaming: true,
            stateful,
            lookback: LookbackRequirement::None,
            effect: if stateful {
                ComputeEffect::Stateful
            } else {
                ComputeEffect::Pure
            },
        }
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
    fn plan_layout_assigns_slots_only_to_stateful_nodes() {
        let plan = ComputePlan::compile([
            ComputeNode::new(ComputeNodeId(1), "CLOSE", vec![], capabilities(false)),
            ComputeNode::new(
                ComputeNodeId(2),
                "ROLLING_SUM",
                vec![ComputeNodeId(1)],
                capabilities(true),
            ),
            ComputeNode::new(
                ComputeNodeId(3),
                "EMA",
                vec![ComputeNodeId(1)],
                capabilities(true),
            ),
        ])
        .unwrap();
        let layout = PlanStateLayout::compile(&plan);

        assert_eq!(layout.slot(ComputeNodeId(1)), None);
        assert_eq!(layout.slot(ComputeNodeId(2)), Some(StateSlot(0)));
        assert_eq!(layout.slot(ComputeNodeId(3)), Some(StateSlot(1)));
        assert_eq!(layout.slot_count(), 2);

        let mut arena = StateArena::new();
        layout.prepare(&mut arena);
        assert_eq!(arena.slot_capacity(), 2);
        assert!(arena.is_empty());
    }

    #[test]
    fn explicit_state_keys_intern_identical_family_state() {
        let plan = ComputePlan::compile([
            ComputeNode::new(ComputeNodeId(0), "CLOSE", vec![], capabilities(false)),
            ComputeNode::new(
                ComputeNodeId(1),
                "PLUS_DI",
                vec![ComputeNodeId(0)],
                capabilities(true),
            ),
            ComputeNode::new(
                ComputeNodeId(2),
                "MINUS_DI",
                vec![ComputeNodeId(0)],
                capabilities(true),
            ),
        ])
        .unwrap();

        let layout = PlanStateLayout::compile_with_keys(&plan, |node_id, node| {
            if node_id == ComputeNodeId(1) || node_id == ComputeNodeId(2) {
                StateKey::shared(0x444d49, node.dependencies.iter().copied(), [14])
            } else {
                StateKey::unique(node_id)
            }
        });

        assert_eq!(layout.slot(ComputeNodeId(1)), Some(StateSlot(0)));
        assert_eq!(layout.slot(ComputeNodeId(2)), Some(StateSlot(0)));
        assert_eq!(layout.slot_count(), 1);
    }

    #[test]
    fn state_key_parameters_prevent_incorrect_aliasing() {
        let plan = ComputePlan::compile([
            ComputeNode::new(ComputeNodeId(0), "CLOSE", vec![], capabilities(false)),
            ComputeNode::new(
                ComputeNodeId(1),
                "EMA12",
                vec![ComputeNodeId(0)],
                capabilities(true),
            ),
            ComputeNode::new(
                ComputeNodeId(2),
                "EMA26",
                vec![ComputeNodeId(0)],
                capabilities(true),
            ),
        ])
        .unwrap();

        let layout = PlanStateLayout::compile_with_keys(&plan, |node_id, node| match node_id.0 {
            1 => StateKey::shared(0x454d41, node.dependencies.iter().copied(), [12]),
            2 => StateKey::shared(0x454d41, node.dependencies.iter().copied(), [26]),
            _ => StateKey::unique(node_id),
        });

        assert_eq!(layout.slot(ComputeNodeId(1)), Some(StateSlot(0)));
        assert_eq!(layout.slot(ComputeNodeId(2)), Some(StateSlot(1)));
        assert_eq!(layout.slot_count(), 2);
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
        assert_eq!(
            arena.remove::<RollingSum>(slot),
            Some(RollingSum { sum: 7.0 })
        );
        assert!(arena.is_empty());
    }
}
