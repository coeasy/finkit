mod extrema;
mod moving_average;
mod registry;
mod statistics;
mod volatility;

pub use extrema::MonotonicExtrema;
pub use moving_average::{MovingAverageKind, MovingAverageState};
pub use registry::{capabilities, moving_average, KernelCapability, KernelFamily};
pub use statistics::WelfordState;
pub use volatility::TrueRangeState;
pub use rsi_core::RsiState;
