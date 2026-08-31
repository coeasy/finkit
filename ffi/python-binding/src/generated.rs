// ─────────────────────────────────────────────────────────────────────
// GENERATED FILE — do not edit by hand.
// Source of truth: docs/indicator_registry.json (ffi.bodies.<lang>).
// Regenerate with: python3 scripts/sync_bindings.py --lang python --generate --rewrite
// ─────────────────────────────────────────────────────────────────────

/// Simple Moving Average (SMA)
///
/// Calculates the arithmetic mean of the last `timeperiod` data points.
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=14))]
fn sma(py: Python<'_>, close: PyReadonlyArray1<'_, f64>, timeperiod: usize) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        moving_avg::sma(close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Exponential Moving Average (EMA)
///
/// Applies more weight to recent prices using exponential smoothing.
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=14))]
fn ema(py: Python<'_>, close: PyReadonlyArray1<'_, f64>, timeperiod: usize) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        moving_avg::ema(close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Weighted Moving Average (WMA)
///
/// Applies linearly decreasing weights to older data points.
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=14))]
fn wma(py: Python<'_>, close: PyReadonlyArray1<'_, f64>, timeperiod: usize) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        moving_avg::wma(close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Double Exponential Moving Average (DEMA)
///
/// DEMA = 2 * EMA - EMA(EMA)
/// Reduces lag compared to traditional EMA.
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=14))]
fn dema(py: Python<'_>, close: PyReadonlyArray1<'_, f64>, timeperiod: usize) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        moving_avg::dema(close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Triple Exponential Moving Average (TEMA)
///
/// TEMA = 3 * EMA - 3 * EMA(EMA) + EMA(EMA(EMA))
/// Further reduces lag compared to DEMA.
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=14))]
fn tema(py: Python<'_>, close: PyReadonlyArray1<'_, f64>, timeperiod: usize) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        moving_avg::tema(close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Kaufman's Adaptive Moving Average (KAMA)
///
/// Adapts to market noise by adjusting the smoothing constant based on the
/// Efficiency Ratio (ER).
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Lookback period for ER calculation (default: 10)
/// * `fastperiod` - Fast EMA period (default: 2)
/// * `slowperiod` - Slow EMA period (default: 30)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=10, fastperiod=2, slowperiod=30))]
fn kama(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
    fastperiod: usize,
    slowperiod: usize,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        moving_avg::kama(close, timeperiod, fastperiod, slowperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// MESA Adaptive Moving Average (MAMA)
///
/// Uses Hilbert Transform to create an adaptive moving average that
/// adapts to price fluctuations without the phase lag of traditional MAs.
///
/// # Arguments
/// * `close` - Input data series
/// * `fastlimit` - Fast limit for alpha (default: 0.5)
/// * `slowlimit` - Slow limit for alpha (default: 0.05)
///
/// # Returns
/// Tuple of (MAMA, FAMA) arrays
#[pyfunction]
#[pyo3(signature = (close, fastlimit=0.5, slowlimit=0.05))]
fn mama(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    fastlimit: f64,
    slowlimit: f64,
) -> PyResult<(Vec<f64>, Vec<f64>)> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::mama(close, fastlimit, slowlimit)
            .map(|res| (res.mama.into_raw_vec(), res.fama.into_raw_vec()))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// T3 Moving Average (T3)
///
/// A moving average that uses exponential smoothing with a volume factor
/// to reduce lag and improve signal quality.
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Lookback period (default: 5)
/// * `vfactor` - Volume factor 0-1 (default: 0.7)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=5, vfactor=0.7))]
fn t3(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
    vfactor: f64,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::t3(close, timeperiod, vfactor)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Bollinger Bands (BBANDS)
///
/// Upper = SMA + (std_dev * nbdevup)
/// Middle = SMA
/// Lower = SMA - (std_dev * nbdevdn)
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Lookback period (default: 5)
/// * `nbdevup` - Number of standard deviations for upper band (default: 2.0)
/// * `nbdevdn` - Number of standard deviations for lower band (default: 2.0)
///
/// # Returns
/// Tuple of (upper, middle, lower) arrays
#[pyfunction]
#[pyo3(signature = (close, timeperiod=5, nbdevup=2.0, nbdevdn=2.0))]
fn bollinger_bands(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
    nbdevup: f64,
    nbdevdn: f64,
) -> PyResult<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::bbands(close, timeperiod, nbdevup, nbdevdn)
            .map(|res| {
                (
                    res.upper.into_raw_vec(),
                    res.middle.into_raw_vec(),
                    res.lower.into_raw_vec(),
                )
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Midpoint (MIDPOINT)
///
/// MIDPOINT = (highest_high + lowest_low) / 2
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=14))]
fn midpoint(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::midpoint(close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Midprice (MIDPRICE)
///
/// MIDPRICE = (highest_high + lowest_low) / 2
/// Calculated using high and low prices
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (high, low, timeperiod=14))]
fn midprice(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::midprice(high, low, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Parabolic SAR (SAR)
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `acceleration` - Acceleration factor step (default: 0.02)
/// * `maximum` - Maximum acceleration factor (default: 0.2)
///
/// # Returns
/// Tuple of (SAR, AF) arrays
#[pyfunction]
#[pyo3(signature = (high, low, acceleration=0.02, maximum=0.2))]
fn sar(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    acceleration: f64,
    maximum: f64,
) -> PyResult<(Vec<f64>, Vec<f64>)> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::sar(high, low, acceleration, maximum)
            .map(|res| (res.sar.into_raw_vec(), res.af.into_raw_vec()))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Relative Strength Index (RSI)
///
/// Measures the magnitude of recent price changes to evaluate overbought/oversold conditions.
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=14))]
fn rsi(py: Python<'_>, close: PyReadonlyArray1<'_, f64>, timeperiod: usize) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::rsi(close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Moving Average Convergence Divergence (MACD)
///
/// Shows the relationship between two moving averages of a security's price.
///
/// # Arguments
/// * `close` - Input data series
/// * `fastperiod` - Fast EMA period (default: 12)
/// * `slowperiod` - Slow EMA period (default: 26)
/// * `signalperiod` - Signal line EMA period (default: 9)
///
/// # Returns
/// Tuple of (MACD, Signal, Histogram) arrays
#[pyfunction]
#[pyo3(signature = (close, fastperiod=12, slowperiod=26, signalperiod=9))]
fn macd(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    fastperiod: usize,
    slowperiod: usize,
    signalperiod: usize,
) -> PyResult<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::macd(close, fastperiod, slowperiod, signalperiod)
            .map(|res| {
                (
                    res.macd.into_raw_vec(),
                    res.signal.into_raw_vec(),
                    res.hist.into_raw_vec(),
                )
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Stochastic Oscillator (STOCH)
///
/// Compares a security's closing price to its price range over a given period.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `fastk_period` - %K lookback period (default: 5)
/// * `slowk_period` - %K slowing period (default: 3)
/// * `slowd_period` - %D period (default: 3)
///
/// # Returns
/// Tuple of (%K, %D) arrays
#[pyfunction]
#[pyo3(signature = (high, low, close, fastk_period=5, slowk_period=3, slowd_period=3))]
fn stoch(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    fastk_period: usize,
    slowk_period: usize,
    slowd_period: usize,
) -> PyResult<(Vec<f64>, Vec<f64>)> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::stoch(high, low, close, fastk_period, slowk_period, slowd_period)
            .map(|res| (res.k.into_raw_vec(), res.d.into_raw_vec()))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Average Directional Index (ADX)
///
/// Measures trend strength regardless of trend direction.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (high, low, close, timeperiod=14))]
fn adx(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::adx(high, low, close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Aroon Indicator (AROON)
///
/// Identifies trend changes and the strength of the trend.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `timeperiod` - Lookback period (default: 14)
///
/// # Returns
/// Tuple of (Aroon Up, Aroon Down) arrays
#[pyfunction]
#[pyo3(signature = (high, low, timeperiod=14))]
fn aroon(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<(Vec<f64>, Vec<f64>)> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::aroon(high, low, timeperiod)
            .map(|res| (res.aroon_up.into_raw_vec(), res.aroon_down.into_raw_vec()))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Commodity Channel Index (CCI)
///
/// Measures the current price level relative to an average price level over a given period.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (high, low, close, timeperiod=14))]
fn cci(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::cci(high, low, close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Momentum (MOM)
///
/// Measures the change in price over a given period.
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Lookback period (default: 10)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=10))]
fn mom(py: Python<'_>, close: PyReadonlyArray1<'_, f64>, timeperiod: usize) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::mom(close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Rate of Change (ROC)
///
/// Measures the percentage change in price over a given period.
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Lookback period (default: 10)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=10))]
fn roc(py: Python<'_>, close: PyReadonlyArray1<'_, f64>, timeperiod: usize) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::roc(close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Williams %R (WILLR)
///
/// A momentum indicator that measures overbought/oversold levels.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (high, low, close, timeperiod=14))]
fn willr(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::willr(high, low, close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Absolute Price Oscillator (APO)
///
/// The difference between two moving averages.
///
/// # Arguments
/// * `close` - Input data series
/// * `fastperiod` - Fast period (default: 12)
/// * `slowperiod` - Slow period (default: 26)
#[pyfunction]
#[pyo3(signature = (close, fastperiod=12, slowperiod=26))]
fn apo(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    fastperiod: usize,
    slowperiod: usize,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::apo(close, fastperiod, slowperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Balance of Power (BOP)
///
/// Measures the strength of buyers vs sellers in the market.
///
/// # Arguments
/// * `open` - Open prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn bop(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<f64>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::bop(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Chande Momentum Oscillator (CMO)
///
/// A momentum indicator that measures the percentage of sum of up days vs sum of down days.
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=14))]
fn cmo(py: Python<'_>, close: PyReadonlyArray1<'_, f64>, timeperiod: usize) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::cmo(close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Money Flow Index (MFI)
///
/// A momentum indicator that uses both price and volume to identify overbought/oversold conditions.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (high, low, close, volume, timeperiod=14))]
fn mfi(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let volume = volume
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::mfi(high, low, close, volume, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Triple Exponential Average (TRIX)
///
/// A momentum oscillator that calculates a triple smoothed EMA.
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=14))]
fn trix(py: Python<'_>, close: PyReadonlyArray1<'_, f64>, timeperiod: usize) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::trix(close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (high, low, close, timeperiod=14))]
fn vortex(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<(Vec<f64>, Vec<f64>)> {
    let high = high.as_slice().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low.as_slice().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close.as_slice().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::vortex(high, low, close, timeperiod)
            .map(|r| (r.vi_plus.into_raw_vec(), r.vi_minus.into_raw_vec()))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (close, volume, timeperiod=14))]
fn vzo(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let close = close.as_slice().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let volume = volume.as_slice().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::vzo(close, volume, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (volume, timeperiod=14))]
fn volume_momentum(
    py: Python<'_>,
    volume: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let volume = volume.as_slice().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::volume_momentum(volume, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (volume, timeperiod=14))]
fn volume_roc(
    py: Python<'_>,
    volume: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let volume = volume.as_slice().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::volume_roc(volume, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (close, timeperiod=14))]
fn chande_forecast_oscillator(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let close = close.as_slice().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::chande_forecast_oscillator(close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (high, low, close, volume, timeperiod=14))]
fn twiggs_money_flow(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let high = high.as_slice().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low.as_slice().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close.as_slice().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let volume = volume.as_slice().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::twiggs_money_flow(high, low, close, volume, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (open, high, low, close, rvi_period=10, linreg_period=14))]
fn inertia(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    rvi_period: usize,
    linreg_period: usize,
) -> PyResult<Vec<f64>> {
    let open = open.as_slice().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high.as_slice().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low.as_slice().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close.as_slice().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::inertia(open, high, low, close, rvi_period, linreg_period)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Average True Range (ATR)
///
/// A volatility indicator that measures the average price range over a given period.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (high, low, close, timeperiod=14))]
fn atr(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::atr(high, low, close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Normalized Average True Range (NATR)
///
/// The ATR expressed as a percentage of the close price.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (high, low, close, timeperiod=14))]
fn natr(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::natr(high, low, close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// True Range (TRANGE)
///
/// The greatest of: High - Low, |High - Previous Close|, |Low - Previous Close|
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
#[pyfunction]
#[pyo3(signature = (high, low, close))]
fn trange(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::trange(high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// On Balance Volume (OBV)
///
/// A cumulative indicator that uses volume flow to predict price changes.
///
/// # Arguments
/// * `close` - Close prices
/// * `volume` - Volume data
#[pyfunction]
#[pyo3(signature = (close, volume))]
fn obv(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let volume = volume
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::obv(close, volume)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Accumulation/Distribution Line (AD)
///
/// A cumulative indicator that uses volume and price to assess whether an asset is being accumulated or distributed.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
#[pyfunction]
#[pyo3(signature = (high, low, close, volume))]
fn ad(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let volume = volume
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::ad(high, low, close, volume)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Chaikin A/D Oscillator (ADOSC)
///
/// Measures the momentum of the Accumulation/Distribution Line using two EMAs.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `fastperiod` - Fast EMA period (default: 3)
/// * `slowperiod` - Slow EMA period (default: 10)
#[pyfunction]
#[pyo3(signature = (high, low, close, volume, fastperiod=3, slowperiod=10))]
fn adosc(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    fastperiod: usize,
    slowperiod: usize,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let volume = volume
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::adosc(high, low, close, volume, fastperiod, slowperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Hilbert Transform - Dominant Cycle Period (HT_DCPERIOD)
///
/// Measures the dominant cycle period of the price series using the Hilbert Transform.
///
/// # Arguments
/// * `close` - Input data series (typically close prices)
#[pyfunction]
#[pyo3(signature = (close))]
fn ht_dcperiod(py: Python<'_>, close: PyReadonlyArray1<'_, f64>) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::ht_dcperiod(close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Hilbert Transform - Dominant Cycle Phase (HT_DCPHASE)
///
/// Measures the dominant cycle phase of the price series in degrees (0-360).
///
/// # Arguments
/// * `close` - Input data series
#[pyfunction]
#[pyo3(signature = (close))]
fn ht_dcphase(py: Python<'_>, close: PyReadonlyArray1<'_, f64>) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::ht_dcphase(close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Hilbert Transform - Phasor Components (HT_PHASOR)
///
/// Returns the in-phase and quadrature components of the Hilbert Transform.
///
/// # Arguments
/// * `close` - Input data series
///
/// # Returns
/// Tuple of (in_phase, quadrature) arrays
#[pyfunction]
#[pyo3(signature = (close))]
fn ht_phasor(py: Python<'_>, close: PyReadonlyArray1<'_, f64>) -> PyResult<(Vec<f64>, Vec<f64>)> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::ht_phasor(close)
            .map(|res| (res.0.into_raw_vec(), res.1.into_raw_vec()))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Hilbert Transform - Sine Wave (HT_SINE)
///
/// Returns the sine and lead sine wave components derived from the Hilbert Transform.
///
/// # Arguments
/// * `close` - Input data series
///
/// # Returns
/// Tuple of (sine, lead_sine) arrays
#[pyfunction]
#[pyo3(signature = (close))]
fn ht_sine(py: Python<'_>, close: PyReadonlyArray1<'_, f64>) -> PyResult<(Vec<f64>, Vec<f64>)> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::ht_sine(close)
            .map(|res| (res.0.into_raw_vec(), res.1.into_raw_vec()))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Hilbert Transform - Trend vs Cycle Mode (HT_TRENDMODE)
///
/// Indicates whether the market is in trend mode (1) or cycle mode (0).
///
/// # Arguments
/// * `close` - Input data series
#[pyfunction]
#[pyo3(signature = (close))]
fn ht_trendmode(py: Python<'_>, close: PyReadonlyArray1<'_, f64>) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::ht_trendmode(close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Hilbert Transform - Instantaneous Trendline (HT_TRENDLINE)
///
/// Computes the instantaneous trendline of the price series using the Hilbert Transform.
///
/// # Arguments
/// * `close` - Input data series (typically close prices)
#[pyfunction]
#[pyo3(signature = (close))]
fn ht_trendline(py: Python<'_>, close: PyReadonlyArray1<'_, f64>) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::ht_trendline(close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Z-Score (Z分数/标准化)
///
/// Calculates the standard score of each data point relative to a rolling window.
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Rolling window size (default: 14)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=14))]
fn zscore(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::zscore(close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Beta Coefficient (Beta系数)
///
/// Calculates the Beta coefficient between two assets, measuring relative volatility.
///
/// # Arguments
/// * `asset` - Asset price series (e.g., stock)
/// * `benchmark` - Benchmark price series (e.g., market index)
/// * `timeperiod` - Rolling window size (default: 5)
#[pyfunction]
#[pyo3(signature = (asset, benchmark, timeperiod=5))]
fn beta(
    py: Python<'_>,
    asset: PyReadonlyArray1<'_, f64>,
    benchmark: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let asset = asset
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let benchmark = benchmark
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::beta(asset, benchmark, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Correlation (Pearson Correlation Coefficient)
///
/// Calculates the rolling Pearson correlation coefficient between two price series.
///
/// # Arguments
/// * `input_a` - First data series
/// * `input_b` - Second data series
/// * `timeperiod` - Rolling window size (default: 14)
#[pyfunction]
#[pyo3(signature = (input_a, input_b, timeperiod=14))]
fn correlation(
    py: Python<'_>,
    input_a: PyReadonlyArray1<'_, f64>,
    input_b: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let input_a = input_a
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let input_b = input_b
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::correlation(input_a, input_b, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Standard Deviation (StdDev)
///
/// Calculates the rolling standard deviation (sample standard deviation).
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Rolling window size (default: 5)
/// * `nbdev` - Number of standard deviations multiplier (default: 1.0)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=5, nbdev=1.0))]
fn std_dev(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
    nbdev: f64,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::std_dev(close, timeperiod, nbdev)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Time Series Forecast (TSF)
///
/// Predicts the next time point value using linear regression extrapolation.
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Rolling window size (default: 14)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=14))]
fn tsf(py: Python<'_>, close: PyReadonlyArray1<'_, f64>, timeperiod: usize) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::tsf(close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Linear Regression (线性回归)
///
/// Calculates rolling linear regression fitted values using least squares.
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Rolling window size (default: 14)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=14))]
fn linear_reg(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::linear_reg(close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Percent Rank (百分比排名)
///
/// Calculates the percentile rank of the current value within the rolling window.
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Rolling window size (default: 10)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=10))]
fn percent_rank(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::percent_rank(close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Average Price (AVGPRICE)
///
/// (Open + High + Low + Close) / 4
///
/// # Arguments
/// * `open` - Open prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn avgprice(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<f64>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::avgprice(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Median Price (MEDPRICE)
///
/// (High + Low) / 2
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
#[pyfunction]
#[pyo3(signature = (high, low))]
fn medprice(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::medprice(high, low)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Typical Price (TYPPRICE)
///
/// (High + Low + Close) / 3
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
#[pyfunction]
#[pyo3(signature = (high, low, close))]
fn typprice(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::typprice(high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Weighted Close Price (WCLPRICE)
///
/// (High + Low + 2 * Close) / 4
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
#[pyfunction]
#[pyo3(signature = (high, low, close))]
fn wclprice(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::wclprice(high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Doji (十字星)
///
/// Open and close are virtually the same.
///
/// # Arguments
/// * `open` - Open prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `doji_pct` - Doji threshold percentage (default: 0.1)
#[pyfunction]
#[pyo3(signature = (open, high, low, close, doji_pct=0.1))]
fn cdl_doji(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    doji_pct: f64,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        candlestick::doji(open, high, low, close, doji_pct)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Dragonfly Doji (蜻蜓十字)
#[pyfunction]
#[pyo3(signature = (open, high, low, close, doji_pct=0.1))]
fn cdl_dragonfly_doji(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    doji_pct: f64,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        candlestick::dragonfly_doji(open, high, low, close, doji_pct)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Gravestone Doji (墓碑十字)
#[pyfunction]
#[pyo3(signature = (open, high, low, close, doji_pct=0.1))]
fn cdl_gravestone_doji(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    doji_pct: f64,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        candlestick::gravestone_doji(open, high, low, close, doji_pct)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Long-Legged Doji (长腿十字)
#[pyfunction]
#[pyo3(signature = (open, high, low, close, doji_pct=0.1))]
fn cdl_long_legged_doji(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    doji_pct: f64,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        candlestick::long_legged_doji(open, high, low, close, doji_pct)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Hammer (锤子线)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_hammer(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        candlestick::hammer(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Inverted Hammer (倒锤子线)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_inverted_hammer(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        candlestick::inverted_hammer(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Hanging Man (上吊线)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_hanging_man(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        candlestick::hanging_man(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Shooting Star (射击之星)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_shooting_star(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        candlestick::shooting_star(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Engulfing Pattern (吞没形态)
///
/// Returns: 100 for bullish engulfing, -100 for bearish engulfing
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_engulfing(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        candlestick::engulfing(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Harami Pattern (孕线形态)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_harami(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        candlestick::harami(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Morning Star (晨星)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_morning_star(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        candlestick::morning_star(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Evening Star (暮星)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_evening_star(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        candlestick::evening_star(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Three White Soldiers (三白兵)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_three_white_soldiers(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        candlestick::three_white_soldiers(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Three Black Crows (三乌鸦)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_three_black_crows(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        candlestick::three_black_crows(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Marubozu (光头光脚)
#[pyfunction]
#[pyo3(signature = (open, high, low, close, shadow_pct=0.05))]
fn cdl_marubozu(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    shadow_pct: f64,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        candlestick::marubozu(open, high, low, close, shadow_pct)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

