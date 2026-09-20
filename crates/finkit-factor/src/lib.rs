//! Factor abstraction layer.
//!
//! # Mixed: partially adopted, mostly superseded
//!
//! Part of the unconverged `crates/finkit-*` migration track -- see
//! `docs/runtime-carrier-adoption-plan-2026-09-20.md`.
//!
//! * **Adopted (phase R2):** the [`Factor`] object boundary. `core` has only the
//!   closure-based `factors::FactorFn` over `FactorInputs`; the provider/factory
//!   boundary built on this trait is a genuine gap and moves into `core`.
//!   [`FactorResult`] is renamed on the way in (`FactorOutputSet`) because
//!   `core::factors::FactorResult<T>` is already `Result<T, FactorError>`.
//! * **Superseded:** [`Sma`], [`Ema`], [`Rsi`] and [`Macd`] duplicate `core`'s
//!   indicator set, which passes the TA-Lib parity gate. Do not extend them.
//!
//! Outputs here are warm-up *trimmed*; `core`'s `runtime::WarmupPolicy` preserves
//! length. Core wins -- see the note in `finkit-series`.

use finkit_series::QuantSeries;

mod ema;
mod macd;
mod result;
mod rsi;
mod sma;

pub use ema::Ema;
pub use macd::Macd;
pub use result::FactorResult;
pub use rsi::Rsi;
pub use sma::Sma;

/// Executable factor implementation used by the runtime provider boundary.
///
/// Factors are immutable after construction, so a single instance can be
/// safely shared by concurrent runtime executions.
pub trait Factor: Send + Sync {
    fn name(&self) -> &str;

    fn compute(&self, input: &QuantSeries) -> FactorResult;
}

#[cfg(test)]
mod tests {
    use super::*;
    use finkit_array::FloatArray;

    fn input() -> QuantSeries {
        QuantSeries::new(
            "TEST",
            (0..10).collect(),
            FloatArray::new((0..10).map(f64::from).collect()),
        )
    }

    #[test]
    fn rsi_uses_wilder_seed_and_returns_full_uptrend() {
        let output = Rsi::new(3).compute(&input());
        assert_eq!(output.series.timestamps(), &[3, 4, 5, 6, 7, 8, 9]);
        assert!(output.series.values().iter().all(|value| *value == 100.0));
    }

    #[test]
    fn macd_aligns_fast_and_slow_warmup_timestamps() {
        let output = Macd::new(3, 5, 2).compute(&input());
        assert_eq!(output.series.timestamps(), &[4, 5, 6, 7, 8, 9]);
        assert_eq!(output.series.len(), 6);
        assert_eq!(
            output.output("signal").unwrap().timestamps(),
            &[5, 6, 7, 8, 9]
        );
        assert_eq!(output.output("histogram").unwrap().len(), 5);
    }

    #[test]
    fn invalid_periods_produce_empty_series_without_panicking() {
        let series = input();
        assert!(Sma::new(0).compute(&series).series.is_empty());
        assert!(Ema::new(20).compute(&series).series.is_empty());
        assert!(Rsi::new(0).compute(&series).series.is_empty());
    }
}
