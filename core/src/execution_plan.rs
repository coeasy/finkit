//! Architecture v3 hot execution plan.
//!
//! [`crate::compute::ComputePlan`] is the logical/semantic DAG. It intentionally
//! keeps human-readable operation labels for diagnostics, optimization and
//! tooling. This module is the execution boundary: operation labels are parsed
//! exactly once while compiling a [`HotExecutionPlan`], then discarded from the
//! hot nodes. Runtime executors address kernels, inputs, parameters, temporary
//! buffers and persistent state only through compact numeric ids.

use crate::buffer_arena::{BufferSlot, PlanBufferLayout};
use crate::compute::{ComputeNodeId, ComputePlan};
use crate::state_arena::{PlanStateLayout, StateSlot};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Stable numeric kernel identifier used by the hot execution layer.
///
/// The id is the FNV-1a hash of the canonical semantic operation. Compilation
/// verifies that two distinct operation labels never collide inside one plan.
/// The original strings are not retained by [`HotExecutionPlan`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KernelId(pub u64);

impl KernelId {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    /// Hash a static canonical operation into a kernel id.
    pub const fn from_static(operation: &str) -> Self {
        let bytes = operation.as_bytes();
        let mut hash = Self::FNV_OFFSET;
        let mut index = 0usize;
        while index < bytes.len() {
            hash ^= bytes[index] as u64;
            hash = hash.wrapping_mul(Self::FNV_PRIME);
            index += 1;
        }
        Self(hash)
    }

    /// Hash a compile-time semantic operation into a kernel id.
    pub fn compile(operation: &str) -> Self {
        Self::from_static(operation)
    }
}

/// Compact external-input address used by executors after frontend binding.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputSlot(pub usize);

/// Compact parameter address into a [`ParameterArena`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterSlot(pub usize);

/// Eq-safe scalar parameter payload stored outside hot instructions.
///
/// Floating-point values use their exact IEEE-754 bit representation so plan
/// equality, CSE keys and cache keys never depend on partial `f64` equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParameterValue {
    /// Exact IEEE-754 `f64` bits.
    F64Bits(u64),
    /// Non-negative integer parameter such as a lookback period.
    Usize(usize),
}

impl ParameterValue {
    /// Encode an `f64` without losing NaN payload or signed-zero identity.
    pub const fn from_f64(value: f64) -> Self {
        Self::F64Bits(value.to_bits())
    }

    /// Decode an `f64` parameter when this value contains floating-point bits.
    pub const fn as_f64(self) -> Option<f64> {
        match self {
            Self::F64Bits(bits) => Some(f64::from_bits(bits)),
            Self::Usize(_) => None,
        }
    }

    /// Return the integer parameter when present.
    pub const fn as_usize(self) -> Option<usize> {
        match self {
            Self::Usize(value) => Some(value),
            Self::F64Bits(_) => None,
        }
    }
}

/// Contiguous parameter range consumed by one hot instruction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterRange {
    /// First parameter slot.
    pub start: ParameterSlot,
    /// Number of consecutive parameter values.
    pub len: usize,
}

impl ParameterRange {
    /// Empty parameter range used by kernels without scalar parameters.
    pub const EMPTY: Self = Self {
        start: ParameterSlot(0),
        len: 0,
    };

    /// Whether this range contains no parameters.
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }
}

/// Immutable scalar parameter storage shared by all hot instructions in a plan.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParameterArena {
    values: Vec<ParameterValue>,
}

impl ParameterArena {
    /// Create an empty parameter arena.
    pub const fn new() -> Self {
        Self { values: Vec::new() }
    }

    /// Append one parameter and return its numeric slot.
    pub fn push(&mut self, value: ParameterValue) -> ParameterSlot {
        let slot = ParameterSlot(self.values.len());
        self.values.push(value);
        slot
    }

    /// Append one contiguous parameter group and return its range.
    pub fn extend(
        &mut self,
        values: impl IntoIterator<Item = ParameterValue>,
    ) -> ParameterRange {
        let start = ParameterSlot(self.values.len());
        self.values.extend(values);
        ParameterRange {
            start,
            len: self.values.len() - start.0,
        }
    }

    /// Read one parameter by numeric slot.
    pub fn get(&self, slot: ParameterSlot) -> Option<ParameterValue> {
        self.values.get(slot.0).copied()
    }

    /// Read one precompiled contiguous parameter range.
    pub fn range(&self, range: ParameterRange) -> Option<&[ParameterValue]> {
        let end = range.start.0.checked_add(range.len)?;
        self.values.get(range.start.0..end)
    }

    /// Number of scalar parameters stored by the plan.
    pub const fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether no scalar parameters are stored.
    pub const fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Numeric binding layout for external formula/runtime inputs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InputLayout {
    slots: BTreeMap<ComputeNodeId, InputSlot>,
}

impl InputLayout {
    fn compile(plan: &ComputePlan) -> Self {
        let mut slots = BTreeMap::new();
        for &node_id in plan.execution_order() {
            let node = plan
                .node(node_id)
                .expect("execution order only contains compiled nodes");
            // Variable-name parsing is deliberately compile-time only. The hot
            // plan retains only the numeric node->input-slot relation.
            if node.operation.starts_with("VARIABLE:") {
                let slot = InputSlot(slots.len());
                slots.insert(node_id, slot);
            }
        }
        Self { slots }
    }

    /// Resolve one semantic source node to its external input slot.
    pub fn slot(&self, node: ComputeNodeId) -> Option<InputSlot> {
        self.slots.get(&node).copied()
    }

    /// Number of external input slots required by the plan.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether this plan has no external inputs.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

/// Numeric layout of retained public outputs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OutputLayout {
    outputs: Vec<(ComputeNodeId, BufferSlot)>,
}

impl OutputLayout {
    fn compile(retained: &[ComputeNodeId], buffers: &PlanBufferLayout) -> Result<Self, HotPlanError> {
        let outputs = retained
            .iter()
            .copied()
            .map(|node| {
                buffers
                    .slot(node)
                    .map(|slot| (node, slot))
                    .ok_or(HotPlanError::MissingBufferSlot(node))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { outputs })
    }

    /// Retained outputs in frontend-requested order.
    pub fn outputs(&self) -> &[(ComputeNodeId, BufferSlot)] {
        &self.outputs
    }

    /// Number of retained outputs.
    pub fn len(&self) -> usize {
        self.outputs.len()
    }

    /// Whether no outputs were retained.
    pub fn is_empty(&self) -> bool {
        self.outputs.is_empty()
    }
}

/// Last hot-step index at which each semantic dependency must remain readable.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DependencyLifetime {
    last_use: BTreeMap<ComputeNodeId, usize>,
}

impl DependencyLifetime {
    fn compile(plan: &ComputePlan, retained: &[ComputeNodeId]) -> Self {
        let mut last_use = BTreeMap::new();
        for (step, &node_id) in plan.execution_order().iter().enumerate() {
            last_use.entry(node_id).or_insert(step);
            let node = plan
                .node(node_id)
                .expect("execution order only contains compiled nodes");
            for dependency in &node.dependencies {
                last_use.insert(*dependency, step);
            }
        }
        let retained_step = plan.len();
        for &node in retained {
            last_use.insert(node, retained_step);
        }
        Self { last_use }
    }

    /// Last step that consumes `node`, or the plan length for retained outputs.
    pub fn last_use(&self, node: ComputeNodeId) -> Option<usize> {
        self.last_use.get(&node).copied()
    }
}

/// Compact execution summary used for arena provisioning and diagnostics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExecutionMetadata {
    /// Number of numeric hot instructions.
    pub steps: usize,
    /// Number of external inputs.
    pub inputs: usize,
    /// Number of retained outputs.
    pub outputs: usize,
    /// Number of physical scratch buffers.
    pub buffers: usize,
    /// Number of persistent state slots.
    pub states: usize,
    /// Number of scalar parameters.
    pub parameters: usize,
}

/// One numeric hot-path instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotNode {
    /// Semantic DAG node represented by this instruction.
    pub node: ComputeNodeId,
    /// Pre-resolved numeric kernel id.
    pub kernel: KernelId,
    /// Physical buffer slots holding dependency values, in dependency order.
    pub inputs: Vec<BufferSlot>,
    /// Physical slot written by this instruction.
    pub output: BufferSlot,
    /// Optional persistent state slot for stateful kernels.
    pub state: Option<StateSlot>,
    /// Contiguous scalar parameter payload for this kernel.
    pub parameters: ParameterRange,
}

/// Errors produced while lowering a semantic DAG into the numeric hot plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotPlanError {
    /// Two distinct operation labels produced the same numeric kernel id.
    KernelCollision {
        /// Colliding numeric id.
        id: KernelId,
        /// First canonical operation encountered at compile time.
        first: String,
        /// Second canonical operation encountered at compile time.
        second: String,
    },
    /// A compiled node unexpectedly has no buffer slot.
    MissingBufferSlot(ComputeNodeId),
    /// A node refers to a parameter range outside the immutable arena.
    InvalidParameterRange {
        /// Node owning the invalid range.
        node: ComputeNodeId,
        /// Invalid range.
        range: ParameterRange,
        /// Number of values available in the arena.
        arena_len: usize,
    },
}

impl fmt::Display for HotPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KernelCollision { id, first, second } => {
                write!(f, "kernel id collision {:#x}: {first} vs {second}", id.0)
            }
            Self::MissingBufferSlot(node) => {
                write!(
                    f,
                    "missing physical buffer slot for compute node {}",
                    node.0
                )
            }
            Self::InvalidParameterRange {
                node,
                range,
                arena_len,
            } => write!(
                f,
                "parameter range {}..{} for compute node {} exceeds arena length {}",
                range.start.0,
                range.start.0.saturating_add(range.len),
                node.0,
                arena_len
            ),
        }
    }
}

impl std::error::Error for HotPlanError {}

/// Immutable Architecture v3 execution plan containing numeric runtime data only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotExecutionPlan {
    nodes: Vec<HotNode>,
    input_layout: InputLayout,
    output_layout: OutputLayout,
    buffer_layout: PlanBufferLayout,
    state_layout: PlanStateLayout,
    parameter_arena: ParameterArena,
    dependency_lifetime: DependencyLifetime,
    metadata: ExecutionMetadata,
}

impl HotExecutionPlan {
    /// Compile a semantic plan into numeric Kernel/Input/Buffer/State slots.
    ///
    /// Human-readable operation labels are consumed here and are not copied
    /// into the returned hot plan. `retained` identifies outputs that must stay
    /// live until execution completes.
    pub fn compile(
        plan: &ComputePlan,
        retained: impl IntoIterator<Item = ComputeNodeId>,
    ) -> Result<Self, HotPlanError> {
        Self::compile_with_parameters(plan, retained, ParameterArena::new(), BTreeMap::new())
    }

    /// Compile a semantic plan while attaching pre-lowered scalar parameters.
    ///
    /// Frontend compilers can use this entrypoint after lowering literals and
    /// registry parameters. The resulting hot instructions carry only numeric
    /// [`ParameterRange`] values; no string lookup is required at execution time.
    pub fn compile_with_parameters(
        plan: &ComputePlan,
        retained: impl IntoIterator<Item = ComputeNodeId>,
        parameter_arena: ParameterArena,
        parameter_ranges: BTreeMap<ComputeNodeId, ParameterRange>,
    ) -> Result<Self, HotPlanError> {
        let retained: Vec<_> = retained.into_iter().collect();
        let retained_set: BTreeSet<_> = retained.iter().copied().collect();
        let buffer_layout = PlanBufferLayout::compile(plan, retained.iter().copied());
        let state_layout = PlanStateLayout::compile(plan);
        let input_layout = InputLayout::compile(plan);
        let output_layout = OutputLayout::compile(&retained, &buffer_layout)?;
        let dependency_lifetime = DependencyLifetime::compile(plan, &retained);

        // Collision checking exists only in this compile phase. The resulting
        // plan retains no operation-name map, keeping string work off hot paths.
        let mut seen_kernels: BTreeMap<KernelId, &str> = BTreeMap::new();
        let mut nodes = Vec::with_capacity(plan.len());
        for &node_id in plan.execution_order() {
            let semantic = plan
                .node(node_id)
                .expect("execution order only contains compiled nodes");
            let kernel = KernelId::compile(&semantic.operation);
            if let Some(previous) = seen_kernels.insert(kernel, semantic.operation.as_str()) {
                if previous != semantic.operation {
                    return Err(HotPlanError::KernelCollision {
                        id: kernel,
                        first: previous.to_string(),
                        second: semantic.operation.clone(),
                    });
                }
            }

            let output = buffer_layout
                .slot(node_id)
                .ok_or(HotPlanError::MissingBufferSlot(node_id))?;
            let inputs = semantic
                .dependencies
                .iter()
                .map(|dependency| {
                    buffer_layout
                        .slot(*dependency)
                        .ok_or(HotPlanError::MissingBufferSlot(*dependency))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let parameters = parameter_ranges
                .get(&node_id)
                .copied()
                .unwrap_or(ParameterRange::EMPTY);
            if parameter_arena.range(parameters).is_none() {
                return Err(HotPlanError::InvalidParameterRange {
                    node: node_id,
                    range: parameters,
                    arena_len: parameter_arena.len(),
                });
            }

            nodes.push(HotNode {
                node: node_id,
                kernel,
                inputs,
                output,
                state: state_layout.slot(node_id),
                parameters,
            });
        }

        debug_assert!(retained_set
            .iter()
            .all(|node| dependency_lifetime.last_use(*node) == Some(plan.len())));

        let metadata = ExecutionMetadata {
            steps: nodes.len(),
            inputs: input_layout.len(),
            outputs: output_layout.len(),
            buffers: buffer_layout.slot_count(),
            states: state_layout.slot_count(),
            parameters: parameter_arena.len(),
        };

        Ok(Self {
            nodes,
            input_layout,
            output_layout,
            buffer_layout,
            state_layout,
            parameter_arena,
            dependency_lifetime,
            metadata,
        })
    }

    /// Numeric instructions in deterministic topological order.
    pub fn nodes(&self) -> &[HotNode] {
        &self.nodes
    }

    /// External-input binding layout.
    pub const fn input_layout(&self) -> &InputLayout {
        &self.input_layout
    }

    /// Retained-output layout.
    pub const fn output_layout(&self) -> &OutputLayout {
        &self.output_layout
    }

    /// Compile-time physical scratch-buffer layout.
    pub const fn buffer_layout(&self) -> &PlanBufferLayout {
        &self.buffer_layout
    }

    /// Compile-time persistent-state layout.
    pub const fn state_layout(&self) -> &PlanStateLayout {
        &self.state_layout
    }

    /// Immutable scalar parameters consumed by hot kernels.
    pub const fn parameter_arena(&self) -> &ParameterArena {
        &self.parameter_arena
    }

    /// Compile-time dependency lifetime metadata.
    pub const fn dependency_lifetime(&self) -> &DependencyLifetime {
        &self.dependency_lifetime
    }

    /// Compact plan summary for executor provisioning.
    pub const fn metadata(&self) -> ExecutionMetadata {
        self.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compute::{ComputeCapabilities, ComputeEffect, ComputeNode, LookbackRequirement};

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
    fn hot_plan_contains_only_numeric_runtime_addresses() {
        let plan = ComputePlan::compile([
            ComputeNode::new(
                ComputeNodeId(0),
                "VARIABLE:CLOSE",
                vec![],
                capabilities(false),
            ),
            ComputeNode::new(
                ComputeNodeId(1),
                "CALL:EMA",
                vec![ComputeNodeId(0)],
                capabilities(true),
            ),
            ComputeNode::new(
                ComputeNodeId(2),
                "CALL:ROC",
                vec![ComputeNodeId(1)],
                capabilities(false),
            ),
        ])
        .unwrap();

        let hot = HotExecutionPlan::compile(&plan, [ComputeNodeId(2)]).unwrap();
        assert_eq!(hot.nodes().len(), 3);
        assert_eq!(hot.nodes()[1].kernel, KernelId::from_static("CALL:EMA"));
        assert_eq!(hot.nodes()[1].state, Some(StateSlot(0)));
        assert_eq!(hot.nodes()[1].parameters, ParameterRange::EMPTY);
        assert_eq!(hot.nodes()[2].inputs, vec![hot.nodes()[1].output]);
        assert_eq!(hot.input_layout().slot(ComputeNodeId(0)), Some(InputSlot(0)));
        assert_eq!(hot.output_layout().outputs(), &[(ComputeNodeId(2), hot.nodes()[2].output)]);
        assert_eq!(hot.buffer_layout().slot_count(), 2);
        assert_eq!(hot.state_layout().slot_count(), 1);
        assert_eq!(hot.dependency_lifetime().last_use(ComputeNodeId(2)), Some(3));
        assert_eq!(
            hot.metadata(),
            ExecutionMetadata {
                steps: 3,
                inputs: 1,
                outputs: 1,
                buffers: 2,
                states: 1,
                parameters: 0,
            }
        );
    }

    #[test]
    fn same_operation_reuses_kernel_id_without_retaining_string_dispatch() {
        let plan = ComputePlan::compile([
            ComputeNode::new(
                ComputeNodeId(0),
                "VARIABLE:CLOSE",
                vec![],
                capabilities(false),
            ),
            ComputeNode::new(
                ComputeNodeId(1),
                "CALL:EMA",
                vec![ComputeNodeId(0)],
                capabilities(false),
            ),
            ComputeNode::new(
                ComputeNodeId(2),
                "CALL:EMA",
                vec![ComputeNodeId(0)],
                capabilities(false),
            ),
        ])
        .unwrap();

        let hot = HotExecutionPlan::compile(&plan, [ComputeNodeId(1), ComputeNodeId(2)]).unwrap();
        assert_eq!(hot.nodes()[1].kernel, hot.nodes()[2].kernel);
    }

    #[test]
    fn exact_parameters_are_numeric_and_prebound() {
        let plan = ComputePlan::compile([
            ComputeNode::new(
                ComputeNodeId(0),
                "VARIABLE:CLOSE",
                vec![],
                capabilities(false),
            ),
            ComputeNode::new(
                ComputeNodeId(1),
                "CALL:EMA",
                vec![ComputeNodeId(0)],
                capabilities(true),
            ),
        ])
        .unwrap();

        let mut arena = ParameterArena::new();
        let range = arena.extend([
            ParameterValue::Usize(14),
            ParameterValue::from_f64(-0.0),
        ]);
        let hot = HotExecutionPlan::compile_with_parameters(
            &plan,
            [ComputeNodeId(1)],
            arena,
            BTreeMap::from([(ComputeNodeId(1), range)]),
        )
        .unwrap();

        assert_eq!(hot.nodes()[1].parameters, range);
        let params = hot.parameter_arena().range(range).unwrap();
        assert_eq!(params[0].as_usize(), Some(14));
        assert_eq!(params[1].as_f64().unwrap().to_bits(), (-0.0f64).to_bits());
        assert_eq!(hot.metadata().parameters, 2);
    }

    #[test]
    fn invalid_parameter_range_is_rejected_during_compile() {
        let plan = ComputePlan::compile([ComputeNode::new(
            ComputeNodeId(0),
            "VARIABLE:CLOSE",
            vec![],
            capabilities(false),
        )])
        .unwrap();
        let error = HotExecutionPlan::compile_with_parameters(
            &plan,
            [ComputeNodeId(0)],
            ParameterArena::new(),
            BTreeMap::from([(
                ComputeNodeId(0),
                ParameterRange {
                    start: ParameterSlot(0),
                    len: 1,
                },
            )]),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            HotPlanError::InvalidParameterRange {
                node: ComputeNodeId(0),
                arena_len: 0,
                ..
            }
        ));
    }
}
