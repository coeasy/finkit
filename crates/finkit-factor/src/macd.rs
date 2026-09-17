use crate::{Ema, Factor};
use finkit_series::QuantSeries;

#[derive(Debug, Clone)]
pub struct Macd {
    pub fast: usize,
    pub slow: usize,
    pub signal: usize,
}

impl Macd {
    pub fn new(fast: usize, slow: usize, signal: usize) -> Self {
        Self { fast, slow, signal }
    }
}

impl Factor for Macd {
    fn name(&self) -> &str {
        "MACD"
    }

    fn compute(&self, input: &QuantSeries) -> QuantSeries {
        let fast = Ema::new(self.fast).compute(input);
        let slow = Ema::new(self.slow).compute(input);
        let values = fast
            .values()
            .iter()
            .zip(slow.values().iter())
            .map(|(a, b)| a - b)
            .collect();
        QuantSeries::new(
            input.symbol(),
            fast.timestamps().to_vec(),
            finkit_array::FloatArray::new(values),
        )
    }
}
