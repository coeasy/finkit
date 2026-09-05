//! Architecture v3 hot execution plan.
//!
//! [`crate::compute::ComputePlan`] is the logical/semantic DAG. It intentionally
//! keeps human-readable operation labels for diagnostics, optimization and
//! tooling. This module is the execution boundary: operation labels are parsed
//! exactly once while compiling a [`HotExecutionPlan`], then discarded from the
//! hot nodes. Runtime executors address kernels, temporary buffers and persistent
//! state only through compact numeric ids.

use crate::buffer_arena::{BufferSlot, PlanBufferLayout};
use crate::compute::{ComputeNodeId, ComputePlan};
use crate::state_arena::{PlanStateLayout, StateSlot};
use std::collections::BTreeMap;
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
}

impl fmt::Display for HotPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KernelCollision { id, first, second } => write!(
                f,
                "kernel id collision {:#x}: {first} vs {second}",
                id.0
            ),
            Self::MissingBufferSlot(node) => {
                write!(f, "missing physical buffer slot for compute node {}", node.0)
            }
        }
    }
}

impl std::error::Error for HotPlanError {}

/// Immutable Architecture v3 execution plan containing numeric runtime data only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotExecutionPlan {
    nodes: Vec<HotNode>,
    buffer_layout: PlanBufferLayout,
    state_layout: PlanStateLayout,
}

impl HotExecutionPlan {
    /// Compile a semantic plan into numeric Kernel/Buffer/State slots.
    ///
    /// Human-readable operation labels are consumed here and are not copied
    /// into the returned hot plan. `retained` identifies outputs that must stay
    /// live until execution completes.
    pub fn compile(
        plan: &ComputePlan,
        retained: impl IntoIterator<Item = ComputeNodeId>,
    ) -> Result<Self, HotPlanError> {
        let retained: Vec<_> = retained.into_iter().collect();
        let buffer_layout = PlanBufferLayout::compile(plan, retained.iter().copied());
        let state_layout = PlanStateLayout::compile(plan);

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

            nodes.push(HotNode {
                node: node_id,
                kernel,
                inputs,
                output,
                state: state_layout.slot(node_id),
            });
        }

        Ok(Self {
            nodes,
            buffer_layout,
            state_layout,
        })
    }

    /// Numeric instructions in deterministic topological order.
    pub fn nodes(&self) -> &[HotNode] {
        &self.nodes
    }

    /// Compile-time physical scratch-buffer layout.
    pub const fn buffer_layout(&self) -> &PlanBufferLayout {
        &self.buffer_layout
    }

    /// Compile-time persistent-state layout.
    pub const fn state_layout(&self) -> &PlanStateLayout {
        &self.state_layout
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
            ComputeNode::new(ComputeNodeId(0), "VARIABLE:CLOSE", vec![], capabilities(false)),
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
        assert_eq!(hot.nodes()[2].inputs, vec![hot.nodes()[1].output]);
        assert_eq!(hot.buffer_layout().slot_count(), 2);
        assert_eq!(hot.state_layout().slot_count(), 1);
    }

    #[test]
    fn same_operation_reuses_kernel_id_without_retaining_string_dispatch() {
        let plan = ComputePlan::compile([
            ComputeNode::new(ComputeNodeId(0), "VARIABLE:CLOSE", vec![], capabilities(false)),
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
}
