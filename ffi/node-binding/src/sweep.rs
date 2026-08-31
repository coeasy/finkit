//! Node.js bindings for parameter sweep API.

use finkit::indicators::sweep::{ema_sweep, rsi_sweep, sma_sweep};
use finkit::indicators::sweep_engine::{ParamRange, SweepEngine};
use finkit::indicators::sweepable::{EmaSweepable, RsiSweepable, SmaSweepable};
use napi::bindgen_prelude::*;
use napi_derive::napi;

/// Sweep SMA across multiple periods.
#[napi]
pub fn sweep_sma(data: Vec<f64>, periods: Vec<u32>) -> Result<Vec<Vec<f64>>> {
    let periods: Vec<usize> = periods.iter().map(|&p| p as usize).collect();
    sma_sweep(&data, &periods).map_err(|e| Error::from_reason(e.to_string()))
}

/// Sweep EMA across multiple periods.
#[napi]
pub fn sweep_ema(data: Vec<f64>, periods: Vec<u32>) -> Result<Vec<Vec<f64>>> {
    let periods: Vec<usize> = periods.iter().map(|&p| p as usize).collect();
    ema_sweep(&data, &periods).map_err(|e| Error::from_reason(e.to_string()))
}

/// Sweep RSI across multiple periods.
#[napi]
pub fn sweep_rsi(data: Vec<f64>, periods: Vec<u32>) -> Result<Vec<Vec<f64>>> {
    let periods: Vec<usize> = periods.iter().map(|&p| p as usize).collect();
    rsi_sweep(&data, &periods).map_err(|e| Error::from_reason(e.to_string()))
}

/// SweepEngine result for JS.
#[napi(object)]
pub struct JsSweepEngineResult {
    pub indicator: String,
    pub param_count: u32,
    pub results: Vec<Vec<f64>>,
}

/// Generic SweepEngine: Cartesian-product parameter scan.
///
/// @param indicator - One of "sma", "ema", "rsi"
/// @param data - Price series
/// @param ranges - Array of [start, end, step] arrays
#[napi]
pub fn sweep_engine_run(
    indicator: String,
    data: Vec<f64>,
    ranges: Vec<Vec<u32>>,
) -> Result<JsSweepEngineResult> {
    let param_ranges: Vec<ParamRange> = ranges
        .iter()
        .map(|r| {
            if r.len() < 3 {
                ParamRange::new(
                    r[0] as usize,
                    r.get(1).copied().unwrap_or(r[0] + 1) as usize,
                    1,
                )
            } else {
                ParamRange::new(r[0] as usize, r[1] as usize, r[2] as usize)
            }
        })
        .collect();

    let engine = SweepEngine::new();
    let result = match indicator.to_lowercase().as_str() {
        "sma" => engine.run(&SmaSweepable, &data, &param_ranges),
        "ema" => engine.run(&EmaSweepable, &data, &param_ranges),
        "rsi" => engine.run(&RsiSweepable, &data, &param_ranges),
        other => {
            return Err(Error::from_reason(format!(
                "Unknown indicator: '{other}'. Supported: sma, ema, rsi"
            )))
        }
    }
    .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(JsSweepEngineResult {
        indicator: result.indicator_name,
        param_count: result.param_count as u32,
        results: result.results.into_iter().map(|r| r.values).collect(),
    })
}
