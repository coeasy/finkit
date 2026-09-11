use crate::streaming::traits::{IndicatorMeta, Ohlcv};

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SarOutput {
    pub sar: f64,
    pub direction: i32,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingSar {
    acceleration: f64,
    maximum: f64,
    sar: f64,
    ep: f64,
    af: f64,
    direction: i32,
    prev_low: f64,
    prev_low2: f64,
    prev_high: f64,
    prev_high2: f64,
    count: usize,
    last_value: Option<SarOutput>,
}

impl StreamingSar {
    pub fn new(acceleration: f64, maximum: f64) -> Self {
        Self {
            acceleration,
            maximum,
            sar: f64::NAN,
            ep: f64::NAN,
            af: acceleration,
            direction: 1,
            prev_low: f64::NAN,
            prev_low2: f64::NAN,
            prev_high: f64::NAN,
            prev_high2: f64::NAN,
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    pub fn next(&mut self, bar: &dyn Ohlcv) -> Option<SarOutput> {
        self.count += 1;
        let high = bar.high();
        let low = bar.low();

        if self.count == 1 {
            self.direction = 1;
            self.ep = high;
            self.sar = low;
            self.af = self.acceleration;
            self.prev_high = high;
            self.prev_low = low;
            let result = Some(SarOutput {
                sar: self.sar,
                direction: self.direction,
            });
            self.last_value = result;
            return result;
        }

        // Match TA-Lib's batch state machine: the stored SAR is the value
        // emitted for the current bar, and the recursive step prepares the
        // value for the next bar. The second bar establishes direction from
        // the first +DM/-DM pair; ties default to long.
        if self.count == 2 {
            let up_move = high - self.prev_high;
            let down_move = self.prev_low - low;
            self.direction = if down_move > up_move && down_move > 0.0 { -1 } else { 1 };
            self.ep = if self.direction == 1 { high } else { low };
            self.prev_high = high;
            self.prev_low = low;
        }

        let prev_high = self.prev_high;
        let prev_low = self.prev_low;
        let output_sar;
        let next_sar;
        if self.direction == 1 {
            if low <= self.sar {
                self.direction = -1;
                output_sar = self.ep.max(prev_high).max(high);
                self.af = self.acceleration;
                self.ep = low;
                let candidate = self.af.mul_add(self.ep - output_sar, output_sar);
                next_sar = candidate.max(prev_high).max(high);
            } else {
                output_sar = self.sar;
                if high > self.ep {
                    self.ep = high;
                    self.af = (self.af + self.acceleration).min(self.maximum);
                }
                let candidate = self.af.mul_add(self.ep - output_sar, output_sar);
                next_sar = candidate.min(prev_low).min(low);
            }
        } else if high >= self.sar {
            self.direction = 1;
            output_sar = self.ep.min(prev_low).min(low);
            self.af = self.acceleration;
            self.ep = high;
            let candidate = self.af.mul_add(self.ep - output_sar, output_sar);
            next_sar = candidate.min(prev_low).min(low);
        } else {
            output_sar = self.sar;
            if low < self.ep {
                self.ep = low;
                self.af = (self.af + self.acceleration).min(self.maximum);
            }
            let candidate = self.af.mul_add(self.ep - output_sar, output_sar);
            next_sar = candidate.max(prev_high).max(high);
        }

        self.sar = next_sar;
        self.prev_low2 = self.prev_low;
        self.prev_low = low;
        self.prev_high2 = self.prev_high;
        self.prev_high = high;

        let result = Some(SarOutput {
            sar: output_sar,
            direction: self.direction,
        });
        self.last_value = result;
        result
    }

    pub fn reset(&mut self) {
        self.sar = f64::NAN;
        self.ep = f64::NAN;
        self.af = self.acceleration;
        self.direction = 1;
        self.prev_low = f64::NAN;
        self.prev_low2 = f64::NAN;
        self.prev_high = f64::NAN;
        self.prev_high2 = f64::NAN;
        self.count = 0;
        self.last_value = None;
    }

    pub fn is_ready(&self) -> bool {
        self.count >= 1
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn value(&self) -> Option<SarOutput> {
        self.last_value
    }
}

impl IndicatorMeta for StreamingSar {
    fn name() -> &'static str {
        "SAR"
    }

    fn category() -> &'static str {
        "overlap"
    }

    fn description() -> &'static str {
        "Parabolic SAR"
    }

    fn warm_up_period(&self) -> usize {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::types::OhlcvBar;

    #[test]
    fn test_streaming_sar_first_bar() {
        let mut sar = StreamingSar::new(0.02, 0.2);
        let out = sar
            .next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0))
            .unwrap();
        assert!((out.sar - 9.0).abs() < 1e-10);
        assert_eq!(out.direction, 1);
        assert!(sar.is_ready());
    }

    #[test]
    fn test_streaming_sar_meta() {
        let sar = StreamingSar::new(0.02, 0.2);
        assert_eq!(StreamingSar::name(), "SAR");
        assert_eq!(StreamingSar::category(), "overlap");
        assert_eq!(sar.warm_up_period(), 2);
    }

    #[test]
    fn test_streaming_sar_reset() {
        let mut sar = StreamingSar::new(0.02, 0.2);
        for i in 0..5 {
            sar.next(&OhlcvBar::new(
                10.0 + i as f64,
                12.0 + i as f64,
                9.0 + i as f64,
                11.0 + i as f64,
                100.0,
            ));
        }
        assert!(sar.is_ready());
        sar.reset();
        assert!(!sar.is_ready());
        assert_eq!(sar.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let high: Vec<f64> = (0..30)
            .map(|i| 55.0 + (i as f64 * 0.3).sin() * 5.0)
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 2.0).collect();
        let batch = crate::indicators::sar(&high, &low, 0.02, 0.2).unwrap();

        let mut streaming = StreamingSar::new(0.02, 0.2);
        for i in 0..30 {
            let bar = OhlcvBar::new(0.0, high[i], low[i], 0.0, 0.0);
            let s = streaming.next(&bar).unwrap();
            assert!(
                (s.sar - batch.sar[i]).abs() < 1e-10,
                "SAR mismatch at {i}: streaming={}, batch={}",
                s.sar,
                batch.sar[i]
            );
        }
    }
}
