//! Streaming MACD with controllable MA type (MACDEXT).
//!
//! This is the streaming counterpart to the batch
//! `indicators::momentum::macdext` function. The full TA-Lib MACDEXT allows
//! each MA (fast, slow, signal) to be any of the supported `MaType` values.
//!
//! Streaming support covers the scalar MA variants available in the streaming
//! module: SMA, EMA, WMA, DEMA, TEMA, KAMA, T3, TRIMA, HMA, ALMA, and VIDYA.
//! MAMA is intentionally excluded because it produces a pair of lines, while
//! FRAMA has no streaming implementation yet. The signal line is always an
//! EMA for now, matching the typical TA-Lib usage.

use crate::impl_standard_methods;
use crate::indicators::overlap::MaType;
use crate::streaming::momentum::macd::MacdOutput;
use crate::streaming::overlap::alma::StreamingAlma;
use crate::streaming::overlap::dema::StreamingDema;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::overlap::hma::StreamingHma;
use crate::streaming::overlap::kama::StreamingKama;
use crate::streaming::overlap::sma::StreamingSma;
use crate::streaming::overlap::t3::StreamingT3;
use crate::streaming::overlap::tema::StreamingTema;
use crate::streaming::overlap::trima::StreamingTrima;
use crate::streaming::overlap::vidya::StreamingVidya;
use crate::streaming::overlap::wma::StreamingWma;
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
    signal_state: MaState,
    count: usize,
    last_value: Option<MacdOutput>,
    snapshot: Option<SnapshotState>,
    last_open_time: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MaKind {
    Sma,
    Ema,
    Wma,
    Dema,
    Tema,
    Kama,
    T3,
    Trima,
    Hma,
    Alma,
    Vidya,
}

#[derive(Clone)]
enum MaState {
    Sma(StreamingSma),
    Ema(StreamingEma),
    Wma(StreamingWma),
    Dema(StreamingDema),
    Tema(StreamingTema),
    Kama(StreamingKama),
    T3(StreamingT3),
    Trima(StreamingTrima),
    Hma(StreamingHma),
    Alma(StreamingAlma),
    Vidya(StreamingVidya),
}

#[derive(Clone)]
struct SnapshotState {
    fast_state: MaState,
    slow_state: MaState,
    signal_state: MaState,
    count: usize,
    last_value: Option<MacdOutput>,
    last_open_time: i64,
}

impl MaState {
    fn new(kind: MaKind, period: usize) -> Self {
        match kind {
            MaKind::Sma => MaState::Sma(StreamingSma::new(period)),
            MaKind::Ema => MaState::Ema(StreamingEma::new(period)),
            MaKind::Wma => MaState::Wma(StreamingWma::new(period)),
            MaKind::Dema => MaState::Dema(StreamingDema::new(period)),
            MaKind::Tema => MaState::Tema(StreamingTema::new(period)),
            MaKind::Kama => MaState::Kama(StreamingKama::new(period)),
            MaKind::T3 => MaState::T3(StreamingT3::new(period)),
            MaKind::Trima => MaState::Trima(StreamingTrima::new(period)),
            MaKind::Hma => MaState::Hma(StreamingHma::new(period)),
            MaKind::Alma => MaState::Alma(StreamingAlma::new(period, 6.0, 0.85)),
            MaKind::Vidya => MaState::Vidya(StreamingVidya::new(period.max(9), 9)),
        }
    }

    fn next(&mut self, value: f64) -> Option<f64> {
        match self {
            MaState::Sma(s) => s.next(value),
            MaState::Ema(e) => e.next(value),
            MaState::Wma(w) => w.next(value),
            MaState::Dema(d) => d.next(value),
            MaState::Tema(t) => t.next(value),
            MaState::Kama(k) => k.next(value),
            MaState::T3(t) => t.next(value),
            MaState::Trima(t) => t.next(value),
            MaState::Hma(h) => h.next(value),
            MaState::Alma(a) => a.next(value),
            MaState::Vidya(v) => v.next(value),
        }
    }

    fn reset(&mut self) {
        match self {
            MaState::Sma(s) => s.reset(),
            MaState::Ema(e) => e.reset(),
            MaState::Wma(w) => w.reset(),
            MaState::Dema(d) => d.reset(),
            MaState::Tema(t) => t.reset(),
            MaState::Kama(k) => k.reset(),
            MaState::T3(t) => t.reset(),
            MaState::Trima(t) => t.reset(),
            MaState::Hma(h) => h.reset(),
            MaState::Alma(a) => a.reset(),
            MaState::Vidya(v) => v.reset(),
        }
    }

    fn is_ready(&self) -> bool {
        match self {
            MaState::Sma(s) => s.is_ready(),
            MaState::Ema(e) => e.is_ready(),
            MaState::Wma(w) => w.is_ready(),
            MaState::Dema(d) => d.is_ready(),
            MaState::Tema(t) => t.is_ready(),
            MaState::Kama(k) => k.is_ready(),
            MaState::T3(t) => t.is_ready(),
            MaState::Trima(t) => t.is_ready(),
            MaState::Hma(h) => h.is_ready(),
            MaState::Alma(a) => a.is_ready(),
            MaState::Vidya(v) => v.is_ready(),
        }
    }
}

fn kind_from(ma_type: MaType) -> Result<MaKind, UnsupportedMaType> {
    match ma_type {
        MaType::Sma => Ok(MaKind::Sma),
        MaType::Ema => Ok(MaKind::Ema),
        MaType::Wma => Ok(MaKind::Wma),
        MaType::Dema => Ok(MaKind::Dema),
        MaType::Tema => Ok(MaKind::Tema),
        MaType::Kama => Ok(MaKind::Kama),
        MaType::T3 => Ok(MaKind::T3),
        MaType::Trima => Ok(MaKind::Trima),
        MaType::Hma => Ok(MaKind::Hma),
        MaType::Alma => Ok(MaKind::Alma),
        MaType::Vidya => Ok(MaKind::Vidya),
        other => Err(UnsupportedMaType(other)),
    }
}

impl StreamingMacdExt {
    /// Construct a new streaming MACDEXT with an EMA signal line.
    ///
    /// Returns `Err(UnsupportedMaType)` if the supplied MA type is not
    /// supported in the scalar streaming implementation.
    pub fn new(
        fast_period: usize,
        fast_ma_type: MaType,
        slow_period: usize,
        slow_ma_type: MaType,
        signal_period: usize,
    ) -> Result<Self, UnsupportedMaType> {
        Self::new_with_signal_ma(
            fast_period,
            fast_ma_type,
            slow_period,
            slow_ma_type,
            signal_period,
            MaType::Ema,
        )
    }

    /// Construct a streaming MACDEXT with an explicit scalar signal MA.
    ///
    /// [`Self::new`] remains the compatibility-friendly EMA-signal shortcut.
    pub fn new_with_signal_ma(
        fast_period: usize,
        fast_ma_type: MaType,
        slow_period: usize,
        slow_ma_type: MaType,
        signal_period: usize,
        signal_ma_type: MaType,
    ) -> Result<Self, UnsupportedMaType> {
        let fast_kind = kind_from(fast_ma_type)?;
        let slow_kind = kind_from(slow_ma_type)?;
        let signal_kind = kind_from(signal_ma_type)?;
        Ok(Self {
            fast_kind,
            slow_kind,
            fast_state: MaState::new(fast_kind, fast_period),
            slow_state: MaState::new(slow_kind, slow_period),
            signal_state: MaState::new(signal_kind, signal_period),
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
                self.signal_state = snapshot.signal_state;
                self.count = snapshot.count;
                self.last_value = snapshot.last_value;
                self.last_open_time = snapshot.last_open_time;
            }
        }

        self.snapshot = Some(SnapshotState {
            fast_state: self.fast_state.clone(),
            slow_state: self.slow_state.clone(),
            signal_state: self.signal_state.clone(),
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
        let Some(signal) = self.signal_state.next(macd) else {
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
        self.signal_state.reset();
        self.count = 0;
        self.last_value = None;
        self.snapshot = None;
        self.last_open_time = 0;
    }

    fn is_ready(&self) -> bool {
        self.fast_state.is_ready() && self.slow_state.is_ready() && self.signal_state.is_ready()
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
        "MACD with configurable scalar MA types for fast, slow, and signal lines"
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
        // MAMA returns two lines and FRAMA has no streaming implementation.
        let res = StreamingMacdExt::new(12, MaType::Mama, 26, MaType::Ema, 9);
        assert!(res.is_err());
        let res = StreamingMacdExt::new(12, MaType::Ema, 26, MaType::Frama, 9);
        assert!(res.is_err());
    }

    #[test]
    fn test_streaming_macd_ext_common_ma_variants() {
        let variants = [
            MaType::Wma,
            MaType::Dema,
            MaType::Tema,
            MaType::Kama,
            MaType::T3,
            MaType::Trima,
            MaType::Hma,
            MaType::Alma,
            MaType::Vidya,
        ];
        for variant in variants {
            let mut macd = StreamingMacdExt::new(10, variant, 20, MaType::Ema, 5).unwrap();
            for index in 0..100 {
                macd.next(100.0 + (index as f64 * 0.17).sin());
            }
            assert!(
                macd.is_ready(),
                "variant {:?} did not become ready",
                variant
            );
        }
    }

    #[test]
    fn test_streaming_macd_ext_explicit_signal_variant() {
        let mut macd =
            StreamingMacdExt::new_with_signal_ma(10, MaType::Wma, 20, MaType::Ema, 5, MaType::Dema)
                .unwrap();
        for index in 0..120 {
            macd.next(100.0 + (index as f64 * 0.13).cos());
        }
        assert!(macd.is_ready());
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
