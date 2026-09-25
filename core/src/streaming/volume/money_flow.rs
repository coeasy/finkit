use crate::impl_indicator_meta;
use crate::impl_standard_methods;
use crate::streaming::traits::StreamingIndicator;

/// Streaming Money Flow (资金流量).
///
/// The running sum of `typical_price * volume` over a fixed window, where
/// `typical_price = (high + low + close) / 3`. This mirrors
/// [`crate::indicators::astock::money_flow`] bar for bar, which is why it
/// carries the `astock` category rather than `volume`: it is the A-share
/// money-flow convention, not the universal Chaikin/OBV family.
///
/// The first `period - 1` bars yield `None`, matching the batch path's leading
/// `NaN` run. A non-finite input propagates rather than being skipped: `NaN` is
/// absorbing for the running sum in the batch accumulator too, so suppressing
/// it here would make the two disagree.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMoneyFlow {
    period: usize,
    buffer: Vec<f64>,
    head: usize,
    len: usize,
    sum: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingMoneyFlow {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: vec![f64::NAN; period],
            head: 0,
            len: 0,
            sum: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64, f64, f64)> for StreamingMoneyFlow {
    #[inline]
    fn next(&mut self, input: (f64, f64, f64, f64)) -> Option<f64> {
        let (high, low, close, volume) = input;
        self.count += 1;

        // Kept in the batch path's exact operand order so the two agree bit for
        // bit; reassociating this expression would break the parity test below.
        let typical_volume = (high + low + close) / 3.0 * volume;

        let cap = self.period;
        if self.len < cap {
            self.buffer[(self.head + self.len) % cap] = typical_volume;
            self.len += 1;
            self.sum += typical_volume;
        } else {
            let oldest = self.buffer[self.head];
            self.buffer[self.head] = typical_volume;
            self.head = (self.head + 1) % cap;
            self.sum += typical_volume - oldest;
        }

        if self.len < self.period {
            self.last_value = None;
            return None;
        }
        self.last_value = Some(self.sum);
        self.last_value
    }

    #[inline]
    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.sum = 0.0;
        self.count = 0;
        self.last_value = None;
        for value in &mut self.buffer {
            *value = f64::NAN;
        }
    }

    #[inline]
    fn is_ready(&self) -> bool {
        self.len >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(
    StreamingMoneyFlow,
    "MONEY_FLOW",
    "astock",
    "Rolling sum of typical price × volume (资金流量)"
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;

    fn oscillating_input(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = (0..n)
            .map(|i| 55.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 2.0).collect();
        let close: Vec<f64> = high
            .iter()
            .zip(low.iter())
            .map(|(h, l)| (h + l) / 2.0)
            .collect();
        let volume: Vec<f64> = (0..n).map(|i| 1000.0 + i as f64 * 10.0).collect();
        (high, low, close, volume)
    }

    #[test]
    fn test_streaming_money_flow_basic() {
        let mut mf = StreamingMoneyFlow::new(3);
        assert_eq!(mf.next((12.0, 10.0, 11.0, 100.0)), None);
        assert_eq!(mf.next((13.0, 11.0, 12.0, 150.0)), None);
        let value = mf.next((14.0, 12.0, 13.0, 200.0)).unwrap();
        // (12+10+11)/3*100 = 1100, (13+11+12)/3*150 = 1800, (14+12+13)/3*200 = 2600
        assert!((value - 5500.0).abs() < 1e-9, "got {value}");
    }

    #[test]
    fn test_streaming_money_flow_meta() {
        let mf = StreamingMoneyFlow::new(14);
        assert_eq!(StreamingMoneyFlow::name(), "MONEY_FLOW");
        assert_eq!(StreamingMoneyFlow::category(), "astock");
        assert_eq!(mf.warm_up_period(), 14);
    }

    #[test]
    fn test_streaming_money_flow_reset() {
        let mut mf = StreamingMoneyFlow::new(3);
        for i in 0..5 {
            mf.next((10.0 + i as f64, 8.0 + i as f64, 9.0 + i as f64, 100.0));
        }
        assert!(mf.is_ready());
        mf.reset();
        assert!(!mf.is_ready());
        assert_eq!(mf.count(), 0);
        assert_eq!(mf.value(), None);
    }

    /// The claim in `docs/indicator_registry.json` is `streaming: true` for
    /// `MONEY_FLOW`. Assert it is actually backed: the streaming path must
    /// reproduce the batch path exactly, including the leading `None` run.
    #[test]
    fn test_streaming_money_flow_matches_batch() {
        let n = 120;
        let (high, low, close, volume) = oscillating_input(n);
        let period = 14;

        let batch =
            crate::indicators::astock::money_flow(&high, &low, &close, &volume, period).unwrap();
        let mut streaming = StreamingMoneyFlow::new(period);

        let mut emitted = 0;
        for i in 0..n {
            let got = streaming.next((high[i], low[i], close[i], volume[i]));
            if i < period - 1 {
                assert_eq!(got, None, "bar {i} must not be ready yet");
                continue;
            }
            let got = got.expect("streaming must emit once warmed up");
            assert_eq!(
                got, batch[i],
                "bar {i}: streaming {got} != batch {}",
                batch[i]
            );
            emitted += 1;
        }
        assert_eq!(emitted, n - (period - 1));
    }

    /// A non-finite input must poison the window exactly as it does on the
    /// batch path -- if the streaming side skipped it, the two would silently
    /// diverge for the rest of the window.
    #[test]
    fn test_streaming_money_flow_propagates_nan_like_batch() {
        let n = 40;
        let (mut high, low, close, volume) = oscillating_input(n);
        let period = 5;
        high[10] = f64::NAN;

        let batch =
            crate::indicators::astock::money_flow(&high, &low, &close, &volume, period).unwrap();
        let mut streaming = StreamingMoneyFlow::new(period);

        for i in 0..n {
            let got = streaming.next((high[i], low[i], close[i], volume[i]));
            if i < period - 1 {
                assert_eq!(got, None);
                continue;
            }
            let got = got.expect("must emit once warmed up");
            assert_eq!(
                got.is_nan(),
                batch[i].is_nan(),
                "bar {i}: NaN-ness differs (streaming {got}, batch {})",
                batch[i]
            );
            if !batch[i].is_nan() {
                assert_eq!(got, batch[i], "bar {i}");
            }
        }
    }
}
