use crate::streaming::price_source::PriceSource;
use crate::streaming::traits::{Ohlcv, StreamingIndicator};
use crate::{impl_indicator_meta, impl_standard_methods};

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct StreamingSma {
    period: usize,
    buffer: Vec<f64>,
    head: usize,
    len: usize,
    sum: f64,
    inv_period: f64,
    count: usize,
    last_value: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip))]
    snapshot: Option<SnapshotState>,
    last_open_time: i64,
    price_source: PriceSource,
}

#[derive(Clone, Copy)]
struct SnapshotState {
    sum: f64,
    head: usize,
    len: usize,
    count: usize,
    last_value: Option<f64>,
    last_open_time: i64,
    head_val: f64,
}

impl StreamingSma {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: vec![0.0; period],
            head: 0,
            len: 0,
            sum: 0.0,
            inv_period: 1.0 / period as f64,
            count: 0,
            last_value: None,
            snapshot: None,
            last_open_time: 0,
            price_source: PriceSource::Close,
        }
    }

    pub fn with_price_source(period: usize, price_source: PriceSource) -> Self {
        Self {
            period,
            buffer: vec![0.0; period],
            head: 0,
            len: 0,
            sum: 0.0,
            inv_period: 1.0 / period as f64,
            count: 0,
            last_value: None,
            snapshot: None,
            last_open_time: 0,
            price_source,
        }
    }

    /// Feed an OHLCV bar with forming-bar repaint support.
    ///
    /// If the bar has the same `open_time()` as the previous bar, the indicator
    /// rolls back to the pre-bar state and recomputes using the new bar's close.
    pub fn compute_bar(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        let t = bar.open_time();
        if t != 0 && t == self.last_open_time {
            if let Some(snap) = self.snapshot.take() {
                self.sum = snap.sum;
                self.head = snap.head;
                self.len = snap.len;
                self.count = snap.count;
                self.last_value = snap.last_value;
                self.last_open_time = snap.last_open_time;
                self.buffer[snap.head] = snap.head_val;
            }
        }
        self.snapshot = Some(SnapshotState {
            sum: self.sum,
            head: self.head,
            len: self.len,
            count: self.count,
            last_value: self.last_value,
            last_open_time: self.last_open_time,
            head_val: self.buffer[self.head],
        });
        self.last_open_time = t;
        self.next(self.price_source.extract(bar))
    }

    /// Public snapshot of the current internal state. Used by composed
    /// indicators (e.g. TRIMA) that need to roll back on repaint bars.
    pub fn snapshot(&self) -> SmaSnapshot {
        SmaSnapshot {
            sum: self.sum,
            head: self.head,
            len: self.len,
            count: self.count,
            last_value: self.last_value,
            last_open_time: self.last_open_time,
            head_val: self.buffer[self.head],
        }
    }

    /// Restore from a previously-taken [`SmaSnapshot`].
    pub fn restore(&mut self, snap: SmaSnapshot) {
        self.sum = snap.sum;
        self.head = snap.head;
        self.len = snap.len;
        self.count = snap.count;
        self.last_value = snap.last_value;
        self.last_open_time = snap.last_open_time;
        self.buffer[snap.head] = snap.head_val;
    }
}

/// Public, copy-able snapshot of [`StreamingSma`]. Used by composed
/// indicators to save/restore state across repaint bars.
#[derive(Clone, Copy)]
pub struct SmaSnapshot {
    pub(crate) sum: f64,
    pub(crate) head: usize,
    pub(crate) len: usize,
    pub(crate) count: usize,
    pub(crate) last_value: Option<f64>,
    pub(crate) last_open_time: i64,
    pub(crate) head_val: f64,
}

impl StreamingIndicator for StreamingSma {
    #[inline]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip(self, input))
    )]
    fn next(&mut self, input: f64) -> Option<f64> {
        #[cfg(feature = "metrics")]
        let __start = std::time::Instant::now();
        self.count += 1;
        self.sum += input;

        if self.len == self.period {
            self.sum -= self.buffer[self.head];
        } else {
            self.len += 1;
        }

        self.buffer[self.head] = input;
        self.head += 1;
        if self.head == self.period {
            self.head = 0;
        }

        let result = if self.len == self.period {
            Some(self.sum * self.inv_period)
        } else {
            None
        };
        self.last_value = result;
        #[cfg(feature = "metrics")]
        {
            crate::metrics::streaming_next("sma", result.is_some());
            crate::metrics::record_indicator_duration(
                "sma_streaming",
                __start.elapsed().as_secs_f64(),
            );
        }
        result
    }

    fn next_with_time(&mut self, input: f64, open_time: i64) -> Option<f64> {
        if open_time != 0 && open_time == self.last_open_time {
            if let Some(snap) = self.snapshot.take() {
                self.sum = snap.sum;
                self.head = snap.head;
                self.len = snap.len;
                self.count = snap.count;
                self.last_value = snap.last_value;
                self.last_open_time = snap.last_open_time;
                self.buffer[snap.head] = snap.head_val;
            }
        }
        self.snapshot = Some(SnapshotState {
            sum: self.sum,
            head: self.head,
            len: self.len,
            count: self.count,
            last_value: self.last_value,
            last_open_time: self.last_open_time,
            head_val: self.buffer[self.head],
        });
        self.last_open_time = open_time;
        self.next(input)
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.sum = 0.0;
        self.count = 0;
        self.last_value = None;
        self.snapshot = None;
        self.last_open_time = 0;
    }

    fn is_ready(&self) -> bool {
        self.len >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingSma, "SMA", "overlap", "Simple Moving Average");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;
    use crate::{test_streaming_meta, test_streaming_reset, test_streaming_vs_batch};

    #[test]
    fn test_streaming_sma_basic() {
        let mut sma = StreamingSma::new(3);
        assert_eq!(sma.next(1.0), None);
        assert_eq!(sma.next(2.0), None);
        assert!((sma.next(3.0).unwrap() - 2.0).abs() < 1e-10);
        assert!((sma.next(4.0).unwrap() - 3.0).abs() < 1e-10);
        assert!((sma.next(5.0).unwrap() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_sma_reset() {
        let mut sma = StreamingSma::new(2);
        sma.next(10.0);
        sma.next(20.0);
        assert!(sma.is_ready());
        sma.reset();
        assert!(!sma.is_ready());
        assert_eq!(sma.count(), 0);
        assert_eq!(sma.next(5.0), None);
    }

    #[test]
    fn test_streaming_sma_meta() {
        test_streaming_meta!(StreamingSma, 10, "SMA", "overlap", 10);
    }

    #[test]
    fn test_streaming_sma_repaint() {
        use crate::streaming::OhlcvBar;

        let mut sma = StreamingSma::new(3);
        sma.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 1.0, 0.0, 1000));
        sma.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 2.0, 0.0, 2000));

        sma.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 10.0, 0.0, 3000));
        sma.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 20.0, 0.0, 3000));
        let result_repaint =
            sma.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 3.0, 0.0, 3000));

        let mut sma_clean = StreamingSma::new(3);
        sma_clean.next(1.0);
        sma_clean.next(2.0);
        let result_clean = sma_clean.next(3.0);

        assert_eq!(result_repaint, result_clean);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        test_streaming_vs_batch!(StreamingSma, 14, |data, period| {
            crate::math::moving_avg::sma(data, period).unwrap()
        });
    }
}
