use pyo3::prelude::*;
use pyo3::types::PyBytes;
use alpha_ta_core::streaming::indicators::*;
use alpha_ta_core::streaming::{CheckpointState, OhlcvBar, StreamingIndicator};

type IchimokuTuple = (f64, f64, f64, f64, f64);

// ============================================================================
// Category 1: f64 → f64 (single-period constructor)
// ============================================================================

macro_rules! py_streaming_f64_f64 {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            fn new(period: usize) -> Self {
                Self {
                    inner: <$rust_type>::new(period),
                }
            }

            fn update(&mut self, value: f64) -> f64 {
                self.inner.next(value).unwrap_or(f64::NAN)
            }

            fn update_batch(&mut self, values: Vec<f64>) -> Vec<f64> {
                values
                    .into_iter()
                    .map(|v| self.inner.next(v).unwrap_or(f64::NAN))
                    .collect()
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }

            fn save_state<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
                let bytes = self.inner.save_state().map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
                })?;
                Ok(PyBytes::new(py, &bytes))
            }

            #[staticmethod]
            fn restore_state(bytes: &[u8]) -> PyResult<Self> {
                let inner = <$rust_type>::restore_state(bytes).map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
                })?;
                Ok(Self { inner })
            }
        }
    };
}

py_streaming_f64_f64!(
    StreamingSMA,
    StreamingSma,
    "Streaming Simple Moving Average"
);
py_streaming_f64_f64!(
    StreamingEMA,
    StreamingEma,
    "Streaming Exponential Moving Average"
);
py_streaming_f64_f64!(
    StreamingWMA,
    StreamingWma,
    "Streaming Weighted Moving Average"
);
py_streaming_f64_f64!(
    StreamingDEMA,
    StreamingDema,
    "Streaming Double Exponential Moving Average"
);
py_streaming_f64_f64!(
    StreamingTEMA,
    StreamingTema,
    "Streaming Triple Exponential Moving Average"
);
py_streaming_f64_f64!(
    StreamingKAMA,
    StreamingKama,
    "Streaming Kaufman Adaptive Moving Average"
);
// StreamingT3 handled separately due to name collision with the PyClass name
#[pyclass(name = "StreamingT3")]
pub struct PyStreamingT3 {
    inner: alpha_ta_core::streaming::indicators::StreamingT3,
}

#[pymethods]
impl PyStreamingT3 {
    #[new]
    fn new(period: usize) -> Self {
        Self {
            inner: alpha_ta_core::streaming::indicators::StreamingT3::new(period),
        }
    }

    fn update(&mut self, value: f64) -> f64 {
        self.inner.next(value).unwrap_or(f64::NAN)
    }

    fn update_batch(&mut self, values: Vec<f64>) -> Vec<f64> {
        values
            .into_iter()
            .map(|v| self.inner.next(v).unwrap_or(f64::NAN))
            .collect()
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    fn count(&self) -> usize {
        self.inner.count()
    }
}
py_streaming_f64_f64!(
    StreamingRSI,
    StreamingRsi,
    "Streaming Relative Strength Index"
);
py_streaming_f64_f64!(StreamingMOM, StreamingMom, "Streaming Momentum");
py_streaming_f64_f64!(StreamingROC, StreamingRoc, "Streaming Rate of Change");
py_streaming_f64_f64!(StreamingHMA, StreamingHma, "Streaming Hull Moving Average");
py_streaming_f64_f64!(StreamingZLEMA, StreamingZlema, "Streaming Zero Lag EMA");
py_streaming_f64_f64!(
    StreamingMcGINLEY,
    StreamingMcGinley,
    "Streaming McGinley Dynamic"
);
py_streaming_f64_f64!(StreamingBIAS, StreamingBias, "Streaming Bias Rate");
py_streaming_f64_f64!(StreamingTRIX, StreamingTrix, "Streaming TRIX");
py_streaming_f64_f64!(
    StreamingDPO,
    StreamingDpo,
    "Streaming Detrended Price Oscillator"
);
py_streaming_f64_f64!(
    StreamingSTDDEV,
    StreamingStdDev,
    "Streaming Standard Deviation"
);
py_streaming_f64_f64!(StreamingVAR, StreamingVar, "Streaming Variance");
py_streaming_f64_f64!(StreamingZSCORE, StreamingZscore, "Streaming Z-Score");
py_streaming_f64_f64!(
    StreamingLINREG,
    StreamingLinReg,
    "Streaming Linear Regression"
);
py_streaming_f64_f64!(
    StreamingLinregSlope,
    StreamingLinRegSlope,
    "Streaming LinReg Slope"
);
py_streaming_f64_f64!(
    StreamingLinregIntercept,
    StreamingLinRegIntercept,
    "Streaming LinReg Intercept"
);
py_streaming_f64_f64!(
    StreamingLinregAngle,
    StreamingLinRegAngle,
    "Streaming LinReg Angle"
);
py_streaming_f64_f64!(StreamingTSF, StreamingTsf, "Streaming Time Series Forecast");
py_streaming_f64_f64!(
    StreamingCMO,
    StreamingCmo,
    "Streaming Chande Momentum Oscillator"
);
py_streaming_f64_f64!(
    StreamingULCER,
    StreamingUlcerIndex,
    "Streaming Ulcer Index"
);
py_streaming_f64_f64!(
    StreamingPSY,
    StreamingPsy,
    "Streaming Psychological Line"
);

macro_rules! py_streaming_f64_f64_default {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            fn new() -> Self {
                Self {
                    inner: <$rust_type>::new(),
                }
            }

            fn update(&mut self, value: f64) -> f64 {
                self.inner.next(value).unwrap_or(f64::NAN)
            }

            fn update_batch(&mut self, values: Vec<f64>) -> Vec<f64> {
                values
                    .into_iter()
                    .map(|v| self.inner.next(v).unwrap_or(f64::NAN))
                    .collect()
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_f64_f64_default!(
    StreamingHtDcperiod,
    StreamingHtDcPeriod,
    "Streaming Hilbert Transform Dominant Cycle Period"
);
py_streaming_f64_f64_default!(
    StreamingHtDcphase,
    StreamingHtDcPhase,
    "Streaming Hilbert Transform Dominant Cycle Phase"
);
py_streaming_f64_f64_default!(
    PyStreamingHtTrendline,
    StreamingHtTrendline,
    "Streaming Hilbert Transform Instantaneous Trendline"
);
py_streaming_f64_f64_default!(
    StreamingHtTrendmode,
    StreamingHtTrendMode,
    "Streaming Hilbert Transform Trend Mode"
);
py_streaming_f64_f64_default!(
    PyStreamingHtSine,
    StreamingHtSine,
    "Streaming Hilbert Transform Sine Wave"
);

macro_rules! py_streaming_f64_f64_2period {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            #[pyo3(signature = (fast_period=12, slow_period=26))]
            fn new(fast_period: usize, slow_period: usize) -> Self {
                Self {
                    inner: <$rust_type>::new(fast_period, slow_period),
                }
            }

            fn update(&mut self, value: f64) -> f64 {
                self.inner.next(value).unwrap_or(f64::NAN)
            }

            fn update_batch(&mut self, values: Vec<f64>) -> Vec<f64> {
                values
                    .into_iter()
                    .map(|v| self.inner.next(v).unwrap_or(f64::NAN))
                    .collect()
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_f64_f64_2period!(StreamingAPO, StreamingApo, "Streaming Absolute Price Oscillator");
py_streaming_f64_f64_2period!(
    StreamingPPO,
    StreamingPpo,
    "Streaming Percentage Price Oscillator"
);
py_streaming_f64_f64_2period!(
    StreamingTSI,
    StreamingTsi,
    "Streaming True Strength Index"
);

macro_rules! py_streaming_f64_f64_3period {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            #[pyo3(signature = (wma_period=10, long_roc=14, short_roc=11))]
            fn new(wma_period: usize, long_roc: usize, short_roc: usize) -> Self {
                Self {
                    inner: <$rust_type>::new(wma_period, long_roc, short_roc),
                }
            }

            fn update(&mut self, value: f64) -> f64 {
                self.inner.next(value).unwrap_or(f64::NAN)
            }

            fn update_batch(&mut self, values: Vec<f64>) -> Vec<f64> {
                values
                    .into_iter()
                    .map(|v| self.inner.next(v).unwrap_or(f64::NAN))
                    .collect()
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_f64_f64_3period!(
    StreamingCOPPOCK,
    StreamingCoppock,
    "Streaming Coppock Curve"
);

macro_rules! py_streaming_f64_f64_stc {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            #[pyo3(signature = (fast_period=23, slow_period=50, cycle=10))]
            fn new(fast_period: usize, slow_period: usize, cycle: usize) -> Self {
                Self {
                    inner: <$rust_type>::new(fast_period, slow_period, cycle),
                }
            }

            fn update(&mut self, value: f64) -> f64 {
                self.inner.next(value).unwrap_or(f64::NAN)
            }

            fn update_batch(&mut self, values: Vec<f64>) -> Vec<f64> {
                values
                    .into_iter()
                    .map(|v| self.inner.next(v).unwrap_or(f64::NAN))
                    .collect()
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_f64_f64_stc!(
    StreamingSTC,
    StreamingStc,
    "Streaming Schaff Trend Cycle"
);

macro_rules! py_streaming_f64_f64_alma {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            #[pyo3(signature = (period=9, sigma=6.0, offset=0.85))]
            fn new(period: usize, sigma: f64, offset: f64) -> Self {
                Self {
                    inner: <$rust_type>::new(period, sigma, offset),
                }
            }

            fn update(&mut self, value: f64) -> f64 {
                self.inner.next(value).unwrap_or(f64::NAN)
            }

            fn update_batch(&mut self, values: Vec<f64>) -> Vec<f64> {
                values
                    .into_iter()
                    .map(|v| self.inner.next(v).unwrap_or(f64::NAN))
                    .collect()
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_f64_f64_alma!(
    StreamingALMA,
    StreamingAlma,
    "Streaming Arnaud Legoux Moving Average"
);

macro_rules! py_streaming_f64_f64_vidya {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            #[pyo3(signature = (period=14, cmo_period=9))]
            fn new(period: usize, cmo_period: usize) -> Self {
                Self {
                    inner: <$rust_type>::new(period, cmo_period),
                }
            }

            fn update(&mut self, value: f64) -> f64 {
                self.inner.next(value).unwrap_or(f64::NAN)
            }

            fn update_batch(&mut self, values: Vec<f64>) -> Vec<f64> {
                values
                    .into_iter()
                    .map(|v| self.inner.next(v).unwrap_or(f64::NAN))
                    .collect()
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_f64_f64_vidya!(StreamingVIDYA, StreamingVidya, "Streaming VIDYA");

macro_rules! py_streaming_f64_pair_f64 {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            fn new(period: usize) -> Self {
                Self {
                    inner: <$rust_type>::new(period),
                }
            }

            fn update(&mut self, x: f64, y: f64) -> f64 {
                self.inner.next_pair(x, y).unwrap_or(f64::NAN)
            }

            fn update_batch(&mut self, x: Vec<f64>, y: Vec<f64>) -> PyResult<Vec<f64>> {
                if x.len() != y.len() {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "x and y must have the same length",
                    ));
                }
                Ok(x.iter()
                    .zip(y.iter())
                    .map(|(&a, &b)| self.inner.next_pair(a, b).unwrap_or(f64::NAN))
                    .collect())
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_f64_pair_f64!(StreamingBETA, StreamingBeta, "Streaming Beta");
py_streaming_f64_pair_f64!(StreamingCORREL, StreamingCorrel, "Streaming Correlation");

macro_rules! py_streaming_f64_f64_4period {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            #[pyo3(signature = (rsi_period=14, stoch_period=14, fastk_period=3, fastd_period=3))]
            fn new(
                rsi_period: usize,
                stoch_period: usize,
                fastk_period: usize,
                fastd_period: usize,
            ) -> Self {
                Self {
                    inner: <$rust_type>::new(rsi_period, stoch_period, fastk_period, fastd_period),
                }
            }

            fn update(&mut self, value: f64) -> f64 {
                self.inner.next(value).unwrap_or(f64::NAN)
            }

            fn update_batch(&mut self, values: Vec<f64>) -> Vec<f64> {
                values
                    .into_iter()
                    .map(|v| self.inner.next(v).unwrap_or(f64::NAN))
                    .collect()
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_f64_f64_4period!(
    PyStreamingStochRsi,
    StreamingStochRsi,
    "Streaming Stochastic RSI"
);

// ============================================================================
// Category 2: f64 → struct output
// ============================================================================

#[pyclass(name = "MACDResult")]
pub struct MACDResult {
    #[pyo3(get)]
    macd: f64,
    #[pyo3(get)]
    signal: f64,
    #[pyo3(get)]
    histogram: f64,
}

#[pyclass]
pub struct StreamingMACD {
    inner: StreamingMacd,
}

#[pymethods]
impl StreamingMACD {
    #[new]
    #[pyo3(signature = (fast_period=12, slow_period=26, signal_period=9))]
    fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self {
            inner: StreamingMacd::new(fast_period, slow_period, signal_period),
        }
    }

    fn update(&mut self, value: f64) -> MACDResult {
        match self.inner.next(value) {
            Some(out) => MACDResult {
                macd: out.macd,
                signal: out.signal,
                histogram: out.histogram,
            },
            None => MACDResult {
                macd: f64::NAN,
                signal: f64::NAN,
                histogram: f64::NAN,
            },
        }
    }

    fn update_batch(&mut self, values: Vec<f64>) -> Vec<MACDResult> {
        values
            .into_iter()
            .map(|v| match self.inner.next(v) {
                Some(out) => MACDResult {
                    macd: out.macd,
                    signal: out.signal,
                    histogram: out.histogram,
                },
                None => MACDResult {
                    macd: f64::NAN,
                    signal: f64::NAN,
                    histogram: f64::NAN,
                },
            })
            .collect()
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    fn count(&self) -> usize {
        self.inner.count()
    }
}

#[pyclass]
pub struct StreamingBOLL {
    inner: StreamingBoll,
}

#[pymethods]
impl StreamingBOLL {
    #[new]
    #[pyo3(signature = (period=20, nb_dev_up=2.0, nb_dev_dn=2.0))]
    fn new(period: usize, nb_dev_up: f64, nb_dev_dn: f64) -> Self {
        Self {
            inner: StreamingBoll::new(period, nb_dev_up, nb_dev_dn),
        }
    }

    fn update(&mut self, value: f64) -> (f64, f64, f64) {
        match self.inner.next(value) {
            Some(out) => (out.upper, out.middle, out.lower),
            None => (f64::NAN, f64::NAN, f64::NAN),
        }
    }

    fn update_batch(&mut self, values: Vec<f64>) -> Vec<(f64, f64, f64)> {
        values
            .into_iter()
            .map(|v| match self.inner.next(v) {
                Some(out) => (out.upper, out.middle, out.lower),
                None => (f64::NAN, f64::NAN, f64::NAN),
            })
            .collect()
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    fn count(&self) -> usize {
        self.inner.count()
    }
}

// ============================================================================
// Category 3: (high, low, close) tuple → f64
// ============================================================================

macro_rules! py_streaming_hlc_f64 {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            fn new(period: usize) -> Self {
                Self {
                    inner: <$rust_type>::new(period),
                }
            }

            fn update(&mut self, high: f64, low: f64, close: f64) -> f64 {
                self.inner.next((high, low, close)).unwrap_or(f64::NAN)
            }

            fn update_batch(
                &mut self,
                high: Vec<f64>,
                low: Vec<f64>,
                close: Vec<f64>,
            ) -> PyResult<Vec<f64>> {
                if high.len() != low.len() || high.len() != close.len() {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "high, low, close must have the same length",
                    ));
                }
                Ok(high
                    .iter()
                    .zip(low.iter())
                    .zip(close.iter())
                    .map(|((&h, &l), &c)| self.inner.next((h, l, c)).unwrap_or(f64::NAN))
                    .collect())
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_hlc_f64!(StreamingATR, StreamingAtr, "Streaming Average True Range");
py_streaming_hlc_f64!(
    StreamingADX,
    StreamingAdx,
    "Streaming Average Directional Index"
);
py_streaming_hlc_f64!(
    StreamingCCI,
    StreamingCci,
    "Streaming Commodity Channel Index"
);
py_streaming_hlc_f64!(StreamingADXR, StreamingAdxr, "Streaming ADXR");
py_streaming_hlc_f64!(StreamingDX, StreamingDx, "Streaming DX");
py_streaming_hlc_f64!(PyStreamingMinusDi, StreamingMinusDi, "Streaming Minus DI");
py_streaming_hlc_f64!(PyStreamingPlusDi, StreamingPlusDi, "Streaming Plus DI");
py_streaming_hlc_f64!(StreamingCHOP, StreamingChop, "Streaming Choppiness Index");

macro_rules! py_streaming_hl_f64 {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            fn new(period: usize) -> Self {
                Self {
                    inner: <$rust_type>::new(period),
                }
            }

            fn update(&mut self, high: f64, low: f64) -> f64 {
                self.inner.next((high, low)).unwrap_or(f64::NAN)
            }

            fn update_batch(&mut self, high: Vec<f64>, low: Vec<f64>) -> PyResult<Vec<f64>> {
                if high.len() != low.len() {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "high and low must have the same length",
                    ));
                }
                Ok(high
                    .iter()
                    .zip(low.iter())
                    .map(|(&h, &l)| self.inner.next((h, l)).unwrap_or(f64::NAN))
                    .collect())
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_hl_f64!(
    StreamingAROONOSC,
    StreamingAroonOsc,
    "Streaming Aroon Oscillator"
);

macro_rules! py_streaming_hlc_f64_3period {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            #[pyo3(signature = (period1=7, period2=14, period3=28))]
            fn new(period1: usize, period2: usize, period3: usize) -> Self {
                Self {
                    inner: <$rust_type>::new(period1, period2, period3),
                }
            }

            fn update(&mut self, high: f64, low: f64, close: f64) -> f64 {
                self.inner.next((high, low, close)).unwrap_or(f64::NAN)
            }

            fn update_batch(
                &mut self,
                high: Vec<f64>,
                low: Vec<f64>,
                close: Vec<f64>,
            ) -> PyResult<Vec<f64>> {
                if high.len() != low.len() || high.len() != close.len() {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "high, low, close must have the same length",
                    ));
                }
                Ok(high
                    .iter()
                    .zip(low.iter())
                    .zip(close.iter())
                    .map(|((&h, &l), &c)| self.inner.next((h, l, c)).unwrap_or(f64::NAN))
                    .collect())
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_hlc_f64_3period!(
    StreamingULTOSC,
    StreamingUltOsc,
    "Streaming Ultimate Oscillator"
);

macro_rules! py_streaming_hlc_stochf_2period {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            #[pyo3(signature = (fastk_period=5, fastd_period=3))]
            fn new(fastk_period: usize, fastd_period: usize) -> Self {
                Self {
                    inner: <$rust_type>::new(fastk_period, fastd_period),
                }
            }

            fn update(&mut self, high: f64, low: f64, close: f64) -> (f64, f64) {
                match self.inner.next((high, low, close)) {
                    Some(out) => (out.k, out.d),
                    None => (f64::NAN, f64::NAN),
                }
            }

            fn update_batch(
                &mut self,
                high: Vec<f64>,
                low: Vec<f64>,
                close: Vec<f64>,
            ) -> PyResult<Vec<(f64, f64)>> {
                if high.len() != low.len() || high.len() != close.len() {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "high, low, close must have the same length",
                    ));
                }
                Ok(high
                    .iter()
                    .zip(low.iter())
                    .zip(close.iter())
                    .map(|((&h, &l), &c)| match self.inner.next((h, l, c)) {
                        Some(out) => (out.k, out.d),
                        None => (f64::NAN, f64::NAN),
                    })
                    .collect())
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_hlc_stochf_2period!(
    StreamingSTOCHF,
    StreamingStochF,
    "Streaming Stochastic Fast"
);

// ============================================================================
// Category 4: (high, low, close) → struct output
// ============================================================================

#[pyclass]
pub struct StreamingSTOCH {
    inner: StreamingStoch,
}

#[pymethods]
impl StreamingSTOCH {
    #[new]
    #[pyo3(signature = (k_period=14, k_slow=3, d_period=3))]
    fn new(k_period: usize, k_slow: usize, d_period: usize) -> Self {
        Self {
            inner: StreamingStoch::new(k_period, k_slow, d_period),
        }
    }

    fn update(&mut self, high: f64, low: f64, close: f64) -> (f64, f64) {
        match self.inner.next((high, low, close)) {
            Some(out) => (out.k, out.d),
            None => (f64::NAN, f64::NAN),
        }
    }

    fn update_batch(
        &mut self,
        high: Vec<f64>,
        low: Vec<f64>,
        close: Vec<f64>,
    ) -> PyResult<Vec<(f64, f64)>> {
        if high.len() != low.len() || high.len() != close.len() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "high, low, close must have the same length",
            ));
        }
        Ok(high
            .iter()
            .zip(low.iter())
            .zip(close.iter())
            .map(|((&h, &l), &c)| match self.inner.next((h, l, c)) {
                Some(out) => (out.k, out.d),
                None => (f64::NAN, f64::NAN),
            })
            .collect())
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    fn count(&self) -> usize {
        self.inner.count()
    }
}

// ============================================================================
// Category 5: (high, low) → struct output
// ============================================================================

#[pyclass]
pub struct StreamingAROON {
    inner: StreamingAroon,
}

#[pymethods]
impl StreamingAROON {
    #[new]
    #[pyo3(signature = (period=14))]
    fn new(period: usize) -> Self {
        Self {
            inner: StreamingAroon::new(period),
        }
    }

    fn update(&mut self, high: f64, low: f64) -> (f64, f64) {
        match self.inner.next((high, low)) {
            Some(out) => (out.aroon_up, out.aroon_down),
            None => (f64::NAN, f64::NAN),
        }
    }

    fn update_batch(&mut self, high: Vec<f64>, low: Vec<f64>) -> PyResult<Vec<(f64, f64)>> {
        if high.len() != low.len() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "high and low must have the same length",
            ));
        }
        Ok(high
            .iter()
            .zip(low.iter())
            .map(|(&h, &l)| match self.inner.next((h, l)) {
                Some(out) => (out.aroon_up, out.aroon_down),
                None => (f64::NAN, f64::NAN),
            })
            .collect())
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    fn count(&self) -> usize {
        self.inner.count()
    }
}

// ============================================================================
// Category 6: OHLCV bar → f64 (uses &dyn Ohlcv interface)
// ============================================================================

#[pyclass]
pub struct StreamingOBV {
    inner: StreamingObv,
}

#[pymethods]
impl StreamingOBV {
    #[new]
    fn new() -> Self {
        Self {
            inner: StreamingObv::new(),
        }
    }

    fn update(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let bar = OhlcvBar::new(open, high, low, close, volume);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }

    fn update_batch(
        &mut self,
        open: Vec<f64>,
        high: Vec<f64>,
        low: Vec<f64>,
        close: Vec<f64>,
        volume: Vec<f64>,
    ) -> PyResult<Vec<f64>> {
        let n = open.len();
        if high.len() != n || low.len() != n || close.len() != n || volume.len() != n {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "all arrays must have the same length",
            ));
        }
        Ok((0..n)
            .map(|i| {
                let bar = OhlcvBar::new(open[i], high[i], low[i], close[i], volume[i]);
                self.inner.next(&bar).unwrap_or(f64::NAN)
            })
            .collect())
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    fn count(&self) -> usize {
        self.inner.count()
    }
}

#[pyclass]
pub struct StreamingVWAP {
    inner: StreamingVwap,
}

#[pymethods]
impl StreamingVWAP {
    #[new]
    fn new() -> Self {
        Self {
            inner: StreamingVwap::new(),
        }
    }

    fn update(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let bar = OhlcvBar::new(open, high, low, close, volume);
        self.inner.next(&bar).unwrap_or(f64::NAN)
    }

    fn update_batch(
        &mut self,
        open: Vec<f64>,
        high: Vec<f64>,
        low: Vec<f64>,
        close: Vec<f64>,
        volume: Vec<f64>,
    ) -> PyResult<Vec<f64>> {
        let n = open.len();
        if high.len() != n || low.len() != n || close.len() != n || volume.len() != n {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "all arrays must have the same length",
            ));
        }
        Ok((0..n)
            .map(|i| {
                let bar = OhlcvBar::new(open[i], high[i], low[i], close[i], volume[i]);
                self.inner.next(&bar).unwrap_or(f64::NAN)
            })
            .collect())
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    fn count(&self) -> usize {
        self.inner.count()
    }
}

macro_rules! py_streaming_ohlcv_f64 {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            fn new(period: usize) -> Self {
                Self {
                    inner: <$rust_type>::new(period),
                }
            }

            fn update(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
                let bar = OhlcvBar::new(open, high, low, close, volume);
                self.inner.next(&bar).unwrap_or(f64::NAN)
            }

            fn update_batch(
                &mut self,
                open: Vec<f64>,
                high: Vec<f64>,
                low: Vec<f64>,
                close: Vec<f64>,
                volume: Vec<f64>,
            ) -> PyResult<Vec<f64>> {
                let n = open.len();
                if high.len() != n || low.len() != n || close.len() != n || volume.len() != n {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "all arrays must have the same length",
                    ));
                }
                Ok((0..n)
                    .map(|i| {
                        let bar = OhlcvBar::new(open[i], high[i], low[i], close[i], volume[i]);
                        self.inner.next(&bar).unwrap_or(f64::NAN)
                    })
                    .collect())
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_ohlcv_f64!(StreamingWILLR, StreamingWillR, "Streaming Williams %R");
py_streaming_ohlcv_f64!(StreamingMFI, StreamingMfi, "Streaming Money Flow Index");
py_streaming_ohlcv_f64!(
    StreamingNATR,
    StreamingNatr,
    "Streaming Normalized Average True Range"
);
py_streaming_ohlcv_f64!(StreamingAR, StreamingAr, "Streaming AR");
py_streaming_ohlcv_f64!(StreamingBR, StreamingBr, "Streaming BR");
py_streaming_ohlcv_f64!(StreamingCR, StreamingCr, "Streaming CR");
py_streaming_ohlcv_f64!(StreamingVR, StreamingVr, "Streaming VR");
py_streaming_ohlcv_f64!(
    PyStreamingForceIndex,
    StreamingForceIndex,
    "Streaming Force Index"
);
py_streaming_ohlcv_f64!(StreamingEOM, StreamingEom, "Streaming Ease of Movement");
py_streaming_ohlcv_f64!(StreamingVWMA, StreamingVwma, "Streaming VWMA");

macro_rules! py_streaming_ohlcv_f64_default {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            fn new() -> Self {
                Self {
                    inner: <$rust_type>::new(),
                }
            }

            fn update(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
                let bar = OhlcvBar::new(open, high, low, close, volume);
                self.inner.next(&bar).unwrap_or(f64::NAN)
            }

            fn update_batch(
                &mut self,
                open: Vec<f64>,
                high: Vec<f64>,
                low: Vec<f64>,
                close: Vec<f64>,
                volume: Vec<f64>,
            ) -> PyResult<Vec<f64>> {
                let n = open.len();
                if high.len() != n || low.len() != n || close.len() != n || volume.len() != n {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "all arrays must have the same length",
                    ));
                }
                Ok((0..n)
                    .map(|i| {
                        let bar = OhlcvBar::new(open[i], high[i], low[i], close[i], volume[i]);
                        self.inner.next(&bar).unwrap_or(f64::NAN)
                    })
                    .collect())
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_ohlcv_f64_default!(StreamingAD, StreamingAd, "Streaming Accumulation/Distribution");
py_streaming_ohlcv_f64_default!(
    StreamingPVI,
    StreamingPvi,
    "Streaming Positive Volume Index"
);
py_streaming_ohlcv_f64_default!(
    StreamingNVI,
    StreamingNvi,
    "Streaming Negative Volume Index"
);
py_streaming_ohlcv_f64_default!(
    StreamingPVT,
    StreamingPvt,
    "Streaming Price Volume Trend"
);
py_streaming_ohlcv_f64_default!(
    PyStreamingAnchoredVwap,
    StreamingAnchoredVwap,
    "Streaming Anchored VWAP"
);
py_streaming_ohlcv_f64_default!(
    StreamingAVGPRICE,
    StreamingAvgPrice,
    "Streaming Average Price"
);
py_streaming_ohlcv_f64_default!(
    StreamingMEDPRICE,
    StreamingMedPrice,
    "Streaming Median Price"
);
py_streaming_ohlcv_f64_default!(
    StreamingTYPPRICE,
    StreamingTypPrice,
    "Streaming Typical Price"
);

macro_rules! py_streaming_ohlcv_f64_2period {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            #[pyo3(signature = (fast_period=5, slow_period=34))]
            fn new(fast_period: usize, slow_period: usize) -> Self {
                Self {
                    inner: <$rust_type>::new(fast_period, slow_period),
                }
            }

            fn update(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
                let bar = OhlcvBar::new(open, high, low, close, volume);
                self.inner.next(&bar).unwrap_or(f64::NAN)
            }

            fn update_batch(
                &mut self,
                open: Vec<f64>,
                high: Vec<f64>,
                low: Vec<f64>,
                close: Vec<f64>,
                volume: Vec<f64>,
            ) -> PyResult<Vec<f64>> {
                let n = open.len();
                if high.len() != n || low.len() != n || close.len() != n || volume.len() != n {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "all arrays must have the same length",
                    ));
                }
                Ok((0..n)
                    .map(|i| {
                        let bar = OhlcvBar::new(open[i], high[i], low[i], close[i], volume[i]);
                        self.inner.next(&bar).unwrap_or(f64::NAN)
                    })
                    .collect())
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_ohlcv_f64_2period!(StreamingAO, StreamingAo, "Streaming Awesome Oscillator");
py_streaming_ohlcv_f64_2period!(
    StreamingADOSC,
    StreamingAdosc,
    "Streaming Chaikin A/D Oscillator"
);

macro_rules! py_streaming_ohlcv_f64_period_ema {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            #[pyo3(signature = (period=25, ema_period=9))]
            fn new(period: usize, ema_period: usize) -> Self {
                Self {
                    inner: <$rust_type>::new(period, ema_period),
                }
            }

            fn update(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
                let bar = OhlcvBar::new(open, high, low, close, volume);
                self.inner.next(&bar).unwrap_or(f64::NAN)
            }

            fn update_batch(
                &mut self,
                open: Vec<f64>,
                high: Vec<f64>,
                low: Vec<f64>,
                close: Vec<f64>,
                volume: Vec<f64>,
            ) -> PyResult<Vec<f64>> {
                let n = open.len();
                if high.len() != n || low.len() != n || close.len() != n || volume.len() != n {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "all arrays must have the same length",
                    ));
                }
                Ok((0..n)
                    .map(|i| {
                        let bar = OhlcvBar::new(open[i], high[i], low[i], close[i], volume[i]);
                        self.inner.next(&bar).unwrap_or(f64::NAN)
                    })
                    .collect())
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_ohlcv_f64_period_ema!(
    PyStreamingMassIndex,
    StreamingMassIndex,
    "Streaming Mass Index"
);

macro_rules! py_streaming_ohlcv_f64_period_dev {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            #[pyo3(signature = (period=20, nb_dev=2.0))]
            fn new(period: usize, nb_dev: f64) -> Self {
                Self {
                    inner: <$rust_type>::new(period, nb_dev),
                }
            }

            fn update(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
                let bar = OhlcvBar::new(open, high, low, close, volume);
                self.inner.next(&bar).unwrap_or(f64::NAN)
            }

            fn update_batch(
                &mut self,
                open: Vec<f64>,
                high: Vec<f64>,
                low: Vec<f64>,
                close: Vec<f64>,
                volume: Vec<f64>,
            ) -> PyResult<Vec<f64>> {
                let n = open.len();
                if high.len() != n || low.len() != n || close.len() != n || volume.len() != n {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "all arrays must have the same length",
                    ));
                }
                Ok((0..n)
                    .map(|i| {
                        let bar = OhlcvBar::new(open[i], high[i], low[i], close[i], volume[i]);
                        self.inner.next(&bar).unwrap_or(f64::NAN)
                    })
                    .collect())
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_ohlcv_f64_period_dev!(
    PyStreamingVwapBands,
    StreamingVwapBands,
    "Streaming VWAP Bands"
);

macro_rules! py_streaming_hlcv_f64 {
    ($py_name:ident, $rust_type:ty, $doc:expr) => {
        #[doc = $doc]
        #[pyclass]
        pub struct $py_name {
            inner: $rust_type,
        }

        #[pymethods]
        impl $py_name {
            #[new]
            fn new(period: usize) -> Self {
                Self {
                    inner: <$rust_type>::new(period),
                }
            }

            fn update(
                &mut self,
                high: f64,
                low: f64,
                close: f64,
                volume: f64,
            ) -> f64 {
                self.inner
                    .next((high, low, close, volume))
                    .unwrap_or(f64::NAN)
            }

            fn update_batch(
                &mut self,
                high: Vec<f64>,
                low: Vec<f64>,
                close: Vec<f64>,
                volume: Vec<f64>,
            ) -> PyResult<Vec<f64>> {
                if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len()
                {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "high, low, close, volume must have the same length",
                    ));
                }
                Ok(high
                    .iter()
                    .zip(low.iter())
                    .zip(close.iter())
                    .zip(volume.iter())
                    .map(|(((&h, &l), &c), &v)| {
                        self.inner.next((h, l, c, v)).unwrap_or(f64::NAN)
                    })
                    .collect())
            }

            fn reset(&mut self) {
                self.inner.reset();
            }

            fn is_ready(&self) -> bool {
                self.inner.is_ready()
            }

            fn count(&self) -> usize {
                self.inner.count()
            }
        }
    };
}

py_streaming_hlcv_f64!(
    StreamingCMF,
    StreamingCmf,
    "Streaming Chaikin Money Flow"
);

#[pyclass]
pub struct StreamingTRANGE {
    inner: StreamingTrange,
}

#[pymethods]
impl StreamingTRANGE {
    #[new]
    fn new() -> Self {
        Self {
            inner: StreamingTrange::new(),
        }
    }

    fn update(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let _ = (open, volume);
        self.inner.next((high, low, close)).unwrap_or(f64::NAN)
    }

    fn update_batch(
        &mut self,
        open: Vec<f64>,
        high: Vec<f64>,
        low: Vec<f64>,
        close: Vec<f64>,
        volume: Vec<f64>,
    ) -> PyResult<Vec<f64>> {
        let n = open.len();
        if high.len() != n || low.len() != n || close.len() != n || volume.len() != n {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "all arrays must have the same length",
            ));
        }
        Ok((0..n)
            .map(|i| {
                self.inner
                    .next((high[i], low[i], close[i]))
                    .unwrap_or(f64::NAN)
            })
            .collect())
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    fn count(&self) -> usize {
        self.inner.count()
    }
}

// ============================================================================
// Category 7: OHLCV bar → struct output
// ============================================================================

#[pyclass]
pub struct StreamingDONCHIAN {
    inner: StreamingDonchian,
}

#[pymethods]
impl StreamingDONCHIAN {
    #[new]
    #[pyo3(signature = (period=20))]
    fn new(period: usize) -> Self {
        Self {
            inner: StreamingDonchian::new(period),
        }
    }

    fn update(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> (f64, f64, f64) {
        let bar = OhlcvBar::new(open, high, low, close, volume);
        match self.inner.next(&bar) {
            Some(out) => (out.upper, out.middle, out.lower),
            None => (f64::NAN, f64::NAN, f64::NAN),
        }
    }

    fn update_batch(
        &mut self,
        open: Vec<f64>,
        high: Vec<f64>,
        low: Vec<f64>,
        close: Vec<f64>,
        volume: Vec<f64>,
    ) -> PyResult<Vec<(f64, f64, f64)>> {
        let n = open.len();
        if high.len() != n || low.len() != n || close.len() != n || volume.len() != n {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "all arrays must have the same length",
            ));
        }
        Ok((0..n)
            .map(|i| {
                let bar = OhlcvBar::new(open[i], high[i], low[i], close[i], volume[i]);
                match self.inner.next(&bar) {
                    Some(out) => (out.upper, out.middle, out.lower),
                    None => (f64::NAN, f64::NAN, f64::NAN),
                }
            })
            .collect())
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    fn count(&self) -> usize {
        self.inner.count()
    }
}

#[pyclass]
pub struct StreamingICHIMOKU {
    inner: StreamingIchimoku,
}

#[pymethods]
impl StreamingICHIMOKU {
    #[new]
    #[pyo3(signature = (tenkan=9, kijun=26, senkou_b=52))]
    fn new(tenkan: usize, kijun: usize, senkou_b: usize) -> Self {
        Self {
            inner: StreamingIchimoku::new(tenkan, kijun, senkou_b),
        }
    }

    fn update(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> IchimokuTuple {
        let bar = OhlcvBar::new(open, high, low, close, volume);
        match self.inner.next(&bar) {
            Some(out) => (
                out.tenkan,
                out.kijun,
                out.senkou_a,
                out.senkou_b,
                out.chikou,
            ),
            None => (f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN),
        }
    }

    fn update_batch(
        &mut self,
        open: Vec<f64>,
        high: Vec<f64>,
        low: Vec<f64>,
        close: Vec<f64>,
        volume: Vec<f64>,
    ) -> PyResult<Vec<IchimokuTuple>> {
        let n = open.len();
        if high.len() != n || low.len() != n || close.len() != n || volume.len() != n {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "all arrays must have the same length",
            ));
        }
        Ok((0..n)
            .map(|i| {
                let bar = OhlcvBar::new(open[i], high[i], low[i], close[i], volume[i]);
                match self.inner.next(&bar) {
                    Some(out) => (
                        out.tenkan,
                        out.kijun,
                        out.senkou_a,
                        out.senkou_b,
                        out.chikou,
                    ),
                    None => (f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN),
                }
            })
            .collect())
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    fn count(&self) -> usize {
        self.inner.count()
    }
}

#[pyclass]
pub struct StreamingSUPERTREND {
    inner: StreamingSuperTrend,
}

#[pymethods]
impl StreamingSUPERTREND {
    #[new]
    #[pyo3(signature = (period=10, multiplier=3.0))]
    fn new(period: usize, multiplier: f64) -> Self {
        Self {
            inner: StreamingSuperTrend::new(period, multiplier),
        }
    }

    fn update(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> (f64, i32) {
        let bar = OhlcvBar::new(open, high, low, close, volume);
        match self.inner.next(&bar) {
            Some(out) => (out.supertrend, out.direction),
            None => (f64::NAN, 0),
        }
    }

    fn update_batch(
        &mut self,
        open: Vec<f64>,
        high: Vec<f64>,
        low: Vec<f64>,
        close: Vec<f64>,
        volume: Vec<f64>,
    ) -> PyResult<Vec<(f64, i32)>> {
        let n = open.len();
        if high.len() != n || low.len() != n || close.len() != n || volume.len() != n {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "all arrays must have the same length",
            ));
        }
        Ok((0..n)
            .map(|i| {
                let bar = OhlcvBar::new(open[i], high[i], low[i], close[i], volume[i]);
                match self.inner.next(&bar) {
                    Some(out) => (out.supertrend, out.direction),
                    None => (f64::NAN, 0),
                }
            })
            .collect())
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    fn count(&self) -> usize {
        self.inner.count()
    }
}

#[pyclass]
pub struct StreamingKELTNER {
    inner: StreamingKeltner,
}

#[pymethods]
impl StreamingKELTNER {
    #[new]
    #[pyo3(signature = (ema_period=20, atr_period=10, multiplier=2.0))]
    fn new(ema_period: usize, atr_period: usize, multiplier: f64) -> Self {
        Self {
            inner: StreamingKeltner::new(ema_period, atr_period, multiplier),
        }
    }

    fn update(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> (f64, f64, f64) {
        let bar = OhlcvBar::new(open, high, low, close, volume);
        match self.inner.next(&bar) {
            Some(out) => (out.upper, out.middle, out.lower),
            None => (f64::NAN, f64::NAN, f64::NAN),
        }
    }

    fn update_batch(
        &mut self,
        open: Vec<f64>,
        high: Vec<f64>,
        low: Vec<f64>,
        close: Vec<f64>,
        volume: Vec<f64>,
    ) -> PyResult<Vec<(f64, f64, f64)>> {
        let n = open.len();
        if high.len() != n || low.len() != n || close.len() != n || volume.len() != n {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "all arrays must have the same length",
            ));
        }
        Ok((0..n)
            .map(|i| {
                let bar = OhlcvBar::new(open[i], high[i], low[i], close[i], volume[i]);
                match self.inner.next(&bar) {
                    Some(out) => (out.upper, out.middle, out.lower),
                    None => (f64::NAN, f64::NAN, f64::NAN),
                }
            })
            .collect())
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    fn count(&self) -> usize {
        self.inner.count()
    }
}

pub fn register_streaming_classes(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<StreamingSMA>()?;
    m.add_class::<StreamingEMA>()?;
    m.add_class::<StreamingWMA>()?;
    m.add_class::<StreamingDEMA>()?;
    m.add_class::<StreamingTEMA>()?;
    m.add_class::<StreamingKAMA>()?;
    m.add_class::<PyStreamingT3>()?;
    m.add_class::<StreamingRSI>()?;
    m.add_class::<StreamingMOM>()?;
    m.add_class::<StreamingROC>()?;
    m.add_class::<StreamingHMA>()?;
    m.add_class::<StreamingZLEMA>()?;
    m.add_class::<StreamingMcGINLEY>()?;
    m.add_class::<StreamingBIAS>()?;
    m.add_class::<StreamingTRIX>()?;
    m.add_class::<StreamingDPO>()?;
    m.add_class::<StreamingSTDDEV>()?;
    m.add_class::<StreamingVAR>()?;
    m.add_class::<StreamingZSCORE>()?;
    m.add_class::<StreamingLINREG>()?;
    m.add_class::<StreamingLinregSlope>()?;
    m.add_class::<StreamingLinregIntercept>()?;
    m.add_class::<StreamingLinregAngle>()?;
    m.add_class::<StreamingTSF>()?;
    m.add_class::<StreamingCMO>()?;
    m.add_class::<StreamingULCER>()?;
    m.add_class::<StreamingPSY>()?;
    m.add_class::<StreamingHtDcperiod>()?;
    m.add_class::<StreamingHtDcphase>()?;
    m.add_class::<PyStreamingHtTrendline>()?;
    m.add_class::<StreamingHtTrendmode>()?;
    m.add_class::<PyStreamingHtSine>()?;
    m.add_class::<StreamingAPO>()?;
    m.add_class::<StreamingPPO>()?;
    m.add_class::<StreamingTSI>()?;
    m.add_class::<StreamingCOPPOCK>()?;
    m.add_class::<StreamingSTC>()?;
    m.add_class::<StreamingALMA>()?;
    m.add_class::<StreamingVIDYA>()?;
    m.add_class::<StreamingBETA>()?;
    m.add_class::<StreamingCORREL>()?;
    m.add_class::<PyStreamingStochRsi>()?;
    m.add_class::<MACDResult>()?;
    m.add_class::<StreamingMACD>()?;
    m.add_class::<StreamingBOLL>()?;
    m.add_class::<StreamingATR>()?;
    m.add_class::<StreamingADX>()?;
    m.add_class::<StreamingCCI>()?;
    m.add_class::<StreamingADXR>()?;
    m.add_class::<StreamingDX>()?;
    m.add_class::<PyStreamingMinusDi>()?;
    m.add_class::<PyStreamingPlusDi>()?;
    m.add_class::<StreamingCHOP>()?;
    m.add_class::<StreamingAROONOSC>()?;
    m.add_class::<StreamingULTOSC>()?;
    m.add_class::<StreamingSTOCHF>()?;
    m.add_class::<StreamingSTOCH>()?;
    m.add_class::<StreamingAROON>()?;
    m.add_class::<StreamingOBV>()?;
    m.add_class::<StreamingVWAP>()?;
    m.add_class::<StreamingWILLR>()?;
    m.add_class::<StreamingMFI>()?;
    m.add_class::<StreamingNATR>()?;
    m.add_class::<StreamingAR>()?;
    m.add_class::<StreamingBR>()?;
    m.add_class::<StreamingCR>()?;
    m.add_class::<StreamingVR>()?;
    m.add_class::<PyStreamingForceIndex>()?;
    m.add_class::<StreamingEOM>()?;
    m.add_class::<StreamingVWMA>()?;
    m.add_class::<StreamingAD>()?;
    m.add_class::<StreamingPVI>()?;
    m.add_class::<StreamingNVI>()?;
    m.add_class::<StreamingPVT>()?;
    m.add_class::<PyStreamingAnchoredVwap>()?;
    m.add_class::<StreamingAVGPRICE>()?;
    m.add_class::<StreamingMEDPRICE>()?;
    m.add_class::<StreamingTYPPRICE>()?;
    m.add_class::<StreamingAO>()?;
    m.add_class::<StreamingADOSC>()?;
    m.add_class::<PyStreamingMassIndex>()?;
    m.add_class::<PyStreamingVwapBands>()?;
    m.add_class::<StreamingCMF>()?;
    m.add_class::<StreamingTRANGE>()?;
    m.add_class::<StreamingDONCHIAN>()?;
    m.add_class::<StreamingICHIMOKU>()?;
    m.add_class::<StreamingSUPERTREND>()?;
    m.add_class::<StreamingKELTNER>()?;
    Ok(())
}
