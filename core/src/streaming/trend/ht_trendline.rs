use crate::streaming::cycle::ht_dcperiod::HilbertState;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use std::f64::consts::PI;

/// Streaming Hilbert Transform - Instantaneous Trendline
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingHtTrendline {
    state: HilbertState,
    price_buf: [f64; 4],
    trendline: f64,
    trendline_ready: bool,
    last_value: Option<f64>,
}

impl StreamingHtTrendline {
    pub fn new() -> Self {
        Self {
            state: HilbertState::new(),
            price_buf: [0.0; 4],
            trendline: 0.0,
            trendline_ready: false,
            last_value: None,
        }
    }
}

impl Default for StreamingHtTrendline {
    fn default() -> Self { Self::new() }
}

impl StreamingIndicator for StreamingHtTrendline {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.price_buf.copy_within(1.., 0);
        self.price_buf[3] = input;

        let result = self.state.update(input).map(|(phase, _)| {
            let smooth_price = (4.0 * self.price_buf[3]
                + 3.0 * self.price_buf[2]
                + 2.0 * self.price_buf[1]
                + self.price_buf[0]) / 10.0;

            let weight = if phase.is_finite() {
                ((phase + PI / 2.0) / PI).clamp(0.1, 0.9)
            } else {
                0.5
            };

            if !self.trendline_ready {
                self.trendline = smooth_price;
                self.trendline_ready = true;
            } else {
                self.trendline = weight * smooth_price + (1.0 - weight) * self.trendline;
            }

            self.trendline
        });
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.state.reset();
        self.price_buf = [0.0; 4];
        self.trendline = 0.0;
        self.trendline_ready = false;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool { self.state.count >= 32 }
    fn count(&self) -> usize { self.state.count }
    fn value(&self) -> Option<f64> { self.last_value }
}

impl IndicatorMeta for StreamingHtTrendline {
    fn name() -> &'static str { "HT_TRENDLINE" }
    fn category() -> &'static str { "cycle" }
    fn description() -> &'static str { "Hilbert Transform - Instantaneous Trendline" }
    fn warm_up_period(&self) -> usize { 32 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_wave(n: usize, freq: f64, amp: f64, offset: f64) -> Vec<f64> {
        (0..n).map(|i| amp * (i as f64 * freq).sin() + offset).collect()
    }

    #[test]
    fn test_streaming_ht_trendline_basic() {
        let mut ht = StreamingHtTrendline::new();
        let data = sine_wave(100, 0.1, 1.0, 50.0);
        let mut last = None;
        for &v in &data { last = ht.next(v); }
        assert!(last.is_some());
        assert!(last.unwrap().is_finite());
    }

    #[test]
    fn test_streaming_ht_trendline_meta() {
        assert_eq!(StreamingHtTrendline::name(), "HT_TRENDLINE");
        assert_eq!(StreamingHtTrendline::category(), "cycle");
    }

    #[test]
    fn test_streaming_ht_trendline_reset() {
        let mut ht = StreamingHtTrendline::new();
        for i in 0..50 { ht.next(i as f64); }
        assert!(ht.is_ready());
        ht.reset();
        assert!(!ht.is_ready());
    }

    #[test]
    fn test_streaming_ht_trendline_tracks_mean() {
        let mut ht = StreamingHtTrendline::new();
        let data = sine_wave(200, 0.1, 1.0, 50.0);
        let mut vals = Vec::new();
        for &v in &data {
            if let Some(tl) = ht.next(v) { vals.push(tl); }
        }
        if !vals.is_empty() {
            let mean = vals.iter().sum::<f64>() / vals.len() as f64;
            assert!((mean - 50.0).abs() < 10.0, "mean={mean}");
        }
    }
}
