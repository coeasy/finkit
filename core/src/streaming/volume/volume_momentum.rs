use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

/// Streaming Volume Momentum = Volume - SMA(Volume, period).
///
/// Also provides volume_roc via a separate method or the `StreamingVolumeRoc` struct.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingVolumeMomentum {
    period: usize,
    buffer: Vec<f64>,
    ring_idx: usize,
    sum: f64,
    filled: bool,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingVolumeMomentum {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: vec![0.0; period],
            ring_idx: 0,
            sum: 0.0,
            filled: false,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<f64> for StreamingVolumeMomentum {
    #[inline]
    fn next(&mut self, volume: f64) -> Option<f64> {
        self.count += 1;

        let old = self.buffer[self.ring_idx];
        self.buffer[self.ring_idx] = volume;
        self.ring_idx = (self.ring_idx + 1) % self.period;

        if !self.filled {
            self.sum += volume;
            if self.count >= self.period {
                self.filled = true;
                let sma = self.sum / self.period as f64;
                let val = volume - sma;
                self.last_value = Some(val);
                return Some(val);
            }
            self.last_value = None;
            return None;
        }

        self.sum += volume - old;
        let sma = self.sum / self.period as f64;
        let val = volume - sma;
        self.last_value = Some(val);
        Some(val)
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.ring_idx = 0;
        self.sum = 0.0;
        self.filled = false;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.filled
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingVolumeMomentum {
    fn name() -> &'static str {
        "VolumeMomentum"
    }
    fn category() -> &'static str {
        "volume"
    }
    fn description() -> &'static str {
        "Volume Momentum (Volume - SMA(Volume))"
    }
    fn warm_up_period(&self) -> usize {
        self.period
    }
}

/// Streaming Volume Rate of Change = (Volume - Volume[n]) / Volume[n] * 100.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingVolumeRoc {
    period: usize,
    buffer: Vec<f64>,
    ring_idx: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingVolumeRoc {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: vec![0.0; period + 1],
            ring_idx: 0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<f64> for StreamingVolumeRoc {
    #[inline]
    fn next(&mut self, volume: f64) -> Option<f64> {
        self.count += 1;
        self.buffer[self.ring_idx] = volume;

        if self.count <= self.period {
            self.ring_idx = (self.ring_idx + 1) % (self.period + 1);
            self.last_value = None;
            return None;
        }

        let old_idx = (self.ring_idx + 1) % (self.period + 1);
        let prev = self.buffer[old_idx];
        self.ring_idx = (self.ring_idx + 1) % (self.period + 1);

        let val = if prev.abs() > 1e-15 {
            (volume - prev) / prev * 100.0
        } else {
            0.0
        };
        self.last_value = Some(val);
        Some(val)
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.ring_idx = 0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count > self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingVolumeRoc {
    fn name() -> &'static str {
        "VolumeROC"
    }
    fn category() -> &'static str {
        "volume"
    }
    fn description() -> &'static str {
        "Volume Rate of Change"
    }
    fn warm_up_period(&self) -> usize {
        self.period + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_streaming_volume_momentum_basic() {
        let mut vm = StreamingVolumeMomentum::new(3);
        assert_eq!(vm.next(100.0), None);
        assert_eq!(vm.next(120.0), None);
        let v = vm.next(110.0).unwrap();
        let sma = (100.0 + 120.0 + 110.0) / 3.0;
        assert_relative_eq!(v, 110.0 - sma, epsilon = 1e-10);
    }

    #[test]
    fn test_streaming_volume_momentum_meta() {
        assert_eq!(StreamingVolumeMomentum::name(), "VolumeMomentum");
        assert_eq!(StreamingVolumeMomentum::category(), "volume");
    }

    #[test]
    fn test_streaming_volume_momentum_reset() {
        let mut vm = StreamingVolumeMomentum::new(3);
        vm.next(100.0);
        vm.next(120.0);
        vm.next(110.0);
        assert!(vm.is_ready());
        vm.reset();
        assert!(!vm.is_ready());
        assert_eq!(vm.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_volume_momentum() {
        use crate::indicators::volume_momentum;
        let data = vec![100.0, 200.0, 150.0, 300.0, 250.0, 180.0, 220.0, 280.0, 190.0, 310.0];
        let batch = volume_momentum(&data, 5).unwrap();

        let mut streaming = StreamingVolumeMomentum::new(5);
        for i in 0..data.len() {
            let val = streaming.next(data[i]);
            if batch[i].is_nan() {
                assert!(val.is_none());
            } else {
                assert_relative_eq!(val.unwrap(), batch[i], epsilon = 1e-10);
            }
        }
    }

    #[test]
    fn test_streaming_volume_roc_basic() {
        let mut vroc = StreamingVolumeRoc::new(2);
        assert_eq!(vroc.next(100.0), None);
        assert_eq!(vroc.next(200.0), None);
        let v = vroc.next(150.0).unwrap();
        // (150 - 100) / 100 * 100 = 50
        assert_relative_eq!(v, 50.0, epsilon = 1e-10);
    }

    #[test]
    fn test_streaming_vs_batch_volume_roc() {
        use crate::indicators::volume_roc;
        let data = vec![100.0, 200.0, 150.0, 300.0, 250.0, 180.0, 220.0, 280.0, 190.0, 310.0];
        let batch = volume_roc(&data, 3).unwrap();

        let mut streaming = StreamingVolumeRoc::new(3);
        for i in 0..data.len() {
            let val = streaming.next(data[i]);
            if batch[i].is_nan() {
                assert!(val.is_none());
            } else {
                assert_relative_eq!(val.unwrap(), batch[i], epsilon = 1e-10);
            }
        }
    }
}
