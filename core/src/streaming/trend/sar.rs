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

        let is_long = self.direction == 1;
        let mut current_sar = self.sar + self.af * (self.ep - self.sar);

        if is_long {
            if self.count >= 3 {
                current_sar = current_sar.min(self.prev_low);
            }
            if self.count >= 4 {
                current_sar = current_sar.min(self.prev_low2);
            }
        } else if self.count >= 3 {
            current_sar = current_sar.max(self.prev_high);
            if self.count >= 4 {
                current_sar = current_sar.max(self.prev_high2);
            }
        }

        let mut switched = false;
        if is_long {
            if low < current_sar {
                self.direction = -1;
                current_sar = self.ep;
                self.ep = low;
                self.af = self.acceleration;
                switched = true;
            }
        } else if high > current_sar {
            self.direction = 1;
            current_sar = self.ep;
            self.ep = high;
            self.af = self.acceleration;
            switched = true;
        }

        if !switched {
            if self.direction == 1 && high > self.ep {
                self.ep = high;
                self.af = (self.af + self.acceleration).min(self.maximum);
            } else if self.direction == -1 && low < self.ep {
                self.ep = low;
                self.af = (self.af + self.acceleration).min(self.maximum);
            }
        }

        self.sar = current_sar;
        self.prev_low2 = self.prev_low;
        self.prev_low = low;
        self.prev_high2 = self.prev_high;
        self.prev_high = high;

        let result = Some(SarOutput {
            sar: self.sar,
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
        let out = sar.next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0)).unwrap();
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
