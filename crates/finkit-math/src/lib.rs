//! Mathematical kernels for Quant Factor Computing Engine.

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
