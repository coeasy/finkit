//! Mathematical kernels for Quant Factor Computing Engine.

pub fn rolling_mean(values: &[f64], period: usize) -> Vec<f64> {
    if period == 0 {
        return Vec::new();
    }

    values
        .windows(period)
        .map(|window| window.iter().sum::<f64>() / period as f64)
        .collect()
}
