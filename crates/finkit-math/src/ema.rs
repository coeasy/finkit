//! Exponential moving average kernel.

pub fn ema(values: &[f64], period: usize) -> Vec<f64> {
    if period == 0 || values.len() < period {
        return Vec::new();
    }

    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut result = Vec::with_capacity(values.len() - period + 1);
    let mut current = values[..period].iter().sum::<f64>() / period as f64;
    result.push(current);

    for value in &values[period..] {
        current = (*value - current) * multiplier + current;
        result.push(current);
    }

    result
}
