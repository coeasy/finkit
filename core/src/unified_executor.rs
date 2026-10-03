//! Numeric Architecture v3 executor shared by batch, formula, factor and streaming frontends.
//!
//! Frontends compile semantic work once into [`HotExecutionPlan`](crate::execution_plan::HotExecutionPlan). This executor
//! then runs only numeric kernel/input/buffer/state/parameter addresses. Kernel
//! implementations are supplied through [`KernelDispatcher`](crate::unified_executor::KernelDispatcher), keeping runtime
//! dispatch independent from formula strings or registry hash maps.

use crate::buffer_arena::{BufferArenaConfig, BufferArenaStats, BufferSlot};
use crate::execution_plan::{HotExecutionPlan, KernelId, ParameterValue};
use crate::runtime_context::{ExecutionKind, ExecutionLimits, RuntimeContext, RuntimeContextError};
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
    /// Kernel that failed, when the executor knows it.
    ///
    /// The dispatcher constructs errors without this and the executor attaches
    /// it at the single call site, so a failure names the exact instruction
    /// instead of only a numeric code.
    pub kernel: Option<KernelId>,
}

impl KernelDispatchError {
    /// Construct an error with a stable dispatcher-defined code.
    pub const fn new(code: u32) -> Self {
        Self { code, kernel: None }
    }

    /// Attach the kernel that produced this error.
    pub const fn with_kernel(mut self, kernel: KernelId) -> Self {
        self.kernel = Some(kernel);
        self
    }
}

impl fmt::Display for KernelDispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kernel {
            Some(kernel) => write!(
                f,
                "kernel dispatch failed for kernel 0x{:016x} with code {}",
                kernel.0, self.code
            ),
            None => write!(f, "kernel dispatch failed with code {}", self.code),
        }
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
    /// Number of caller-supplied output destinations does not match the plan.
    OutputCount {
        /// Retained outputs in the compiled plan.
        expected: usize,
        /// Slices supplied by the caller.
        actual: usize,
    },
    /// A caller-supplied output destination has the wrong length.
    OutputLength {
        /// Position of the mismatched output.
        index: usize,
        /// Length the plan produces.
        expected: usize,
        /// Length the caller supplied.
        actual: usize,
    },
    /// The runtime context refused the plan before any work was done.
    Context(RuntimeContextError),
    /// Numeric kernel dispatch failed.
    Kernel(KernelDispatchError),
}

impl From<KernelDispatchError> for ExecuteError {
    fn from(value: KernelDispatchError) -> Self {
        Self::Kernel(value)
    }
}

impl From<RuntimeContextError> for ExecuteError {
    fn from(value: RuntimeContextError) -> Self {
        Self::Context(value)
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
                write!(
                    f,
                    "execution range {start}..{end} exceeds input length {len}"
                )
            }
            Self::MissingParameters => write!(f, "hot node referenced missing parameters"),
            Self::AliasedOutput(slot) => {
                write!(f, "retained outputs alias physical buffer slot {}", slot.0)
            }
            Self::OutputCount { expected, actual } => {
                write!(
                    f,
                    "plan retains {expected} output(s), caller supplied {actual} destination(s)"
                )
            }
            Self::OutputLength {
                index,
                expected,
                actual,
            } => write!(
                f,
                "output {index} has length {expected}, destination has length {actual}"
            ),
            Self::Context(error) => error.fmt(f),
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
/// The object owns a [`RuntimeContext`] — scratch arena, persistent state,
/// limits, counters and diagnostics — across calls. `execute_last` intentionally
/// keeps state for streaming use. Call [`Self::reset`] when starting a logically
/// independent series.
pub struct UnifiedExecutor<D> {
    plan: HotExecutionPlan,
    dispatcher: D,
    context: RuntimeContext,
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
        Self::with_context(
            plan,
            dispatcher,
            RuntimeContext::with_limits_and_arena(ExecutionLimits::default(), buffer_config),
        )
    }

    /// Construct an executor over a caller-supplied context.
    ///
    /// This is the entry point a backend uses to impose its own budgets or to
    /// share one context's accounting across several executors.
    pub fn with_context(
        plan: HotExecutionPlan,
        dispatcher: D,
        mut context: RuntimeContext,
    ) -> Self {
        plan.state_layout().prepare(context.states_mut());
        Self {
            plan,
            dispatcher,
            context,
        }
    }

    /// Execute the complete bound input extent.
    ///
    /// Persistent state is preserved across calls; call [`Self::reset`] before
    /// beginning an independent batch series.
    pub fn execute(&mut self, inputs: &[&[f64]]) -> Result<ExecutionOutput, ExecuteError> {
        let len = inputs
            .first()
            .map(|input| input.len())
            .ok_or(ExecuteError::NoInputs)?;
        self.execute_range(inputs, 0..len)
    }

    /// Execute one explicit half-open input range.
    pub fn execute_range(
        &mut self,
        inputs: &[&[f64]],
        range: Range<usize>,
    ) -> Result<ExecutionOutput, ExecuteError> {
        let common_len = validate_inputs(&self.plan, inputs)?;
        if range.start > range.end {
            return Err(ExecuteError::InvalidRange {
                start: range.start,
                end: range.end,
                len: common_len.unwrap_or(0),
            });
        }
        // Only a plan with bound inputs has an extent the range must fit inside.
        if let Some(len) = common_len {
            if range.end > len {
                return Err(ExecuteError::InvalidRange {
                    start: range.start,
                    end: range.end,
                    len,
                });
            }
        }
        let kind = if range.start == 0 && common_len == Some(range.end) {
            ExecutionKind::Full
        } else {
            ExecutionKind::Range
        };
        self.run(inputs, range, kind)
    }

    /// Execute only the final bound sample while preserving persistent state.
    ///
    /// This is the streaming/`eval_last` path. Stateful dispatchers update the
    /// same [`StateArena`] slots on every call.
    pub fn execute_last(&mut self, inputs: &[&[f64]]) -> Result<ExecutionOutput, ExecuteError> {
        // Without bound inputs there is no "last sample" to address: the length
        // of a constant-only plan is the caller's to choose, and this entry point
        // has no way to receive it.
        let Some(common_len) = validate_inputs(&self.plan, inputs)? else {
            return Err(ExecuteError::InvalidRange {
                start: 0,
                end: 1,
                len: 0,
            });
        };
        if common_len == 0 {
            return Err(ExecuteError::InvalidRange {
                start: 0,
                end: 1,
                len: 0,
            });
        }
        self.run(inputs, common_len - 1..common_len, ExecutionKind::Last)
    }

    fn run(
        &mut self,
        inputs: &[&[f64]],
        range: Range<usize>,
        kind: ExecutionKind,
    ) -> Result<ExecutionOutput, ExecuteError> {
        let logical_len = range.end - range.start;
        if let Some(slot) = self.plan.output_layout().duplicate_slot() {
            return Err(ExecuteError::AliasedOutput(slot));
        }
        // Budgets are checked before a single byte is reserved, so an oversized
        // plan costs one comparison instead of a partially materialised arena.
        self.context.enforce_plan(&self.plan)?;
        let node_count = self.plan.nodes().len();

        let layout = self.plan.buffer_layout();
        let mut buffers = layout.take_buffers(self.context.buffers_mut(), logical_len);
        let buffers_taken = buffers.len();

        let outcome = Self::dispatch_nodes(
            &self.plan,
            &mut self.dispatcher,
            inputs,
            &range,
            &mut buffers,
            self.context.states_mut(),
        );
        layout.recycle_buffers(self.context.buffers_mut(), buffers);

        let (output, kernel_calls) = outcome?;
        self.context.record_execution(kind, node_count);
        self.context.record_kernel_calls(kernel_calls);
        self.context.record_buffers_taken(buffers_taken);
        self.context.record_buffers_recycled(buffers_taken);
        Ok(output)
    }

    /// Execute every plan node into `buffers`.
    ///
    /// Split out of [`Self::run`] so the scratch-buffer recycle happens on both
    /// the success and the failure path while the arenas stay borrowed once.
    fn dispatch_nodes(
        plan: &HotExecutionPlan,
        dispatcher: &mut D,
        inputs: &[&[f64]],
        range: &Range<usize>,
        buffers: &mut [Vec<f64>],
        states: &mut StateArena,
    ) -> Result<(ExecutionOutput, usize), ExecuteError> {
        let mut kernel_calls = 0usize;
        for node in plan.nodes() {
            if let Some(input_slot) = plan.input_layout().slot(node.node) {
                let source = &inputs[input_slot.0][range.clone()];
                buffers[node.output.0].copy_from_slice(source);
                continue;
            }

            let parameters = plan
                .parameter_arena()
                .range(node.parameters)
                .ok_or(ExecuteError::MissingParameters)?;
            dispatcher
                .dispatch(
                    KernelCall {
                        kernel: node.kernel,
                        inputs: &node.inputs,
                        output: node.output,
                        parameters,
                        state: node.state,
                    },
                    buffers,
                    states,
                )
                .map_err(|error| error.with_kernel(node.kernel))?;
            kernel_calls += 1;
        }

        let mut values = Vec::with_capacity(plan.output_layout().len());
        for &(_, slot) in plan.output_layout().outputs() {
            values.push(std::mem::take(&mut buffers[slot.0]));
        }
        Ok((ExecutionOutput { values }, kernel_calls))
    }

    /// Execute and copy the retained outputs into caller-owned storage.
    ///
    /// [`Self::execute`] must move the retained buffers *out* of the arena,
    /// because the caller owns the results. That leaves an empty `Vec` behind
    /// in each output slot, and `BufferArena::recycle` deliberately refuses
    /// zero-length buffers, so the next execution has to allocate a replacement
    /// for every retained output. For a scan that runs one plan over thousands
    /// of symbols, that is one allocation per output per symbol — the exact
    /// "duplicate allocation" §20, priority 2 is about, and the reason the plan
    /// lists *persistent output* alongside `_into` and `BufferArena`.
    ///
    /// This entry point closes that gap. It runs the same plan, copies each
    /// retained output into the slice the caller supplies, and then hands the
    /// arena buffers straight back. A caller that keeps one destination per
    /// output across a scan loop therefore performs **zero arena allocations
    /// after the first execution**, at the cost of one `memcpy` per output per
    /// execution.
    ///
    /// The trade is deliberate: the copy is `logical_len` `f64` against an
    /// allocation of the same size, and unlike the allocation it is flat in the
    /// number of symbols scanned.
    ///
    /// Every destination is validated — both its count and its length — before
    /// the plan runs. A rejected call therefore has *no* side effects, which
    /// matters because a stateful kernel advances its arena slots during a run:
    /// validating the length afterwards would leave the executor one step ahead
    /// of its caller, and a retry with a corrected destination would silently
    /// compute a different series.
    ///
    /// # Errors
    ///
    /// Returns [`ExecuteError::NoInputs`] when `inputs` is empty,
    /// [`ExecuteError::OutputCount`] when `outputs` does not have one entry per
    /// plan output, and [`ExecuteError::OutputLength`] when a destination is not
    /// the logical length of the execution. Errors raised by the run itself are
    /// propagated unchanged.
    pub fn execute_into(
        &mut self,
        inputs: &[&[f64]],
        outputs: &mut [&mut [f64]],
    ) -> Result<(), ExecuteError> {
        let expected = self.plan.output_layout().len();
        if outputs.len() != expected {
            return Err(ExecuteError::OutputCount {
                expected,
                actual: outputs.len(),
            });
        }

        // The logical length of this execution is known before it starts:
        // `execute` takes it from the first input, and `run` applies that same
        // extent to every buffer it draws from the arena.
        let logical_len = inputs
            .first()
            .map(|input| input.len())
            .ok_or(ExecuteError::NoInputs)?;
        if let Some((index, actual)) = outputs
            .iter()
            .enumerate()
            .find(|(_, destination)| destination.len() != logical_len)
            .map(|(index, destination)| (index, destination.len()))
        {
            return Err(ExecuteError::OutputLength {
                index,
                expected: logical_len,
                actual,
            });
        }

        let produced = self.execute(inputs)?;

        for (value, destination) in produced.values.iter().zip(outputs.iter_mut()) {
            debug_assert_eq!(
                value.len(),
                destination.len(),
                "a successful run produces one buffer per output, each of the logical length"
            );
            destination.copy_from_slice(value);
        }

        Self::recycle_produced(&mut self.context, produced.values);
        Ok(())
    }

    /// Return result buffers to the arena instead of dropping them.
    fn recycle_produced(context: &mut RuntimeContext, values: Vec<Vec<f64>>) {
        let arena = context.buffers_mut();
        for buffer in values {
            arena.recycle(buffer);
        }
    }

    /// Drop persistent kernel state while retaining allocated slot capacity and
    /// reusable scratch buffers.
    pub fn reset(&mut self) {
        self.context.reset();
        self.plan.state_layout().prepare(self.context.states_mut());
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

    /// The runtime context this executor runs against.
    pub const fn context(&self) -> &RuntimeContext {
        &self.context
    }

    /// Mutable runtime context (limits, diagnostics, metrics).
    pub const fn context_mut(&mut self) -> &mut RuntimeContext {
        &mut self.context
    }

    /// Persistent state arena used by streaming/stateful kernels.
    pub const fn states(&self) -> &StateArena {
        self.context.states()
    }

    /// Current buffer allocation/reuse counters.
    pub fn buffer_stats(&self) -> BufferArenaStats {
        self.context.buffer_stats()
    }
}

/// Validate the bound inputs and report their common extent.
///
/// `Ok(None)` means the plan declares no input slots at all — a constant-only
/// formula such as `10 + 20`. There is then no input extent to check a range
/// against, and the caller supplies the length itself; collapsing that case to
/// `0` (as this used to) made every non-empty range look out of bounds.
fn validate_inputs(
    plan: &HotExecutionPlan,
    inputs: &[&[f64]],
) -> Result<Option<usize>, ExecuteError> {
    let expected = plan.input_layout().len();
    if inputs.len() != expected {
        return Err(ExecuteError::InputCount {
            expected,
            actual: inputs.len(),
        });
    }
    let Some(common_len) = inputs.first().map(|input| input.len()) else {
        return Ok(None);
    };
    for (slot, input) in inputs.iter().enumerate() {
        if input.len() != common_len {
            return Err(ExecuteError::InputLength {
                slot,
                expected: common_len,
                actual: input.len(),
            });
        }
    }
    Ok(Some(common_len))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compute::{
        ComputeCapabilities, ComputeEffect, ComputeNode, ComputeNodeId, ComputePlan,
        DependencyShape, LookbackRequirement,
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
            dependency: DependencyShape::FixedLookback(0),
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
            ComputeNode::new(ComputeNodeId(0), "VARIABLE:CLOSE", vec![], pure()),
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

    #[test]
    fn context_classifies_every_entry_point_and_counts_kernels() {
        let mut executor = UnifiedExecutor::new(plan(), TestDispatcher);
        let close = [1.0, 2.0, 3.0, 4.0];

        executor.execute(&[&close]).unwrap();
        executor.execute_range(&[&close], 1..3).unwrap();
        executor.execute_last(&[&close]).unwrap();

        let metrics = executor.context().metrics();
        assert_eq!(metrics.full_executions, 1);
        assert_eq!(metrics.range_executions, 1);
        assert_eq!(metrics.last_executions, 1);
        assert_eq!(metrics.executions(), 3);
        // One dispatchable node (COPY_PLUS_ONE) per execution; the input binding
        // is a copy, not a kernel.
        assert_eq!(metrics.kernel_calls, 3);
        // Two plan nodes counted per execution, including the input binding.
        assert_eq!(metrics.node_visits, 6);
        // Two scratch buffers per execution round-tripped through the arena.
        assert_eq!(metrics.buffers_taken, 6);
        assert_eq!(metrics.buffers_recycled, 6);
    }

    #[test]
    fn context_budget_rejects_plan_before_allocating() {
        let context = RuntimeContext::with_limits(ExecutionLimits::new(1, 1 << 20, 16, 8));
        let mut executor = UnifiedExecutor::with_context(plan(), TestDispatcher, context);
        let close = [1.0, 2.0];

        assert_eq!(
            executor.execute(&[&close]).unwrap_err(),
            ExecuteError::Context(RuntimeContextError::PlanTooLarge { nodes: 2, limit: 1 })
        );
        // The rejection happened before checkout, so nothing was allocated and
        // no execution was accounted.
        assert_eq!(executor.context().metrics().executions(), 0);
        assert_eq!(executor.context().buffer_stats().cache_misses, 0);
    }
}
