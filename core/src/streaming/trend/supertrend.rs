use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};
use crate::streaming::volatility::atr::StreamingAtr;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SuperTrendOutput {
    pub supertrend: f64,
    pub direction: i32,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingSuperTrend {
    atr: StreamingAtr,
    multiplier: f64,
    prev_close: f64,
    prev_upper: f64,
    prev_lower: f64,
    prev_st: f64,
    direction: i32,
    count: usize,
    last_value: Option<SuperTrendOutput>,
}

impl StreamingSuperTrend {
    pub fn new(period: usize, multiplier: f64) -> Self {
        Self {
            atr: StreamingAtr::new(period),
            multiplier,
            prev_close: f64::NAN,
            prev_upper: f64::NAN,
            prev_lower: f64::NAN,
            prev_st: f64::NAN,
            direction: 1,
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    pub fn next(&mut self, bar: &dyn Ohlcv) -> Option<SuperTrendOutput> {
        self.count += 1;
        let close = bar.close();
        let hl2 = (bar.high() + bar.low()) / 2.0;

        let Some(atr_val) = self.atr.next((bar.high(), bar.low(), close)) else {
            self.prev_close = close;
            self.last_value = None;
            return None;
        };

        let basic_upper = hl2 + self.multiplier * atr_val;
        let basic_lower = hl2 - self.multiplier * atr_val;

        let final_upper = if self.prev_upper.is_nan()
            || basic_upper < self.prev_upper
            || self.prev_close > self.prev_upper
        {
            basic_upper
        } else {
            self.prev_upper
        };

        let final_lower = if self.prev_lower.is_nan()
            || basic_lower > self.prev_lower
            || self.prev_close < self.prev_lower
        {
            basic_lower
        } else {
            self.prev_lower
        };

        let st = if self.prev_st.is_nan() || self.prev_st == self.prev_upper {
            if close <= final_upper {
                final_upper
            } else {
                final_lower
            }
        } else if close >= final_lower {
            final_lower
        } else {
            final_upper
        };

        self.direction = if st == final_upper { -1 } else { 1 };
        self.prev_close = close;
        self.prev_upper = final_upper;
        self.prev_lower = final_lower;
        self.prev_st = st;

        let result = Some(SuperTrendOutput {
            supertrend: st,
            direction: self.direction,
        });
        self.last_value = result;
        result
    }

    pub fn reset(&mut self) {
        self.atr.reset();
        self.prev_close = f64::NAN;
        self.prev_upper = f64::NAN;
        self.prev_lower = f64::NAN;
        self.prev_st = f64::NAN;
        self.direction = 1;
        self.count = 0;
        self.last_value = None;
    }

    pub fn is_ready(&self) -> bool {
        self.atr.is_ready()
    }
    pub fn count(&self) -> usize {
        self.count
    }

    pub fn value(&self) -> Option<SuperTrendOutput> {
        self.last_value
    }
}

impl IndicatorMeta for StreamingSuperTrend {
    fn name() -> &'static str {
        "SuperTrend"
    }
    fn category() -> &'static str {
        "volatility"
    }
    fn description() -> &'static str {
        "Super Trend Indicator"
    }
    fn warm_up_period(&self) -> usize {
        self.atr.warm_up_period()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::types::OhlcvBar;

    #[test]
    fn test_streaming_supertrend_basic() {
        let mut st = StreamingSuperTrend::new(3, 2.0);
        for i in 0..10 {
            let bar = OhlcvBar::new(
                10.0 + i as f64,
                12.0 + i as f64,
                9.0 + i as f64,
                11.0 + i as f64,
                100.0,
            );
            if let Some(out) = st.next(&bar) {
                assert!(!out.supertrend.is_nan());
                assert!(out.direction == 1 || out.direction == -1);
            }
        }
    }

    #[test]
    fn test_streaming_supertrend_uptrend() {
        let mut st = StreamingSuperTrend::new(3, 2.0);
        let mut last = None;
        for i in 0..20 {
            let bar = OhlcvBar::new(
                10.0 + i as f64,
                12.0 + i as f64,
                9.0 + i as f64,
                11.0 + i as f64,
                100.0,
            );
            last = st.next(&bar);
        }
        assert_eq!(last.unwrap().direction, 1);
    }

    #[test]
    fn test_streaming_supertrend_meta() {
        assert_eq!(StreamingSuperTrend::name(), "SuperTrend");
    }

    #[test]
    fn test_streaming_supertrend_reset() {
        let mut st = StreamingSuperTrend::new(3, 2.0);
        for i in 0..10 {
            st.next(&OhlcvBar::new(
                10.0 + i as f64,
                12.0 + i as f64,
                9.0 + i as f64,
                11.0 + i as f64,
                100.0,
            ));
        }
        assert!(st.is_ready());
        st.reset();
        assert!(!st.is_ready());
    }
}
