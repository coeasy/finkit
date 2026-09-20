//! Mathematical kernels for Quant Factor Computing Engine.
//!
//! # Superseded -- do not add public API
//!
//! Every kernel here is a strict subset of `core`'s batch function set
//! (`core/src/formula/functions.rs`, 7431 lines, plus `core/src/indicators/` and
//! the 111 `FunctionSpec` entries in `core/src/registry.rs`), which is also
//! covered by the TA-Lib 201/201 parity gate. Scheduled for removal -- see
//! `docs/runtime-carrier-adoption-plan-2026-09-20.md`.

mod ema;

pub use ema::ema;

pub fn rolling_mean(values: &[f64], period: usize) -> Vec<f64> {
    if period == 0 || values.len() < period {
        return Vec::new();
    }

    values
        .windows(period)
        .map(|window| window.iter().sum::<f64>() / period as f64)
        .collect()
}

pub fn rolling_variance(values: &[f64], period: usize) -> Vec<f64> {
    if period == 0 || values.len() < period {
        return Vec::new();
    }

    values
        .windows(period)
        .map(|window| {
            let mean = window.iter().sum::<f64>() / period as f64;
            window.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / period as f64
        })
        .collect()
}

pub fn rolling_std(values: &[f64], period: usize) -> Vec<f64> {
    rolling_variance(values, period)
        .into_iter()
        .map(f64::sqrt)
        .collect()
}
