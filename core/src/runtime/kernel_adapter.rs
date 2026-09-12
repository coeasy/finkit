//! Typed adapters from canonical kernels into the unified runtime.

use crate::math::kernels::{MonotonicExtrema, MovingAverageState, TrueRangeState, WelfordState};
use std::fmt;

/// Batch execution failure shared by typed adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelAdapterError {
    /// Output length must exactly match the input length for aligned execution.
    LengthMismatch { input: usize, output: usize },
}

impl fmt::Display for KernelAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch { input, output } => write!(
                f,
                "kernel batch output length mismatch: input={input}, output={output}"
            ),
        }
    }
}

impl std::error::Error for KernelAdapterError {}

/// Stateful typed kernel contract used by batch/streaming/runtime schedulers.
pub trait KernelExecutor {
    /// One update payload.
    type Input: Copy;
    /// One output payload.
    type Output: Copy;
    /// Cloneable state image for checkpoints and replay.
    type Snapshot: Clone;

    /// Apply one incremental update.
    fn update(&mut self, input: Self::Input) -> Self::Output;

    /// Capture kernel state.
    fn snapshot(&self) -> Self::Snapshot;

    /// Restore a prior kernel state.
    fn restore(&mut self, snapshot: &Self::Snapshot);

    /// Apply an aligned batch without allocating an output buffer.
    fn update_batch(
        &mut self,
        input: &[Self::Input],
        output: &mut [Self::Output],
    ) -> Result<(), KernelAdapterError> {
        if input.len() != output.len() {
            return Err(KernelAdapterError::LengthMismatch {
                input: input.len(),
                output: output.len(),
            });
        }
        for (source, target) in input.iter().copied().zip(output.iter_mut()) {
            *target = self.update(source);
        }
        Ok(())
    }
}

impl KernelExecutor for MovingAverageState {
    type Input = f64;
    type Output = f64;
    type Snapshot = MovingAverageState;

    #[inline(always)]
    fn update(&mut self, input: Self::Input) -> Self::Output {
        MovingAverageState::update(self, input)
    }

    fn snapshot(&self) -> Self::Snapshot {
        self.clone()
    }

    fn restore(&mut self, snapshot: &Self::Snapshot) {
        self.clone_from(snapshot);
    }
}

/// Compact output emitted after one Welford update.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatisticsOutput {
    /// Number of observations in the state.
    pub count: usize,
    /// Current mean.
    pub mean: f64,
    /// Population variance.
    pub variance: f64,
    /// Population standard deviation.
    pub stddev: f64,
}

impl KernelExecutor for WelfordState {
    type Input = f64;
    type Output = StatisticsOutput;
    type Snapshot = WelfordState;

    #[inline]
    fn update(&mut self, input: Self::Input) -> Self::Output {
        WelfordState::update(self, input);
        StatisticsOutput {
            count: self.count(),
            mean: self.mean(),
            variance: self.variance(),
            stddev: self.stddev(),
        }
    }

    fn snapshot(&self) -> Self::Snapshot {
        *self
    }

    fn restore(&mut self, snapshot: &Self::Snapshot) {
        *self = *snapshot;
    }
}

/// OHLC tuple required by true-range calculations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OhlcInput {
    /// High price.
    pub high: f64,
    /// Low price.
    pub low: f64,
    /// Close price.
    pub close: f64,
}

impl KernelExecutor for TrueRangeState {
    type Input = OhlcInput;
    type Output = f64;
    type Snapshot = TrueRangeState;

    #[inline]
    fn update(&mut self, input: Self::Input) -> Self::Output {
        TrueRangeState::update(self, input.high, input.low, input.close)
    }

    fn snapshot(&self) -> Self::Snapshot {
        *self
    }

    fn restore(&mut self, snapshot: &Self::Snapshot) {
        *self = *snapshot;
    }
}

/// Runtime wrapper that gives the deque-based extrema kernel its own index.
#[derive(Debug, Clone)]
pub struct ExtremaKernelAdapter {
    state: MonotonicExtrema,
    next_index: usize,
}

impl ExtremaKernelAdapter {
    /// Build a rolling min (`max=false`) or max (`max=true`) executor.
    #[must_use]
    pub fn new(window: usize, max: bool) -> Self {
        Self {
            state: MonotonicExtrema::new(window, max),
            next_index: 0,
        }
    }
}

impl KernelExecutor for ExtremaKernelAdapter {
    type Input = f64;
    type Output = f64;
    type Snapshot = ExtremaKernelAdapter;

    #[inline]
    fn update(&mut self, input: Self::Input) -> Self::Output {
        let output = self.state.update(self.next_index, input);
        self.next_index = self.next_index.saturating_add(1);
        output
    }

    fn snapshot(&self) -> Self::Snapshot {
        self.clone()
    }

    fn restore(&mut self, snapshot: &Self::Snapshot) {
        self.clone_from(snapshot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::kernels::{MovingAverageKind, MovingAverageState};

    #[test]
    fn batch_adapter_writes_into_existing_buffer() {
        let mut state = MovingAverageState::new(MovingAverageKind::Sma, 3);
        let input = [1.0, 2.0, 3.0, 4.0];
        let mut output = [0.0; 4];
        KernelExecutor::update_batch(&mut state, &input, &mut output).unwrap();
        assert_eq!(output, [1.0, 1.5, 2.0, 3.0]);
    }

    #[test]
    fn snapshot_restores_incremental_path() {
        let mut state = MovingAverageState::new(MovingAverageKind::Sma, 2);
        assert_eq!(state.update(2.0), 2.0);
        let snapshot = KernelExecutor::snapshot(&state);
        assert_eq!(state.update(4.0), 3.0);
        KernelExecutor::restore(&mut state, &snapshot);
        assert_eq!(state.update(6.0), 4.0);
    }

    #[test]
    fn extrema_adapter_tracks_index_internally() {
        let mut state = ExtremaKernelAdapter::new(3, true);
        assert_eq!(state.update(1.0), 1.0);
        assert_eq!(state.update(4.0), 4.0);
        assert_eq!(state.update(2.0), 4.0);
        assert_eq!(state.update(3.0), 4.0);
        assert_eq!(state.update(0.0), 3.0);
    }
}
