//! Centralized re-exports of all multi-output indicator result types.
//!
//! Each type is defined in its respective indicator module and re-exported
//! here for convenient single-import access.
//!
//! All output structs derive `Copy`, `Clone`, `Debug`, `PartialEq`,
//! and conditionally `Serialize`/`Deserialize` (with the `serde` feature).

pub use super::momentum::aroon::AroonOutput;
pub use super::momentum::fisher_streaming::FisherOutput;
pub use super::momentum::kdj::KdjOutput;
pub use super::momentum::kst::KstOutput;
pub use super::momentum::macd::MacdOutput;
pub use super::momentum::rvi_streaming::RviOutput;
pub use super::momentum::stoch::StochOutput;
pub use super::overlap::dma::DmaOutput;
pub use super::overlap::expma::ExpmaOutput;
pub use super::overlap::ichimoku::IchimokuOutput;
pub use super::trend::sar::SarOutput;
pub use super::trend::supertrend::SuperTrendOutput;
pub use super::volatility::boll::BollOutput;
pub use super::volatility::donchian::DonchianOutput;
pub use super::volatility::ene::EneOutput;
pub use super::volatility::keltner::KeltnerOutput;
pub use super::volume::kvo::KvoOutput;
