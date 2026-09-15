use finkit_series::QuantSeries;
use crate::Factor;

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
        QuantSeries::new(input.symbol().to_string(), Vec::new(), values)
    }
}
