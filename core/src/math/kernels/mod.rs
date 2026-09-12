mod compat;
mod extrema;
mod moving_average;
mod registry;
mod rsi_core;
mod statistics;
mod volatility;

pub use compat::{
    rolling_max_into, rolling_min_into, rolling_sample_stddev_into, rolling_sample_variance_into,
    sma_into, wma_into, KernelCompatError,
};
pub use extrema::MonotonicExtrema;
pub use moving_average::{MovingAverageKind, MovingAverageState};
pub use registry::{capabilities, moving_average, KernelCapability, KernelFamily};
pub use rsi_core::RsiState;
pub use statistics::{RollingStatistics, RollingWelfordState, WelfordState};
pub use volatility::TrueRangeState;
