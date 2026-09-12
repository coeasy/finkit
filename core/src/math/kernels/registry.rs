//! Canonical kernel contracts for Architecture V4.
//!
//! The registry is intentionally lightweight: execution planners depend on
//! stable kernel capabilities rather than concrete indicator implementations.

use super::{MovingAverageKind, MovingAverageState};

/// Runtime family of a reusable quantitative kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelFamily {
    /// Rolling and moving average calculations.
    MovingAverage,
    /// Online statistical calculations.
    Statistics,
    /// Volatility and directional movement calculations.
    Volatility,
    /// Sliding extrema calculations.
    Extrema,
}

/// Capability declaration consumed by planners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelCapability {
    /// Kernel family.
    pub family: KernelFamily,
    /// Supports O(1) streaming updates.
    pub streaming: bool,
    /// Supports DirtyRange partial recomputation.
    pub dirty_range: bool,
}

/// Canonical moving-average constructor used by runtime planners.
#[must_use]
pub fn moving_average(kind: MovingAverageKind, window: usize) -> MovingAverageState {
    MovingAverageState::new(kind, window)
}

/// Returns built-in Architecture V4 kernel capabilities.
#[must_use]
pub const fn capabilities() -> [KernelCapability; 4] {
    [
        KernelCapability {
            family: KernelFamily::MovingAverage,
            streaming: true,
            dirty_range: true,
        },
        KernelCapability {
            family: KernelFamily::Statistics,
            streaming: true,
            dirty_range: true,
        },
        KernelCapability {
            family: KernelFamily::Volatility,
            streaming: true,
            dirty_range: true,
        },
        KernelCapability {
            family: KernelFamily::Extrema,
            streaming: true,
            dirty_range: true,
        },
    ]
}
