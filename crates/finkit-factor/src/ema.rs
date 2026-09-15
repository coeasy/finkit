use finkit_series::QuantSeries;
use crate::Factor;

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

    fn compute(&self, input: &QuantSeries) -> QuantSeries {
        let values = finkit_math::ema(input.values(), self.period);
        QuantSeries::new(input.symbol().to_string(), Vec::new(), finkit_array::FloatArray::new(values))
    }
}
