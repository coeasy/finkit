//! Numeric Architecture v3 executor shared by batch, formula, factor and streaming frontends.
//!
//! Frontends compile semantic work once into [`HotExecutionPlan`]. This executor
//! then runs only numeric kernel/input/buffer/state/parameter addresses. Kernel
//! implementations are supplied through [`KernelDispatcher`], keeping runtime
//! dispatch independent from formula strings or registry hash maps.

use crate::buffer_arena::{BufferArena, BufferArenaConfig, BufferArenaStats, BufferSlot};
use crate::execution_plan::{HotExecutionPlan, KernelId, ParameterValue};
use crate::state_arena::{StateArena, StateSlot};
use std::fmt;
use std::ops::Range;

/// One pre-resolved kernel invocation presented to a numeric dispatcher.
#[derive(Debug, Clone, Copy)]
pub struct KernelCall<'a> {
    /// Numeric kernel identifier.
    pub kernel: KernelId,
    /// Physical input buffers in semantic dependency order.
    pub inputs: &'a [BufferSlot],
    /// Physical output buffer that the kernel must fully write.
    pub output: BufferSlot,
    /// Immutable scalar parameters prebound by the frontend compiler.
    pub parameters: &'a [ParameterValue],
    /// Optional persistent state slot.
    pub state: Option<StateSlot>,
}

/// Compact dispatcher error suitable for a string-free hot loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelDispatchError {
    /// Dispatcher-defined stable numeric error code.
    pub code: u32,
}

impl KernelDispatchError {
    /// Construct an error with a stable dispatcher-defined code.
    pub const fn new(code: u32) -> Self {
        Self { code }
    }
}

impl fmt::Display for KernelDispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "kernel dispatch failed with code {}", self.code)
    }
}

impl std::error::Error for KernelDispatchError {}

/// Numeric kernel backend consumed by [`UnifiedExecutor`].
///
/// Dispatchers receive physical slots rather than borrowed input/output slices
/// so they can use `split_at_mut` (or family-specific fused kernels) without the
/// executor creating temporary vectors of references on every instruction.
pub trait KernelDispatcher {
    /// Execute one numeric instruction and fully write `call.output`.
    fn dispatch(
        &mut self,
        call: KernelCall<'_>,
        buffers: &mut [Vec<f64>],
        states: &mut StateArena,
    ) -> Result<(), KernelDispatchError>;
}

/// Errors produced by unified plan execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecuteError {
    /// Full-series execution cannot infer a logical length without inputs.
    NoInputs,
    /// Number of bound input arrays does not match the compiled input layout.
    InputCount {
        /// Expected numeric input slots.
        expected: usize,
        /// Arrays supplied by the caller.
        actual: usize,
    },
    /// Bound inputs have different logical lengths.
    InputLength {
        /// Slot containing the mismatched input.
        slot: usize,
        /// Expected common length.
        expected: usize,
        /// Actual length for this slot.
        actual: usize,
    },
    /// Requested execution range falls outside the bound input extent.
    InvalidRange {
        /// Requested start index.
        start: usize,
        /// Requested exclusive end index.
        end: usize,
        /// Available input length.
        len: usize,
    },
    /// A hot plan referenced a parameter range that was not present.
    MissingParameters,
    /// A retained output unexpectedly aliases another retained output slot.
    AliasedOutput(BufferSlot),
    /// Numeric kernel dispatch failed.
    Kernel(KernelDispatchError),
}

impl From<KernelDispatchError> for ExecuteError {
    fn from(value: KernelDispatchError) -> Self {
        Self::Kernel(value)
    }
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoInputs => write!(f, "cannot infer execution length without bound inputs"),
            Self::InputCount { expected, actual } => {
                write!(f, "expected {expected} input arrays, received {actual}")
            }
            Self::InputLength {
                slot,
                expected,
                actual,
            } => write!(
                f,
                "input slot {slot} has length {actual}, expected {expected}"
            ),
            Self::InvalidRange { start, end, len } => {
                write!(f, "execution range {start}..{end} exceeds input length {len}")
            }
            Self::MissingParameters => write!(f, "hot node referenced missing parameters"),
            Self::AliasedOutput(slot) => {
                write!(f, "retained outputs alias physical buffer slot {}", slot.0)
            }
            Self::Kernel(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ExecuteError {}

/// Output buffers returned by one unified execution.
///
/// Buffers are moved out of the scratch arena instead of copied. Callers own
/// them and may pass them to another frontend or FFI layer directly.
#[derive(Debug, Default, PartialEq)]
pub struct ExecutionOutput {
    /// Retained outputs in the plan's frontend-requested order.
    pub values: Vec<Vec<f64>>,
}

impl ExecutionOutput {
    /// Number of retained outputs.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether no outputs were retained.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Reusable Architecture v3 numeric executor.
///
/// The object owns persistent state and a bounded buffer arena across calls.
/// `execute_last` intentionally keeps state for streaming use. Call [`Self::reset`]
/// when starting a logically independent series.
pub struct UnifiedExecutor<D> {
    plan: HotExecutionPlan,
    dispatcher: D,
    buffers: BufferArena,
    states: StateArena,
}

impl<D: KernelDispatcher> UnifiedExecutor<D> {
    /// Construct an executor with the default bounded buffer arena.
    pub fn new(plan: HotExecutionPlan, dispatcher: D) -> Self {
        Self::with_buffer_config(plan, dispatcher, BufferArenaConfig::default())
    }

    /// Construct an executor with explicit buffer-retention limits.
    pub fn with_buffer_config(
        plan: HotExecutionPlan,
        dispatcher: D,
        buffer_config: BufferArenaConfig,
    ) -> Self {
        let mut states = StateArena::new();
        plan.state_layout().prepare(&mut states);
        Self {
            plan,
            dispatcher,
            buffers: BufferArena::new(buffer_config),
            states,
        }
    }

    /// Execute the complete bound input extent.
    ///
    /// Persistent state is preserved across calls; call [`Self::reset`] before
    /// beginning an independent batch series.
    pub fn execute(&mut self, inputs: &[&[f64]]) -> Result<ExecutionOutput, ExecuteError> {
        let len = inputs.first().map(|input| input.len()).ok_or(ExecuteError::NoInputs)?;
        self.execute_range(inputs, 0..len)
    }

    /// Execute one explicit half-open input range.
    pub fn execute_range(
        &mut self,
        inputs: &[&[f64]],
        range: Range<usize>,
    ) -> Result<ExecutionOutput, ExecuteError> {
        let common_len = validate_inputs(&self.plan, inputs)?;
        if range.start > range.end || range.end > common_len {
            return Err(ExecuteError::InvalidRange {
                start: range.start,
                end: range.end,
                len: common_len,
            });
        }
        self.run(inputs, range)
    }

    /// Execute only the final bound sample while preserving persistent state.
    ///
    /// This is the streaming/`eval_last` path. Stateful dispatchers update the
    /// same [`StateArena`] slots on every call.
    pub fn execute_last(
        &mut self,
        inputs: &[&[f64]],
    ) -> Result<ExecutionOutput, ExecuteError> {
        let common_len = validate_inputs(&self.plan, inputs)?;
        if common_len == 0 {
            return Err(ExecuteError::InvalidRange {
                start: 0,
                end: 1,
                len: 0,
            });
        }
        self.run(inputs, common_len - 1..common_len)
    }

    fn run(
        &mut self,
        inputs: &[&[f64]],
        range: Range<usize>,
    ) -> Result<ExecutionOutput, ExecuteError> {
        let logical_len = range.end - range.start;
        let mut buffers = self
            .plan
            .buffer_layout()
            .take_buffers(&mut self.buffers, logical_len);

        let result = (|| {
            for node in self.plan.nodes() {
                if let Some(input_slot) = self.plan.input_layout().slot(node.node) {
                    let source = &inputs[input_slot.0][range.clone()];
                    buffers[node.output.0].copy_from_slice(source);
                    continue;
                }

                let parameters = self
                    .plan
                    .parameter_arena()
                    .range(node.parameters)
                    .ok_or(ExecuteError::MissingParameters)?;
                self.dispatcher.dispatch(
                    KernelCall {
                        kernel: node.kernel,
                        inputs: &node.inputs,
                        output: node.output,
                        parameters,
                        state: node.state,
                    },
                    &mut buffers,
                    &mut self.states,
                )?;
            }

            let mut seen = vec![false; buffers.len()];
            let mut values = Vec::with_capacity(self.plan.output_layout().len());
            for &(_, slot) in self.plan.output_layout().outputs() {
                if seen[slot.0] {
                    return Err(ExecuteError::AliasedOutput(slot));
                }
                seen[slot.0] = true;
                values.push(std::mem::take(&mut buffers[slot.0]));
            }
            Ok(ExecutionOutput { values })
        })();

        self.plan
            .buffer_layout()
            .recycle_buffers(&mut self.buffers, buffers);
        result
    }

    /// Drop persistent kernel state while retaining allocated slot capacity and
    /// reusable scratch buffers.
    pub fn reset(&mut self) {
        self.states.clear();
        self.plan.state_layout().prepare(&mut self.states);
    }

    /// Replace the precompiled plan and reset persistent state.
    ///
    /// Scratch-buffer cache allocations remain reusable across compatible lengths.
    pub fn rebind(&mut self, plan: HotExecutionPlan) {
        self.plan = plan;
        self.reset();
    }

    /// Current immutable numeric plan.
    pub const fn plan(&self) -> &HotExecutionPlan {
        &self.plan
    }

    /// Mutable access to the concrete numeric dispatcher.
    pub fn dispatcher_mut(&mut self) -> &mut D {
        &mut self.dispatcher
    }

    /// Persistent state arena used by streaming/stateful kernels.
    pub const fn states(&self) -> &StateArena {
        &self.states
    }

    /// Current buffer allocation/reuse counters.
    pub fn buffer_stats(&self) -> BufferArenaStats {
        self.buffers.stats()
    }
}

fn validate_inputs(plan: &HotExecutionPlan, inputs: &[&[f64]]) -> Result<usize, ExecuteError> {
    let expected = plan.input_layout().len();
    if inputs.len() != expected {
        return Err(ExecuteError::InputCount {
            expected,
            actual: inputs.len(),
        });
    }
    let common_len = inputs.first().map_or(0, |input| input.len());
    for (slot, input) in inputs.iter().enumerate() {
        if input.len() != common_len {
            return Err(ExecuteError::InputLength {
                slot,
                expected: common_len,
                actual: input.len(),
            });
        }
    }
    Ok(common_len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compute::{
        ComputeCapabilities, ComputeEffect, ComputeNode, ComputeNodeId, ComputePlan,
        LookbackRequirement,
    };
    use crate::execution_plan::HotExecutionPlan;

    const COPY_PLUS_ONE: KernelId = KernelId::from_static("COPY_PLUS_ONE");

    fn pure() -> ComputeCapabilities {
        ComputeCapabilities {
            deterministic: true,
            streaming: true,
            stateful: false,
            lookback: LookbackRequirement::None,
            effect: ComputeEffect::Pure,
        }
    }

    struct TestDispatcher;

    impl KernelDispatcher for TestDispatcher {
        fn dispatch(
            &mut self,
            call: KernelCall<'_>,
            buffers: &mut [Vec<f64>],
            _states: &mut StateArena,
        ) -> Result<(), KernelDispatchError> {
            if call.kernel != COPY_PLUS_ONE || call.inputs.len() != 1 {
                return Err(KernelDispatchError::new(1));
            }
            let input = call.inputs[0].0;
            let output = call.output.0;
            if input < output {
                let (left, right) = buffers.split_at_mut(output);
                for (dst, src) in right[0].iter_mut().zip(left[input].iter()) {
                    *dst = *src + 1.0;
                }
            } else {
                let (left, right) = buffers.split_at_mut(input);
                for (dst, src) in left[output].iter_mut().zip(right[0].iter()) {
                    *dst = *src + 1.0;
                }
            }
            Ok(())
        }
    }

    fn plan() -> HotExecutionPlan {
        let semantic = ComputePlan::compile([
            ComputeNode::new(
                ComputeNodeId(0),
                "VARIABLE:CLOSE",
                vec![],
                pure(),
            ),
            ComputeNode::new(
                ComputeNodeId(1),
                "COPY_PLUS_ONE",
                vec![ComputeNodeId(0)],
                pure(),
            ),
        ])
        .unwrap();
        HotExecutionPlan::compile(&semantic, [ComputeNodeId(1)]).unwrap()
    }

    #[test]
    fn execute_and_range_use_numeric_plan_and_recycled_buffers() {
        let mut executor = UnifiedExecutor::new(plan(), TestDispatcher);
        let close = [1.0, 2.0, 3.0, 4.0];

        let full = executor.execute(&[&close]).unwrap();
        assert_eq!(full.values, vec![vec![2.0, 3.0, 4.0, 5.0]]);

        let range = executor.execute_range(&[&close], 1..3).unwrap();
        assert_eq!(range.values, vec![vec![3.0, 4.0]]);
        assert!(executor.buffer_stats().cache_hits > 0);
    }

    #[test]
    fn execute_last_and_rebind_keep_executor_reusable() {
        let mut executor = UnifiedExecutor::new(plan(), TestDispatcher);
        let close = [10.0, 20.0, 30.0];
        let last = executor.execute_last(&[&close]).unwrap();
        assert_eq!(last.values, vec![vec![31.0]]);

        executor.reset();
        executor.rebind(plan());
        let next = executor.execute(&[&close[..2]]).unwrap();
        assert_eq!(next.values, vec![vec![11.0, 21.0]]);
    }

    #[test]
    fn malformed_bindings_are_rejected_before_dispatch() {
        let mut executor = UnifiedExecutor::new(plan(), TestDispatcher);
        assert_eq!(
            executor.execute_range(&[], 0..1).unwrap_err(),
            ExecuteError::InputCount {
                expected: 1,
                actual: 0
            }
        );
    }
}
