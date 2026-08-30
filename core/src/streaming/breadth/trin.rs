use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

/// Streaming TRIN (Arms Index / Short-Term Trading Index).
///
/// TRIN = (Advances/Declines) / (Advancing Volume/Declining Volume)
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingTrin {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingTrin {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl Default for StreamingTrin {
    fn default() -> Self { Self::new() }
}

/// Input: (advances, declines, advancing_volume, declining_volume)
impl StreamingIndicator<(f64, f64, f64, f64)> for StreamingTrin {
    #[inline]
    fn next(&mut self, input: (f64, f64, f64, f64)) -> Option<f64> {
        let (advances, declines, adv_vol, dec_vol) = input;
        self.count += 1;

        if declines.abs() < 1e-15 || dec_vol.abs() < 1e-15 {
            let val = 0.0;
            self.last_value = Some(val);
            return Some(val);
        }

        let ad_ratio = advances / declines;
        let vol_ratio = adv_vol / dec_vol;

        let val = if vol_ratio.abs() > 1e-15 {
            ad_ratio / vol_ratio
        } else {
            0.0
        };

        self.last_value = Some(val);
        Some(val)
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= 1
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingTrin {
    fn name() -> &'static str { "TRIN" }
    fn category() -> &'static str { "breadth" }
    fn description() -> &'static str { "TRIN (Arms Index)" }
    fn warm_up_period(&self) -> usize { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_trin() {
        let mut trin = StreamingTrin::new();
        // ad_ratio = 200/100 = 2.0, vol_ratio = 5000/3000 = 1.667
        // TRIN = 2.0 / 1.667 = 1.2
        let val = trin.next((200.0, 100.0, 5000.0, 3000.0));
        assert!((val.unwrap() - 1.2).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_trin_zero_declines() {
        let mut trin = StreamingTrin::new();
        let val = trin.next((200.0, 0.0, 5000.0, 3000.0));
        assert_eq!(val, Some(0.0));
    }

    #[test]
    fn test_streaming_trin_reset() {
        let mut trin = StreamingTrin::new();
        trin.next((200.0, 100.0, 5000.0, 3000.0));
        trin.reset();
        assert!(!trin.is_ready());
    }
}
