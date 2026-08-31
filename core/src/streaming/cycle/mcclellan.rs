use crate::impl_standard_methods;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming McClellan Oscillator.
///
/// McClellan = EMA(AD_diff, short) - EMA(AD_diff, long)
/// where AD_diff = advances - declines
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMcClellanOscillator {
    short_period: usize,
    long_period: usize,
    short_ema: StreamingEma,
    long_ema: StreamingEma,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingMcClellanOscillator {
    pub fn new(short_period: usize, long_period: usize) -> Self {
        Self {
            short_period,
            long_period,
            short_ema: StreamingEma::new(short_period),
            long_ema: StreamingEma::new(long_period),
            count: 0,
            last_value: None,
        }
    }

    pub fn default_periods() -> Self {
        Self::new(19, 39)
    }
}

/// Input: AD difference (advances - declines)
impl StreamingIndicator for StreamingMcClellanOscillator {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let short_val = self.short_ema.next(input);
        let long_val = self.long_ema.next(input);

        match (short_val, long_val) {
            (Some(s), Some(l)) => {
                let val = s - l;
                self.last_value = Some(val);
                Some(val)
            }
            _ => {
                self.last_value = None;
                None
            }
        }
    }

    fn reset(&mut self) {
        self.short_ema.reset();
        self.long_ema.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.long_period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingMcClellanOscillator {
    fn name() -> &'static str {
        "MCCLELLAN_OSCILLATOR"
    }
    fn category() -> &'static str {
        "breadth"
    }
    fn description() -> &'static str {
        "McClellan Oscillator"
    }
    fn warm_up_period(&self) -> usize {
        self.long_period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_mcclellan() {
        let mut mc = StreamingMcClellanOscillator::new(3, 5);
        let data = [10.0, 20.0, -5.0, 15.0, -10.0, 8.0, 12.0];
        let mut results = Vec::new();
        for &d in &data {
            if let Some(val) = mc.next(d) {
                results.push(val);
            }
        }
        assert!(!results.is_empty());
    }

    #[test]
    fn test_streaming_mcclellan_reset() {
        let mut mc = StreamingMcClellanOscillator::new(3, 5);
        for i in 0..10 {
            mc.next(i as f64);
        }
        mc.reset();
        assert!(!mc.is_ready());
    }
}
