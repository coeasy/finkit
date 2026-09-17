use crate::Factor;
use finkit_series::QuantSeries;

#[derive(Debug, Clone)]
pub struct Sma {
    pub period: usize,
}

impl Sma {
    pub fn new(period: usize) -> Self {
        Self { period }
    }
}

impl Factor for Sma {
    fn name(&self) -> &str {
        "SMA"
    }

    fn compute(&self, input: &QuantSeries) -> QuantSeries {
        let values = finkit_math::rolling_mean(input.values(), self.period);
        let timestamps = input.timestamps()[self.period.saturating_sub(1)..].to_vec();
        QuantSeries::new(
            input.symbol().to_string(),
            timestamps,
            finkit_array::FloatArray::new(values),
        )
    }
}
