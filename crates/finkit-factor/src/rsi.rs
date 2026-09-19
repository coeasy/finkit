use crate::{Factor, FactorResult};
use finkit_series::QuantSeries;

#[derive(Debug, Clone)]
pub struct Rsi {
    pub period: usize,
}

impl Rsi {
    pub fn new(period: usize) -> Self {
        Self { period }
    }
}

impl Factor for Rsi {
    fn name(&self) -> &str {
        "RSI"
    }

    fn compute(&self, input: &QuantSeries) -> FactorResult {
        let values = input.values();
        if !input.is_valid() || self.period == 0 || values.len() <= self.period {
            return FactorResult::new(
                self.name(),
                QuantSeries::new(
                    input.symbol(),
                    Vec::new(),
                    finkit_array::FloatArray::new(Vec::new()),
                ),
            );
        }

        let period = self.period;
        let inv_period = 1.0 / period as f64;
        let mut gain = 0.0;
        let mut loss = 0.0;
        for index in 1..=period {
            let diff = values[index] - values[index - 1];
            if diff > 0.0 {
                gain += diff;
            } else {
                loss -= diff;
            }
        }
        let mut average_gain = gain * inv_period;
        let mut average_loss = loss * inv_period;
        let mut result = Vec::with_capacity(values.len() - period);
        result.push(rsi_value(average_gain, average_loss));

        for index in period + 1..values.len() {
            let diff = values[index] - values[index - 1];
            let current_gain = diff.max(0.0);
            let current_loss = (-diff).max(0.0);
            average_gain = (average_gain * (period as f64 - 1.0) + current_gain) * inv_period;
            average_loss = (average_loss * (period as f64 - 1.0) + current_loss) * inv_period;
            result.push(rsi_value(average_gain, average_loss));
        }

        let timestamps = input.timestamps()[self.period..].to_vec();
        FactorResult::new(
            self.name(),
            QuantSeries::new(
                input.symbol(),
                timestamps,
                finkit_array::FloatArray::new(result),
            ),
        )
    }
}

#[inline]
fn rsi_value(average_gain: f64, average_loss: f64) -> f64 {
    if average_loss < 1e-15 {
        100.0
    } else {
        100.0 * average_gain / (average_gain + average_loss)
    }
}
