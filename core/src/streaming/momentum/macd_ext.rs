//! Streaming MACD with controllable MA type (MACDEXT).
//!
//! This is the streaming counterpart to the batch
//! `indicators::momentum::macdext` function. The full TA-Lib MACDEXT allows
//! each MA (fast, slow, signal) to be any of the supported `MaType` values.
//!
//! Streaming-only support: this implementation currently supports
//! `MaType::Sma` and `MaType::Ema` for the fast and slow lines. Other
//! MA types (Wma, Dema, Tema, ...) are technically usable in the batch
//! function, but the streaming versions of those are heavier-weight and
//! have been intentionally omitted from this first pass. The signal line
//! is always an `Ema` for now, matching the typical TA-Lib usage.

use crate::impl_standard_methods;
use crate::indicators::overlap::MaType;
use crate::streaming::momentum::macd::MacdOutput;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::overlap::sma::StreamingSma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Result of constructing a [`StreamingMacdExt`] with an unsupported MA type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedMaType(pub MaType);

/// Streaming MACDEXT. See module docs for the supported MA type subset.
#[allow(dead_code)]
pub struct StreamingMacdExt {
    fast_kind: MaKind,
    slow_kind: MaKind,
    fast_state: MaState,
    slow_state: MaState,
    signal_ema: StreamingEma,
    count: usize,
    last_value: Option<MacdOutput>,
    snapshot: Option<SnapshotState>,
    last_open_time: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MaKind {
    Sma,
    Ema,
}

#[derive(Clone)]
enum MaState {
    Sma(StreamingSma),
    Ema(StreamingEma),
}

#[derive(Clone)]
struct SnapshotState {
    fast_state: MaState,
    slow_state: MaState,
    signal_ema: StreamingEma,
    count: usize,
    last_value: Option<MacdOutput>,
    last_open_time: i64,
}

impl MaState {
    fn new(kind: MaKind, period: usize) -> Self {
        match kind {
            MaKind::Sma => MaState::Sma(StreamingSma::new(period)),
            MaKind::Ema => MaState::Ema(StreamingEma::new(period)),
        }
    }

    fn next(&mut self, value: f64) -> Option<f64> {
        match self {
            MaState::Sma(s) => s.next(value),
            MaState::Ema(e) => e.next(value),
        }
    }

    fn reset(&mut self) {
        match self {
            MaState::Sma(s) => s.reset(),
            MaState::Ema(e) => e.reset(),
        }
    }

    fn is_ready(&self) -> bool {
        match self {
            MaState::Sma(s) => s.is_ready(),
            MaState::Ema(e) => e.is_ready(),
        }
    }
}

fn kind_from(ma_type: MaType) -> Result<MaKind, UnsupportedMaType> {
    match ma_type {
        MaType::Sma => Ok(MaKind::Sma),
        MaType::Ema => Ok(MaKind::Ema),
        other => Err(UnsupportedMaType(other)),
    }
}

impl StreamingMacdExt {
    /// Construct a new streaming MACDEXT. The signal line is always EMA.
    ///
    /// Returns `Err(UnsupportedMaType)` if the supplied fast or slow MA type
    /// is not yet supported in the streaming implementation.
    pub fn new(
        fast_period: usize,
        fast_ma_type: MaType,
        slow_period: usize,
        slow_ma_type: MaType,
        signal_period: usize,
    ) -> Result<Self, UnsupportedMaType> {
        let fast_kind = kind_from(fast_ma_type)?;
        let slow_kind = kind_from(slow_ma_type)?;
        Ok(Self {
            fast_kind,
            slow_kind,
            fast_state: MaState::new(fast_kind, fast_period),
            slow_state: MaState::new(slow_kind, slow_period),
            signal_ema: StreamingEma::new(signal_period),
            count: 0,
            last_value: None,
            snapshot: None,
            last_open_time: 0,
        })
    }

    /// Feed an OHLCV bar with forming-bar repaint support.
    ///
    /// A repeated non-zero `open_time()` replaces the previous forming bar.
    /// The pre-bar state is restored before calculating the replacement, so
    /// repeated quote updates do not accumulate duplicate observations.
    pub fn compute_bar(&mut self, bar: &dyn crate::streaming::traits::Ohlcv) -> Option<MacdOutput> {
        let timestamp = bar.open_time();
        if timestamp != 0 && timestamp == self.last_open_time {
            if let Some(snapshot) = self.snapshot.take() {
                self.fast_state = snapshot.fast_state;
                self.slow_state = snapshot.slow_state;
                self.signal_ema = snapshot.signal_ema;
                self.count = snapshot.count;
                self.last_value = snapshot.last_value;
                self.last_open_time = snapshot.last_open_time;
            }
        }

        self.snapshot = Some(SnapshotState {
            fast_state: self.fast_state.clone(),
            slow_state: self.slow_state.clone(),
            signal_ema: self.signal_ema.clone(),
            count: self.count,
            last_value: self.last_value,
            last_open_time: self.last_open_time,
        });
        self.last_open_time = timestamp;
        self.next(bar.close())
    }
}

impl StreamingIndicator<f64, MacdOutput> for StreamingMacdExt {
    #[inline]
    fn next(&mut self, input: f64) -> Option<MacdOutput> {
        self.count += 1;
        let fast = self.fast_state.next(input);
        let slow = self.slow_state.next(input);
        let (Some(fast), Some(slow)) = (fast, slow) else {
            self.last_value = None;
            return None;
        };
        let macd = fast - slow;
        let Some(signal) = self.signal_ema.next(macd) else {
            self.last_value = None;
            return None;
        };
        let histogram = macd - signal;
        let result = Some(MacdOutput {
            macd,
            signal,
            histogram,
        });
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.fast_state.reset();
        self.slow_state.reset();
        self.signal_ema.reset();
        self.count = 0;
        self.last_value = None;
        self.snapshot = None;
        self.last_open_time = 0;
    }

    fn is_ready(&self) -> bool {
        self.fast_state.is_ready() && self.slow_state.is_ready() && self.signal_ema.is_ready()
    }

    impl_standard_methods!(output = MacdOutput);
}

impl IndicatorMeta for StreamingMacdExt {
    fn name() -> &'static str {
        "MACDEXT"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "MACD with controllable MA type (Sma/Ema supported)"
    }
    fn warm_up_period(&self) -> usize {
        // Conservative upper bound.
        self.count().max(35)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_macd_ext_basic_ema() {
        let mut m = StreamingMacdExt::new(12, MaType::Ema, 26, MaType::Ema, 9).unwrap();
        for i in 0..80 {
            let v = m.next(50.0 + (i as f64 * 0.1).sin() * 5.0);
            if m.is_ready() {
                let v = v.unwrap();
                assert!(!v.macd.is_nan());
                assert!(!v.signal.is_nan());
                assert!(!v.histogram.is_nan());
            }
        }
    }

    #[test]
    fn test_streaming_macd_ext_basic_sma() {
        let mut m = StreamingMacdExt::new(5, MaType::Sma, 10, MaType::Sma, 3).unwrap();
        for i in 0..50 {
            let v = m.next(50.0 + (i as f64 * 0.1).cos() * 5.0);
            if m.is_ready() {
                let v = v.unwrap();
                assert!(!v.macd.is_nan());
            }
        }
    }

    #[test]
    fn test_streaming_macd_ext_unsupported_ma() {
        // Wma is not yet supported in the streaming version.
        let res = StreamingMacdExt::new(12, MaType::Wma, 26, MaType::Ema, 9);
        assert!(res.is_err());
        let res = StreamingMacdExt::new(12, MaType::Ema, 26, MaType::Tema, 9);
        assert!(res.is_err());
    }

    #[test]
    fn test_streaming_macd_ext_meta() {
        assert_eq!(StreamingMacdExt::name(), "MACDEXT");
        assert_eq!(StreamingMacdExt::category(), "momentum");
    }

    #[test]
    fn test_streaming_macd_ext_reset() {
        let mut m = StreamingMacdExt::new(3, MaType::Ema, 5, MaType::Ema, 3).unwrap();
        for i in 0..40 {
            m.next(i as f64);
        }
        assert!(m.is_ready());
        m.reset();
        assert!(!m.is_ready());
        assert_eq!(m.count(), 0);
    }

    #[test]
    fn test_streaming_macd_ext_repaint() {
        use crate::streaming::OhlcvBar;

        let data = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0];
        let mut repainting = StreamingMacdExt::new(3, MaType::Ema, 5, MaType::Ema, 3).unwrap();
        for (index, &value) in data.iter().enumerate() {
            repainting.compute_bar(&OhlcvBar::new_with_time(
                0.0,
                0.0,
                0.0,
                value,
                0.0,
                (index + 1) as i64 * 1000,
            ));
        }
        repainting.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 100.0, 0.0, 9000));
        repainting.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 200.0, 0.0, 9000));
        let result_repaint =
            repainting.compute_bar(&OhlcvBar::new_with_time(0.0, 0.0, 0.0, 18.0, 0.0, 9000));

        let mut clean = StreamingMacdExt::new(3, MaType::Ema, 5, MaType::Ema, 3).unwrap();
        for &value in &data {
            clean.next(value);
        }
        let result_clean = clean.next(18.0);

        let repaint = result_repaint.unwrap();
        let clean = result_clean.unwrap();
        assert!((repaint.macd - clean.macd).abs() < 1e-10);
        assert!((repaint.signal - clean.signal).abs() < 1e-10);
        assert!((repaint.histogram - clean.histogram).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        // Convergence check: streaming EMA-based MACDEXT should match the
        // batch implementation when both use Sma+Ema combinations.
        let data: Vec<f64> = (0..120)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 5.0)
            .collect();
        let fast = 12;
        let slow = 26;
        let sig = 9;
        let batch = crate::indicators::momentum::macdext(
            &data,
            fast,
            MaType::Ema,
            slow,
            MaType::Ema,
            sig,
            MaType::Ema,
        )
        .unwrap();
        let mut streaming =
            StreamingMacdExt::new(fast, MaType::Ema, slow, MaType::Ema, sig).unwrap();
        for (i, &val) in data.iter().enumerate() {
            if let Some(s) = streaming.next(val) {
                if !batch.macd[i].is_nan() {
                    assert!(
                        (s.macd - batch.macd[i]).abs() < 1e-9,
                        "MACDEXT macd mismatch at {i}: streaming={}, batch={}",
                        s.macd,
                        batch.macd[i]
                    );
                }
            }
        }
    }
}
