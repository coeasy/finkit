use finkit_series::QuantSeries;
use crate::Factor;

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

    fn compute(&self, input: &QuantSeries) -> QuantSeries {
        let values = input.values();
        if self.period == 0 || values.len() <= self.period {
            return QuantSeries::new(input.symbol(), Vec::new(), finkit_array::FloatArray::new(Vec::new()));
        }

        let mut result = Vec::new();
        for window in values.windows(self.period + 1) {
            let mut gain = 0.0;
            let mut loss = 0.0;
            for pair in window.windows(2) {
                let diff = pair[1] - pair[0];
                if diff > 0.0 { gain += diff; } else { loss -= diff; }
            }
            let rs = if loss == 0.0 { 100.0 } else { gain / loss };
            result.push(100.0 - (100.0 / (1.0 + rs)));
        }

        let timestamps = input.timestamps()[self.period..].to_vec();
        QuantSeries::new(input.symbol(), timestamps, finkit_array::FloatArray::new(result))
    }
}
