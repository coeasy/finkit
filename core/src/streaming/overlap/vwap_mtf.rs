use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};

/// Input for StreamingVwapMtf: an OHLCV bar plus a session_start flag.
pub struct VwapMtfInput<'a> {
    pub bar: &'a dyn Ohlcv,
    pub session_start: bool,
}

/// Streaming Multi-timeframe VWAP with session-based resets.
///
/// Accumulates typical_price × volume and volume sums; resets when
/// `session_start` is `true` in the input, anchoring VWAP to the
/// session boundary (day/week/month/quarter).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingVwapMtf {
    cumulative_tp_vol: f64,
    cumulative_vol: f64,
    count: usize,
    last_value: Option<f64>,
}

impl Default for StreamingVwapMtf {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingVwapMtf {
    pub fn new() -> Self {
        Self {
            cumulative_tp_vol: 0.0,
            cumulative_vol: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<VwapMtfInput<'_>> for StreamingVwapMtf {
    #[inline]
    fn next(&mut self, input: VwapMtfInput<'_>) -> Option<f64> {
        self.count += 1;

        if input.session_start {
            self.cumulative_tp_vol = 0.0;
            self.cumulative_vol = 0.0;
        }

        let tp = (input.bar.high() + input.bar.low() + input.bar.close()) / 3.0;
        self.cumulative_tp_vol += tp * input.bar.volume();
        self.cumulative_vol += input.bar.volume();

        let result = if self.cumulative_vol.abs() > 1e-15 {
            Some(self.cumulative_tp_vol / self.cumulative_vol)
        } else {
            None
        };
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.cumulative_tp_vol = 0.0;
        self.cumulative_vol = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= 1
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingVwapMtf {
    fn name() -> &'static str {
        "VWAP_MTF"
    }
    fn category() -> &'static str {
        "volume"
    }
    fn description() -> &'static str {
        "Multi-timeframe Volume Weighted Average Price with session resets"
    }
    fn warm_up_period(&self) -> usize {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::types::OhlcvBar;
    use approx::assert_relative_eq;

    #[test]
    fn test_streaming_vwap_mtf_basic() {
        let mut mtf = StreamingVwapMtf::new();
        let bar1 = OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0);
        let v1 = mtf
            .next(VwapMtfInput {
                bar: &bar1,
                session_start: true,
            })
            .unwrap();
        let tp1 = (12.0 + 9.0 + 11.0) / 3.0;
        assert_relative_eq!(v1, tp1, epsilon = 1e-10);

        let bar2 = OhlcvBar::new(11.0, 13.0, 10.0, 12.0, 200.0);
        let v2 = mtf
            .next(VwapMtfInput {
                bar: &bar2,
                session_start: false,
            })
            .unwrap();
        let tp2 = (13.0 + 10.0 + 12.0) / 3.0;
        let expected = (tp1 * 100.0 + tp2 * 200.0) / 300.0;
        assert_relative_eq!(v2, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_streaming_vwap_mtf_session_reset() {
        let mut mtf = StreamingVwapMtf::new();
        let bar1 = OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0);
        mtf.next(VwapMtfInput {
            bar: &bar1,
            session_start: true,
        });

        let bar2 = OhlcvBar::new(11.0, 13.0, 10.0, 12.0, 200.0);
        mtf.next(VwapMtfInput {
            bar: &bar2,
            session_start: false,
        });

        // New session
        let bar3 = OhlcvBar::new(20.0, 22.0, 19.0, 21.0, 500.0);
        let v3 = mtf
            .next(VwapMtfInput {
                bar: &bar3,
                session_start: true,
            })
            .unwrap();
        let tp3 = (22.0 + 19.0 + 21.0) / 3.0;
        assert_relative_eq!(v3, tp3, epsilon = 1e-10);
    }

    #[test]
    fn test_streaming_vwap_mtf_meta() {
        assert_eq!(StreamingVwapMtf::name(), "VWAP_MTF");
        assert_eq!(StreamingVwapMtf::category(), "volume");
    }

    #[test]
    fn test_streaming_vwap_mtf_reset() {
        let mut mtf = StreamingVwapMtf::new();
        let bar1 = OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0);
        mtf.next(VwapMtfInput {
            bar: &bar1,
            session_start: true,
        });
        assert!(mtf.is_ready());
        mtf.reset();
        assert!(!mtf.is_ready());
        assert_eq!(mtf.count(), 0);
        assert_eq!(mtf.value(), None);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        use crate::indicators::vwap_mtf;
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];
        let volume = vec![
            100.0, 200.0, 150.0, 300.0, 250.0, 180.0, 220.0, 280.0, 190.0, 310.0,
        ];
        let session_start = vec![
            true, false, false, false, true, false, false, true, false, false,
        ];

        let batch = vwap_mtf(&high, &low, &close, &volume, &session_start).unwrap();

        let mut streaming = StreamingVwapMtf::new();
        for i in 0..10 {
            let bar = OhlcvBar::new(close[i], high[i], low[i], close[i], volume[i]);
            let val = streaming
                .next(VwapMtfInput {
                    bar: &bar,
                    session_start: session_start[i],
                })
                .unwrap();
            assert_relative_eq!(val, batch[i], epsilon = 1e-10, max_relative = 1e-10);
        }
    }
}
