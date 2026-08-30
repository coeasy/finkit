//! Alpha/GTJA 常用算子模块。
//!
//! 本模块聚合时间序列算子与截面算子，供因子层复用。

pub mod cross_sectional;
pub mod timeseries;

pub use cross_sectional::{indneutralize, rank, scale, signed_power};
pub use timeseries::{
    correlation, covariance, decay_linear, delay, delta, ts_argmax, ts_argmin, ts_rank,
};
