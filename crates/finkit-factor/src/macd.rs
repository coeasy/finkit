use crate::{Ema, Factor, FactorResult};
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

    fn compute(&self, input: &QuantSeries) -> FactorResult {
        let fast = Ema::new(self.fast).compute(input).series;
        let slow = Ema::new(self.slow).compute(input).series;
        let (timestamps, values) = align_difference(&fast, &slow);
        let macd = QuantSeries::new(
            input.symbol(),
            timestamps,
            finkit_array::FloatArray::new(values),
        );
        if macd.is_empty() {
            return FactorResult::new(self.name(), macd);
        }

        let signal = Ema::new(self.signal).compute(&macd).series;
        let (histogram_timestamps, histogram_values) = align_difference(&macd, &signal);
        let histogram = QuantSeries::new(
            input.symbol(),
            histogram_timestamps,
            finkit_array::FloatArray::new(histogram_values),
        );
        FactorResult::new(self.name(), macd.clone())
            .with_output("macd", macd)
            .with_output("signal", signal)
            .with_output("histogram", histogram)
    }
}

fn align_difference(left: &QuantSeries, right: &QuantSeries) -> (Vec<i64>, Vec<f64>) {
    let mut left_index = 0;
    let mut right_index = 0;
    let mut timestamps = Vec::new();
    let mut values = Vec::new();
    while left_index < left.timestamps().len() && right_index < right.timestamps().len() {
        match left.timestamps()[left_index].cmp(&right.timestamps()[right_index]) {
            std::cmp::Ordering::Less => left_index += 1,
            std::cmp::Ordering::Greater => right_index += 1,
            std::cmp::Ordering::Equal => {
                timestamps.push(left.timestamps()[left_index]);
                values.push(left.values()[left_index] - right.values()[right_index]);
                left_index += 1;
                right_index += 1;
            }
        }
    }
    (timestamps, values)
}
