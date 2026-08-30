//! Pattern recognition for candlestick and chart patterns.
//!
//! # Module organization
//!
//! - [`common`] — shared utilities (ATR pre-compute, candle/shadow helpers,
//!   trend/volume context). All pattern detectors should build on top of
//!   these.
//! - [`candlestick`] — 60+ TA-Lib compatible single/multi-bar K-line
//!   patterns (Doji, Hammer, Engulfing, Morning/Evening Star, etc.).
//! - [`chart`] — 15+ chart-level patterns (H&S, Double Top/Bottom,
//!   Triangles, Wedges, Flags, Pennants, Rectangles).
//!
//! # A-share extensions
//!
//! - [`astock_kline`] — 30 A-share specific K-line patterns (仙人指路,
//!   老鸭头, 多方炮, 空方炮, 三阳开泰, 红杏出墙, 蚂蚁上树, 梅开二度,
//!   拨云见日, 海底捞月, 一阳/阴穿三线, etc.).
//! - [`astock_ma`] — 15 moving-average combination patterns (金叉, 死叉,
//!   银山谷, 金山谷, 死亡谷, 金蜘蛛, 死蜘蛛, 多/空头排列, 粘合, etc.).
//! - [`harmonic`] — 5 harmonic patterns (AB=CD, Gartley, Bat, Butterfly, Crab).
//! - [`classic_ext`] — 10 international classics (VCP, Cup & Handle,
//!   Rounding Top/Bottom, Island Reversal, Broadening Triangle, Diamond,
//!   Harami-with-volume, Morning/Evening Star with trend filter).
//!
//! # Streaming
//!
//! - [`streaming`] — O(1) per-bar streaming pattern detectors. Every
//!   pattern in this module is also exposed as a `StreamingXxx` struct that
//!   maintains its own ring-buffer state.

pub mod astock_kline;
pub mod astock_kline2;
pub mod astock_ma;
pub mod candlestick;
pub mod chart;
pub mod classic_ext;
pub mod common;
pub mod harmonic;
// `patterns::streaming` bridges to the (indicators-all gated) top-level
// `streaming` module, so it shares the same feature gate.
#[cfg(feature = "indicators-all")]
pub mod streaming;

#[allow(ambiguous_glob_reexports)]
pub use astock_kline::*;
#[allow(ambiguous_glob_reexports)]
pub use astock_kline2::*;
#[allow(ambiguous_glob_reexports)]
pub use astock_ma::*;
#[allow(ambiguous_glob_reexports)]
pub use candlestick::*;
#[allow(ambiguous_glob_reexports)]
pub use chart::*;
#[allow(ambiguous_glob_reexports)]
pub use classic_ext::*;
pub use common::*;
#[allow(ambiguous_glob_reexports)]
pub use harmonic::*;
#[cfg(feature = "indicators-all")]
pub use streaming::*;
