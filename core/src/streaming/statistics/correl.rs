use std::collections::VecDeque;

use crate::impl_indicator_meta;

/// Streaming Pearson Correlation Coefficient with rolling window.
///
/// Supports `next_pair(x, y)` dual-input method.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingCorrel {
    period: usize,
    x_window: VecDeque<f64>,
    y_window: VecDeque<f64>,
    sum_x: f64,
    sum_y: f64,
    sum_xy: f64,
    sum_x2: f64,
    sum_y2: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingCorrel {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            x_window: VecDeque::with_capacity(period),
            y_window: VecDeque::with_capacity(period),
            sum_x: 0.0,
            sum_y: 0.0,
            sum_xy: 0.0,
            sum_x2: 0.0,
            sum_y2: 0.0,
            count: 0,
            last_value: None,
        }
    }

    /// Feed a pair of values.
    pub fn next_pair(&mut self, x: f64, y: f64) -> Option<f64> {
        self.count += 1;

        // If window is full, evict the oldest pair and subtract from accumulators
        if self.x_window.len() == self.period {
            let old_x = self.x_window.pop_front().unwrap();
            let old_y = self.y_window.pop_front().unwrap();
            self.sum_x -= old_x;
            self.sum_y -= old_y;
            self.sum_xy -= old_x * old_y;
            self.sum_x2 -= old_x * old_x;
            self.sum_y2 -= old_y * old_y;
        }

        // Push new pair and add to accumulators
        self.x_window.push_back(x);
        self.y_window.push_back(y);
        self.sum_x += x;
        self.sum_y += y;
        self.sum_xy += x * y;
        self.sum_x2 += x * x;
        self.sum_y2 += y * y;

        let result = if self.x_window.len() == self.period {
            let n = self.period as f64;
            let _mean_x = self.sum_x / n;
            let _mean_y = self.sum_y / n;

            let cov = self.sum_xy - self.sum_x * self.sum_y / n;
            let var_x = self.sum_x2 - self.sum_x * self.sum_x / n;
            let var_y = self.sum_y2 - self.sum_y * self.sum_y / n;

            let denom = (var_x * var_y).sqrt();
            if denom.abs() > 1e-15 {
                Some((cov / denom).clamp(-1.0, 1.0))
            } else {
                None
            }
        } else {
            None
        };
        self.last_value = result;
        result
    }

    pub fn reset(&mut self) {
        self.x_window.clear();
        self.y_window.clear();
        self.sum_x = 0.0;
        self.sum_y = 0.0;
        self.sum_xy = 0.0;
        self.sum_x2 = 0.0;
        self.sum_y2 = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    pub fn is_ready(&self) -> bool { self.x_window.len() >= self.period }
    pub fn count(&self) -> usize { self.count }
    pub fn value(&self) -> Option<f64> { self.last_value }
}

impl_indicator_meta!(StreamingCorrel, "CORREL", "statistic", "Rolling Pearson Correlation");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;

    #[test]
    fn test_streaming_correl_perfect_positive() {
        let mut c = StreamingCorrel::new(3);
        c.next_pair(1.0, 10.0);
        c.next_pair(2.0, 20.0);
        let v = c.next_pair(3.0, 30.0).unwrap();
        assert!((v - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_correl_perfect_negative() {
        let mut c = StreamingCorrel::new(3);
        c.next_pair(1.0, 30.0);
        c.next_pair(2.0, 20.0);
        let v = c.next_pair(3.0, 10.0).unwrap();
        assert!((v - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_correl_meta() {
        assert_eq!(StreamingCorrel::name(), "CORREL");
        assert_eq!(StreamingCorrel::category(), "statistic");
    }

    #[test]
    fn test_streaming_correl_reset() {
        let mut c = StreamingCorrel::new(3);
        c.next_pair(1.0, 2.0); c.next_pair(2.0, 4.0); c.next_pair(3.0, 6.0);
        assert!(c.is_ready());
        c.reset();
        assert!(!c.is_ready());
        assert_eq!(c.count(), 0);
    }

    #[test]
    fn test_streaming_correl_vs_batch() {
        let x: Vec<f64> = (0..50).map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0).collect();
        let y: Vec<f64> = (0..50).map(|i| 30.0 + (i as f64 * 0.15).cos() * 5.0).collect();
        let period = 10;

        let batch = crate::indicators::correlation(&x, &y, period).unwrap();

        let mut streaming = StreamingCorrel::new(period);
        for i in 0..50 {
            if let (Some(s), false) = (streaming.next_pair(x[i], y[i]), batch[i].is_nan()) {
                assert!(
                    (s - batch[i]).abs() < 1e-8,
                    "Mismatch at {i}: streaming={s}, batch={}",
                    batch[i]
                );
            }
        }
    }
}
