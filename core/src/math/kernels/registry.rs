//! Canonical kernel contracts for Architecture V4.
//!
//! The registry is intentionally lightweight: execution planners depend on
//! stable kernel capabilities rather than concrete indicator implementations.

use super::{MovingAverageKind, MovingAverageState};

/// Runtime family of a reusable quantitative kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelFamily {
    /// Scalar arithmetic, variables, constants and generic formula operations.
    Scalar,
    /// Rolling and moving average calculations.
    MovingAverage,
    /// Online statistical calculations.
    Statistics,
    /// Volatility and directional movement calculations.
    Volatility,
    /// Sliding extrema calculations.
    Extrema,
    /// Momentum oscillators such as RSI.
    Momentum,
}

/// Capability declaration consumed by planners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelCapability {
    /// Kernel family.
    pub family: KernelFamily,
    /// Supports O(1) streaming updates for canonical state implementations.
    pub streaming: bool,
    /// Can participate in DirtyRange execution. Recursive members require a
    /// row-addressable checkpoint; fixed-window members can use finite replay.
    pub dirty_range: bool,
    /// Family contains recursive-state kernels and therefore must not be
    /// assumed to have a finite lookback without operation-level metadata.
    pub recursive_state_possible: bool,
}

/// Canonical moving-average constructor used by runtime planners.
#[must_use]
pub fn moving_average(kind: MovingAverageKind, window: usize) -> MovingAverageState {
    MovingAverageState::new(kind, window)
}

/// Returns built-in Architecture V4 kernel-family capabilities.
#[must_use]
pub const fn capabilities() -> [KernelCapability; 6] {
    [
        KernelCapability {
            family: KernelFamily::Scalar,
            streaming: true,
            dirty_range: true,
            recursive_state_possible: false,
        },
        KernelCapability {
            family: KernelFamily::MovingAverage,
            streaming: true,
            dirty_range: true,
            recursive_state_possible: true,
        },
        KernelCapability {
            family: KernelFamily::Statistics,
            streaming: true,
            dirty_range: true,
            recursive_state_possible: false,
        },
        KernelCapability {
            family: KernelFamily::Volatility,
            streaming: true,
            dirty_range: true,
            recursive_state_possible: true,
        },
        KernelCapability {
            family: KernelFamily::Extrema,
            streaming: true,
            dirty_range: true,
            recursive_state_possible: false,
        },
        KernelCapability {
            family: KernelFamily::Momentum,
            streaming: true,
            dirty_range: true,
            recursive_state_possible: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_metadata_distinguishes_recursive_families() {
        let capabilities = capabilities();
        let momentum = capabilities
            .iter()
            .find(|capability| capability.family == KernelFamily::Momentum)
            .unwrap();
        let extrema = capabilities
            .iter()
            .find(|capability| capability.family == KernelFamily::Extrema)
            .unwrap();
        let scalar = capabilities
            .iter()
            .find(|capability| capability.family == KernelFamily::Scalar)
            .unwrap();
        assert!(momentum.recursive_state_possible);
        assert!(!extrema.recursive_state_possible);
        assert!(!scalar.recursive_state_possible);
    }
}
