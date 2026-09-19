use crate::{Factor, FactorResult};
use finkit_series::QuantSeries;

#[derive(Debug, Clone)]
pub struct Ema {
    pub period: usize,
}

impl Ema {
    pub fn new(period: usize) -> Self {
        Self { period }
    }
}

impl Factor for Ema {
    fn name(&self) -> &str {
        "EMA"
    }

    fn compute(&self, input: &QuantSeries) -> FactorResult {
        if !input.is_valid() || self.period == 0 || input.len() < self.period {
            return FactorResult::new(
                self.name(),
                QuantSeries::new(
                    input.symbol(),
                    Vec::new(),
                    finkit_array::FloatArray::new(Vec::new()),
                ),
            );
        }
        let values = finkit_math::ema(input.values(), self.period);
        let timestamps = input.timestamps()[self.period - 1..].to_vec();
        FactorResult::new(
            self.name(),
            QuantSeries::new(
                input.symbol().to_string(),
                timestamps,
                finkit_array::FloatArray::new(values),
            ),
        )
    }
}
