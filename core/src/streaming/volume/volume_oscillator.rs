use crate::impl_standard_methods;
use crate::streaming::overlap::sma::StreamingSma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming Volume Oscillator.
///
/// VO = (SMA(volume, fast) - SMA(volume, slow)) / SMA(volume, slow) * 100
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingVolumeOscillator {
    fast_period: usize,
    slow_period: usize,
    fast_sma: StreamingSma,
    slow_sma: StreamingSma,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingVolumeOscillator {
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self {
            fast_period,
            slow_period,
            fast_sma: StreamingSma::new(fast_period),
            slow_sma: StreamingSma::new(slow_period),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingVolumeOscillator {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let fast = self.fast_sma.next(input);
        let slow = self.slow_sma.next(input);

        match (fast, slow) {
            (Some(f), Some(s)) if s.abs() > 1e-15 => {
                let val = (f - s) / s * 100.0;
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
        self.fast_sma.reset();
        self.slow_sma.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.slow_period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingVolumeOscillator {
    fn name() -> &'static str {
        "VOLUME_OSCILLATOR"
    }
    fn category() -> &'static str {
        "volume"
    }
    fn description() -> &'static str {
        "Volume Oscillator"
    }
    fn warm_up_period(&self) -> usize {
        self.slow_period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_volume_oscillator() {
        let mut vo = StreamingVolumeOscillator::new(3, 5);
        let volumes = [100.0, 120.0, 110.0, 130.0, 140.0, 150.0, 160.0];
        let mut results = Vec::new();
        for &v in &volumes {
            if let Some(val) = vo.next(v) {
                results.push(val);
            }
        }
        assert!(!results.is_empty());
    }

    #[test]
    fn test_streaming_volume_oscillator_reset() {
        let mut vo = StreamingVolumeOscillator::new(3, 5);
        for i in 0..10 {
            vo.next(100.0 + i as f64 * 10.0);
        }
        vo.reset();
        assert!(!vo.is_ready());
        assert_eq!(vo.count(), 0);
    }
}
