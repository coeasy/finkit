use crate::streaming::momentum::rsi::StreamingRsi;
use crate::streaming::momentum::stoch::StochOutput;
use crate::streaming::overlap::sma::StreamingSma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use std::collections::VecDeque;

/// Streaming Stochastic RSI.
///
/// Applies Stochastic formula to RSI values.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingStochRsi {
    rsi_period: usize,
    stoch_period: usize,
    rsi: StreamingRsi,
    rsi_buffer: VecDeque<f64>,
    k_sma: StreamingSma,
    d_sma: StreamingSma,
    count: usize,
    last_value: Option<StochOutput>,
}

impl StreamingStochRsi {
    pub fn new(
        rsi_period: usize,
        stoch_period: usize,
        fastk_period: usize,
        fastd_period: usize,
    ) -> Self {
        Self {
            rsi_period,
            stoch_period,
            rsi: StreamingRsi::new(rsi_period),
            rsi_buffer: VecDeque::with_capacity(stoch_period),
            k_sma: StreamingSma::new(fastk_period),
            d_sma: StreamingSma::new(fastd_period),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingStochRsi {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;

        let rsi_val = self.rsi.next(input);
        let Some(rsi) = rsi_val else {
            self.last_value = None;
            return None;
        };

        self.rsi_buffer.push_back(rsi);
        if self.rsi_buffer.len() > self.stoch_period {
            self.rsi_buffer.pop_front();
        }

        if self.rsi_buffer.len() < self.stoch_period {
            self.last_value = None;
            return None;
        }

        let max = self
            .rsi_buffer
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let min = self
            .rsi_buffer
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);

        let range = max - min;
        let raw_k = if range.abs() > 1e-15 {
            ((rsi - min) / range) * 100.0
        } else {
            50.0
        };

        let k = self.k_sma.next(raw_k);
        let k_val = k.unwrap_or(f64::NAN);

        let d = if let Some(kv) = k {
            self.d_sma.next(kv)
        } else {
            None
        };

        let result = StochOutput {
            k: k_val,
            d: d.unwrap_or(f64::NAN),
        };
        self.last_value = Some(result);

        if k_val.is_nan() {
            None
        } else {
            Some(k_val)
        }
    }

    fn reset(&mut self) {
        self.rsi.reset();
        self.rsi_buffer.clear();
        self.k_sma.reset();
        self.d_sma.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.rsi_buffer.len() >= self.stoch_period && self.k_sma.is_ready()
    }

    fn count(&self) -> usize {
        self.count
    }

    fn value(&self) -> Option<f64> {
        self.last_value.map(|o| o.k).filter(|v| !v.is_nan())
    }
}

impl IndicatorMeta for StreamingStochRsi {
    fn name() -> &'static str {
        "STOCHRSI"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Stochastic RSI"
    }
    fn warm_up_period(&self) -> usize {
        self.rsi_period + self.stoch_period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_stoch_rsi_basic() {
        let mut sr = StreamingStochRsi::new(14, 14, 3, 3);
        let data: Vec<f64> = (0..80)
            .map(|i| 100.0 + (i as f64 * 0.2).sin() * 10.0)
            .collect();
        let mut last = None;
        for &d in &data {
            if let Some(v) = sr.next(d) {
                last = Some(v);
            }
        }
        assert!(last.is_some(), "StochRSI should produce output");
    }

    #[test]
    fn test_streaming_stoch_rsi_reset() {
        let mut sr = StreamingStochRsi::new(14, 14, 3, 3);
        for i in 0..80 {
            sr.next(100.0 + (i as f64 * 0.2).sin() * 10.0);
        }
        assert!(sr.is_ready());
        sr.reset();
        assert!(!sr.is_ready());
        assert_eq!(sr.count(), 0);
    }

    #[test]
    fn test_streaming_stoch_rsi_meta() {
        let sr = StreamingStochRsi::new(14, 14, 3, 3);
        assert_eq!(StreamingStochRsi::name(), "STOCHRSI");
        assert_eq!(sr.warm_up_period(), 28);
    }
}
