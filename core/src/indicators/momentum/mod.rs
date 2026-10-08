//! Momentum indicators.
//!
//! Split by indicator family; the module path and the public surface
//! are unchanged. Shared helpers (`typical_price`, `non_finite_at`) and
//! the crate imports the families compose with stay here, and each family
//! reaches them through `prelude` rather than a bare `use super::*;`.

use crate::error::{Result, TaError};
use crate::indicators::overlap::MaType;
use crate::math::moving_avg::{ema, simd_horizontal_sum};
use crate::math::simd_ops;
use crate::math::statistics::rolling_minmax_visit;
use crate::utils::{init_output, smoothing_factor, validate_input};
use ndarray::Array1;

#[inline]
fn typical_price(high: f64, low: f64, close: f64) -> f64 {
    (high + low + close) / 3.0
}

/// Non-finite input rejection, matching the message `sma`/`ema` emit.
#[inline]
fn non_finite_at(index: usize) -> TaError {
    TaError::InvalidParameter {
        name: "input".to_string(),
        constraint: format!("non-finite value at index {index}"),
    }
}

mod apo_ppo_impl;
mod aroon_impl;
mod bop_impl;
mod cci_impl;
mod cmo_impl;
mod dmi_impl;
mod elder_ray_impl;
mod macd_impl;
mod mfi_impl;
pub(crate) mod prelude;
mod rate_of_change_impl;
mod rsi_impl;
mod stoch_impl;
mod stochf_impl;
mod stochrsi_impl;
#[cfg(test)]
mod tests;
mod trix_impl;
mod ultosc_impl;
mod willr_impl;

// The public surface is re-exported explicitly rather than by glob, so
// splitting the file cannot accidentally publish (or hide) a name.
// Sibling families do not need a crate-internal facade on top of this:
// a bucket that builds on another names it through `prelude`.
pub use apo_ppo_impl::apo;
pub use apo_ppo_impl::apo_with_ma_type;
pub use apo_ppo_impl::ppo;
pub use apo_ppo_impl::ppo_with_ma_type;
pub use aroon_impl::aroon;
pub use aroon_impl::aroon_into;
pub use aroon_impl::aroonosc;
pub use aroon_impl::AroonResult;
pub use bop_impl::bop;
pub use cci_impl::cci;
pub use cci_impl::cci_into;
pub use cci_impl::cci_source_into;
pub use cmo_impl::cmo;
pub use cmo_impl::cmo_fast_into;
pub use dmi_impl::adx;
pub use dmi_impl::adx_from_di_into;
pub use dmi_impl::adx_into;
pub use dmi_impl::adxr;
pub use dmi_impl::adxr_into;
pub use dmi_impl::dx;
pub use dmi_impl::dx_into;
pub use dmi_impl::minus_di;
pub use dmi_impl::minus_di_fast_into;
pub use dmi_impl::minus_dm;
pub use dmi_impl::minus_dm_with_period;
pub use dmi_impl::plus_di;
pub use dmi_impl::plus_di_fast_into;
pub use dmi_impl::plus_dm;
pub use dmi_impl::plus_dm_with_period;
pub use elder_ray_impl::elder_ray;
pub use elder_ray_impl::ElderRayResult;
pub use macd_impl::macd;
pub use macd_impl::macd_fast_into;
pub use macd_impl::macd_into;
pub use macd_impl::macd_line_into;
pub use macd_impl::macdext;
pub use macd_impl::macdfix;
pub use macd_impl::macdfix_into;
pub use macd_impl::macdfix_with_signal;
pub use macd_impl::MacdResult;
pub use mfi_impl::mfi;
pub use rate_of_change_impl::mom;
pub use rate_of_change_impl::mom_into;
pub use rate_of_change_impl::roc;
pub use rate_of_change_impl::roc_into;
pub use rate_of_change_impl::rocp;
pub use rate_of_change_impl::rocr;
pub use rate_of_change_impl::rocr100;
pub use rsi_impl::rsi;
pub use rsi_impl::rsi_into;
pub use stoch_impl::stoch;
pub use stoch_impl::stoch_into;
pub use stoch_impl::stoch_with_ma_types;
pub use stoch_impl::StochResult;
pub use stochf_impl::stochf;
pub use stochf_impl::stochf_into;
pub use stochf_impl::stochf_with_ma_type;
pub use stochrsi_impl::stochrsi;
pub use stochrsi_impl::stochrsi_into;
pub use stochrsi_impl::stochrsi_with_ma_type;
pub use trix_impl::trix;
pub use trix_impl::trix_into;
pub use ultosc_impl::ultosc;
pub use ultosc_impl::ultosc_into;
pub use willr_impl::willr;
pub use willr_impl::willr14_into;
pub use willr_impl::willr_into;
