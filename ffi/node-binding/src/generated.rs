// ─────────────────────────────────────────────────────────────────────
// GENERATED FILE — do not edit by hand.
// Source of truth: docs/indicator_registry.json (ffi.bodies.<lang>).
// Regenerate with: python3 scripts/sync_bindings.py --lang node --generate --rewrite
// ─────────────────────────────────────────────────────────────────────

/// Simple Moving Average (SMA)
///
/// Calculates the arithmetic mean of prices over a specified period.
///
/// @param close - Close prices array
/// @param timeperiod - Number of periods (default: 5)
/// @returns Array of SMA values
#[napi]
pub fn sma(close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    moving_avg::sma(&close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Exponential Moving Average (EMA)
///
/// Calculates a moving average that gives more weight to recent prices.
///
/// @param close - Close prices array
/// @param timeperiod - Number of periods (default: 12)
/// @returns Array of EMA values
#[napi]
pub fn ema(close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    moving_avg::ema(&close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Weighted Moving Average (WMA)
///
/// Calculates a moving average that assigns more weight to recent data.
///
/// @param close - Close prices array
/// @param timeperiod - Number of periods
/// @returns Array of WMA values
#[napi]
pub fn wma(close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    moving_avg::wma(&close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Double Exponential Moving Average (DEMA)
///
/// Reduces lag by using a combination of two EMAs.
///
/// @param close - Close prices array
/// @param timeperiod - Number of periods
/// @returns Array of DEMA values
#[napi]
pub fn dema(close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    moving_avg::dema(&close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Triple Exponential Moving Average (TEMA)
///
/// Reduces lag even further than DEMA by using three EMAs.
///
/// @param close - Close prices array
/// @param timeperiod - Number of periods
/// @returns Array of TEMA values
#[napi]
pub fn tema(close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    moving_avg::tema(&close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Kaufman Adaptive Moving Average (KAMA)
///
/// An adaptive moving average that adjusts to market noise.
///
/// @param close - Close prices array
/// @param timeperiod - Number of periods
/// @returns Array of KAMA values
#[napi]
pub fn kama(close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    moving_avg::kama(&close, timeperiod as usize, 2, 30)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

#[napi]
pub fn mama(close: Vec<f64>, fastlimit: Option<f64>, slowlimit: Option<f64>) -> Result<MamaResult> {
    let fast = fastlimit.unwrap_or(0.5);
    let slow = slowlimit.unwrap_or(0.05);
    indicators::mama(&close, fast, slow)
        .map(|res| MamaResult {
            mama: res.mama.into_raw_vec(),
            fama: res.fama.into_raw_vec(),
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Triple Exponential Moving Average (T3)
///
/// A moving average with very little lag and good smoothing.
///
/// @param close - Close prices array
/// @param timeperiod - Number of periods
/// @param vfactor - Volume factor (default: 0.7)
/// @returns Array of T3 values
#[napi]
pub fn t3(close: Vec<f64>, timeperiod: u32, vfactor: Option<f64>) -> Result<Vec<f64>> {
    let v = vfactor.unwrap_or(0.7);
    indicators::t3(&close, timeperiod as usize, v)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Bollinger Bands (BBANDS)
///
/// Volatility bands placed above and below a moving average.
///
/// @param close - Close prices array
/// @param timeperiod - Number of periods (default: 5)
/// @param nbdevup - Upper band standard deviations (default: 2.0)
/// @param nbdevdn - Lower band standard deviations (default: 2.0)
/// @returns Object containing upper, middle, and lower arrays
#[napi]
pub fn bollinger_bands(
    close: Vec<f64>,
    timeperiod: u32,
    nbdevup: f64,
    nbdevdn: f64,
) -> Result<BbandsResult> {
    indicators::bbands(&close, timeperiod as usize, nbdevup, nbdevdn)
        .map(|res| BbandsResult {
            upper: res.upper.into_raw_vec(),
            middle: res.middle.into_raw_vec(),
            lower: res.lower.into_raw_vec(),
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Parabolic SAR (SAR)
///
/// A trend-following indicator that provides stop levels.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param acceleration - Acceleration factor (default: 0.02)
/// @param maximum - Maximum acceleration (default: 0.2)
/// @returns Array of SAR values
#[napi]
pub fn sar(high: Vec<f64>, low: Vec<f64>, acceleration: f64, maximum: f64) -> Result<Vec<f64>> {
    indicators::sar(&high, &low, acceleration, maximum)
        .map(|res| res.sar.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Relative Strength Index (RSI)
///
/// Measures the magnitude of recent price changes to evaluate overbought/oversold conditions.
///
/// @param close - Close prices array
/// @param timeperiod - Number of periods (default: 14)
/// @returns Array of RSI values (0-100)
#[napi]
pub fn rsi(close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::rsi(&close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Moving Average Convergence Divergence (MACD)
///
/// Shows the relationship between two EMAs of a security's price.
///
/// @param close - Close prices array
/// @param fastperiod - Fast EMA period (default: 12)
/// @param slowperiod - Slow EMA period (default: 26)
/// @param signalperiod - Signal line period (default: 9)
/// @returns Object containing macd, signal, and hist arrays
#[napi]
pub fn macd(
    close: Vec<f64>,
    fastperiod: u32,
    slowperiod: u32,
    signalperiod: u32,
) -> Result<MacdResult> {
    indicators::macd(
        &close,
        fastperiod as usize,
        slowperiod as usize,
        signalperiod as usize,
    )
    .map(|res| MacdResult {
        macd: res.macd.into_raw_vec(),
        signal: res.signal.into_raw_vec(),
        hist: res.hist.into_raw_vec(),
    })
    .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Stochastic Oscillator (STOCH)
///
/// Compares a security's closing price to its price range over a given period.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param fastk_period - %K period (default: 5)
/// @param slowk_period - %K slowing period (default: 3)
/// @param slowd_period - %D period (default: 3)
/// @returns Object containing k and d arrays
#[napi]
pub fn stoch(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    fastk_period: u32,
    slowk_period: u32,
    slowd_period: u32,
) -> Result<StochResult> {
    indicators::stoch(
        &high,
        &low,
        &close,
        fastk_period as usize,
        slowk_period as usize,
        slowd_period as usize,
    )
    .map(|res| StochResult {
        k: res.k.into_raw_vec(),
        d: res.d.into_raw_vec(),
    })
    .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Average Directional Index (ADX)
///
/// Measures trend strength regardless of trend direction.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param timeperiod - Number of periods (default: 14)
/// @returns Array of ADX values
#[napi]
pub fn adx(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::adx(&high, &low, &close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Aroon Indicator (AROON)
///
/// Identifies trend changes and the strength of the trend.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param timeperiod - Number of periods (default: 14)
/// @returns Object containing aroon_up and aroon_down arrays
#[napi]
pub fn aroon(high: Vec<f64>, low: Vec<f64>, timeperiod: u32) -> Result<AroonResult> {
    indicators::aroon(&high, &low, timeperiod as usize)
        .map(|res| AroonResult {
            aroon_up: res.aroon_up.into_raw_vec(),
            aroon_down: res.aroon_down.into_raw_vec(),
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Commodity Channel Index (CCI)
///
/// Measures the current price level relative to an average price level.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param timeperiod - Number of periods (default: 14)
/// @returns Array of CCI values
#[napi]
pub fn cci(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::cci(&high, &low, &close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Momentum (MOM)
///
/// Measures the change in price over a given period.
///
/// @param close - Close prices array
/// @param timeperiod - Number of periods
/// @returns Array of momentum values
#[napi]
pub fn mom(close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::mom(&close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Rate of Change (ROC)
///
/// Measures the percentage change in price over a given period.
///
/// @param close - Close prices array
/// @param timeperiod - Number of periods
/// @returns Array of ROC values (percentage)
#[napi]
pub fn roc(close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::roc(&close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Williams %R (WILLR)
///
/// A momentum indicator that measures overbought/oversold levels.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param timeperiod - Number of periods (default: 14)
/// @returns Array of Williams %R values (-100 to 0)
#[napi]
pub fn willr(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::willr(&high, &low, &close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Absolute Price Oscillator (APO)
///
/// The difference between two moving averages.
///
/// @param close - Close prices array
/// @param fastperiod - Fast period
/// @param slowperiod - Slow period
/// @returns Array of APO values
#[napi]
pub fn apo(close: Vec<f64>, fastperiod: u32, slowperiod: u32) -> Result<Vec<f64>> {
    indicators::apo(&close, fastperiod as usize, slowperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Balance of Power (BOP)
///
/// Measures the strength of buyers vs sellers in the market.
///
/// @param open - Open prices array
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @returns Array of BOP values
#[napi]
pub fn bop(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<f64>> {
    indicators::bop(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Chande Momentum Oscillator (CMO)
///
/// A momentum indicator that measures the percentage of sum of up days vs sum of down days.
///
/// @param close - Close prices array
/// @param timeperiod - Number of periods
/// @returns Array of CMO values (-100 to 100)
#[napi]
pub fn cmo(close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::cmo(&close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Money Flow Index (MFI)
///
/// A momentum indicator that uses both price and volume.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param volume - Volume array
/// @param timeperiod - Number of periods (default: 14)
/// @returns Array of MFI values (0-100)
#[napi]
pub fn mfi(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    timeperiod: u32,
) -> Result<Vec<f64>> {
    indicators::mfi(&high, &low, &close, &volume, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Triple Exponential Average (TRIX)
///
/// A momentum oscillator that calculates a triple smoothed EMA.
///
/// @param close - Close prices array
/// @param timeperiod - Number of periods
/// @returns Array of TRIX values (percentage)
#[napi]
pub fn trix(close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::trix(&close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

#[napi]
pub fn vortex(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, timeperiod: Option<u32>) -> Result<Vec<Vec<f64>>> {
    let period = timeperiod.unwrap_or(14) as usize;
    indicators::vortex(&high, &low, &close, period)
        .map(|r| vec![r.vi_plus.into_raw_vec(), r.vi_minus.into_raw_vec()])
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

#[napi]
pub fn vzo(close: Vec<f64>, volume: Vec<f64>, timeperiod: Option<u32>) -> Result<Vec<f64>> {
    let period = timeperiod.unwrap_or(14) as usize;
    indicators::vzo(&close, &volume, period)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

#[napi]
pub fn volume_momentum(volume: Vec<f64>, timeperiod: Option<u32>) -> Result<Vec<f64>> {
    let period = timeperiod.unwrap_or(14) as usize;
    indicators::volume_momentum(&volume, period)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

#[napi]
pub fn volume_roc(volume: Vec<f64>, timeperiod: Option<u32>) -> Result<Vec<f64>> {
    let period = timeperiod.unwrap_or(14) as usize;
    indicators::volume_roc(&volume, period)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

#[napi]
pub fn chande_forecast_oscillator(close: Vec<f64>, timeperiod: Option<u32>) -> Result<Vec<f64>> {
    let period = timeperiod.unwrap_or(14) as usize;
    indicators::chande_forecast_oscillator(&close, period)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

#[napi]
pub fn twiggs_money_flow(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, volume: Vec<f64>, timeperiod: Option<u32>) -> Result<Vec<f64>> {
    let period = timeperiod.unwrap_or(14) as usize;
    indicators::twiggs_money_flow(&high, &low, &close, &volume, period)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

#[napi]
pub fn inertia_indicator(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, rvi_period: Option<u32>, linreg_period: Option<u32>) -> Result<Vec<f64>> {
    let rp = rvi_period.unwrap_or(10) as usize;
    let lp = linreg_period.unwrap_or(14) as usize;
    indicators::inertia(&open, &high, &low, &close, rp, lp)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Average True Range (ATR)
///
/// Measures market volatility.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param timeperiod - Number of periods (default: 14)
/// @returns Array of ATR values
#[napi]
pub fn atr(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::atr(&high, &low, &close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Natural Average True Range (NATR)
///
/// Normalized ATR as a percentage.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param timeperiod - Number of periods (default: 14)
/// @returns Array of NATR values (percentage)
#[napi]
pub fn natr(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::natr(&high, &low, &close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// True Range (TRANGE)
///
/// The greatest of the following: high - low, |high - prev_close|, |low - prev_close|.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @returns Array of True Range values
#[napi]
pub fn trange(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<f64>> {
    indicators::trange(&high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// On Balance Volume (OBV)
///
/// Measures buying and selling pressure using volume.
///
/// @param close - Close prices array
/// @param volume - Volume array
/// @returns Array of OBV values
#[napi]
pub fn obv(close: Vec<f64>, volume: Vec<f64>) -> Result<Vec<f64>> {
    indicators::obv(&close, &volume)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Accumulation/Distribution Line (AD)
///
/// A cumulative indicator that uses volume and price to assess accumulation/distribution.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param volume - Volume array
/// @returns Array of AD values
#[napi]
pub fn ad(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, volume: Vec<f64>) -> Result<Vec<f64>> {
    indicators::ad(&high, &low, &close, &volume)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Accumulation/Distribution Oscillator (ADOSC)
///
/// The difference between fast and slow EMA of the A/D line.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param volume - Volume array
/// @param fastperiod - Fast EMA period (default: 3)
/// @param slowperiod - Slow EMA period (default: 10)
/// @returns Array of ADOSC values
#[napi]
pub fn adosc(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    fastperiod: u32,
    slowperiod: u32,
) -> Result<Vec<f64>> {
    indicators::adosc(
        &high,
        &low,
        &close,
        &volume,
        fastperiod as usize,
        slowperiod as usize,
    )
    .map(|arr| arr.into_raw_vec())
    .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Hilbert Transform - Dominant Cycle Period (HT_DCPERIOD)
///
/// Measures the dominant cycle period of the price series.
///
/// @param close - Close prices array
/// @returns Array of dominant cycle period values
#[napi]
pub fn ht_dcperiod(close: Vec<f64>) -> Result<Vec<f64>> {
    indicators::ht_dcperiod(&close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Hilbert Transform - Dominant Cycle Phase (HT_DCPHASE)
///
/// Measures the dominant cycle phase in degrees (0-360).
///
/// @param close - Close prices array
/// @returns Array of dominant cycle phase values in degrees
#[napi]
pub fn ht_dcphase(close: Vec<f64>) -> Result<Vec<f64>> {
    indicators::ht_dcphase(&close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Hilbert Transform - Phasor Components (HT_PHASOR)
///
/// Returns the in-phase and quadrature components of the Hilbert Transform.
///
/// @param close - Close prices array
/// @returns Object containing in_phase and quadrature arrays
#[napi]
pub fn ht_phasor(close: Vec<f64>) -> Result<HtPhasorResult> {
    indicators::ht_phasor(&close)
        .map(|res| HtPhasorResult {
            in_phase: res.0.into_raw_vec(),
            quadrature: res.1.into_raw_vec(),
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Hilbert Transform - Sine Wave (HT_SINE)
///
/// Returns the sine wave and lead sine wave components.
///
/// @param close - Close prices array
/// @returns Object containing sine and lead_sine arrays
#[napi]
pub fn ht_sine(close: Vec<f64>) -> Result<HtSineResult> {
    indicators::ht_sine(&close)
        .map(|res| HtSineResult {
            sine: res.0.into_raw_vec(),
            lead_sine: res.1.into_raw_vec(),
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Hilbert Transform - Trend vs Cycle Mode (HT_TRENDMODE)
///
/// Indicates whether the market is in trend mode (1) or cycle mode (0).
///
/// @param close - Close prices array
/// @returns Array of mode values (1.0 for trend, 0.0 for cycle)
#[napi]
pub fn ht_trendmode(close: Vec<f64>) -> Result<Vec<f64>> {
    indicators::ht_trendmode(&close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Hilbert Transform - Instantaneous Trendline (HT_TRENDLINE)
///
/// Computes the instantaneous trendline with cycle components removed.
///
/// @param close - Close prices array (typically typical price)
/// @returns Array of trendline values
#[napi]
pub fn ht_trendline(close: Vec<f64>) -> Result<Vec<f64>> {
    indicators::ht_trendline(&close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Z-Score (ZSCORE)
///
/// Calculates the number of standard deviations a data point is from the rolling mean.
///
/// @param input - Input data array
/// @param timeperiod - Rolling window size
/// @returns Array of Z-Score values
#[napi]
pub fn zscore(input: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::zscore(&input, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Beta Coefficient (BETA)
///
/// Measures the volatility of an asset relative to a benchmark.
///
/// @param asset - Asset price array (e.g., stock)
/// @param benchmark - Benchmark price array (e.g., market index)
/// @param timeperiod - Rolling window size
/// @returns Array of Beta values
#[napi]
pub fn beta(asset: Vec<f64>, benchmark: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::beta(&asset, &benchmark, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Pearson Correlation (CORREL)
///
/// Calculates the rolling Pearson correlation coefficient between two series.
///
/// @param input_a - First data array
/// @param input_b - Second data array
/// @param timeperiod - Rolling window size
/// @returns Array of correlation values (-1 to 1)
#[napi]
pub fn correlation(input_a: Vec<f64>, input_b: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::correlation(&input_a, &input_b, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Standard Deviation (STDDEV)
///
/// Calculates the rolling sample standard deviation.
///
/// @param input - Input data array
/// @param timeperiod - Rolling window size
/// @param nb_dev - Number of deviations (for API compatibility)
/// @returns Array of standard deviation values
#[napi]
pub fn std_dev(input: Vec<f64>, timeperiod: u32, nb_dev: f64) -> Result<Vec<f64>> {
    indicators::std_dev(&input, timeperiod as usize, nb_dev)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Time Series Forecast (TSF)
///
/// Predicts the next value using linear regression extrapolation.
///
/// @param input - Input data array
/// @param timeperiod - Rolling window size
/// @returns Array of TSF values
#[napi]
pub fn tsf(input: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::tsf(&input, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Linear Regression (LINEAR_REG)
///
/// Calculates rolling linear regression predicted values.
///
/// @param input - Input data array
/// @param timeperiod - Rolling window size
/// @returns Array of linear regression values
#[napi]
pub fn linear_reg(input: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::linear_reg(&input, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Percent Rank (PERCENT_RANK)
///
/// Calculates the percentage rank of current value within the rolling window.
///
/// @param input - Input data array
/// @param timeperiod - Rolling window size
/// @returns Array of percent rank values (0-100)
#[napi]
pub fn percent_rank(input: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::percent_rank(&input, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Average Price (AVGPRICE)
///
/// (open + high + low + close) / 4
///
/// @param open - Open prices array
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @returns Array of average prices
#[napi]
pub fn avgprice(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<f64>> {
    indicators::avgprice(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Median Price (MEDPRICE)
///
/// (high + low) / 2
///
/// @param high - High prices array
/// @param low - Low prices array
/// @returns Array of median prices
#[napi]
pub fn medprice(high: Vec<f64>, low: Vec<f64>) -> Result<Vec<f64>> {
    indicators::medprice(&high, &low)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Typical Price (TYPPRICE)
///
/// (high + low + close) / 3
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @returns Array of typical prices
#[napi]
pub fn typprice(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<f64>> {
    indicators::typprice(&high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Weighted Close Price (WCLPRICE)
///
/// (high + low + 2 * close) / 4
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @returns Array of weighted close prices
#[napi]
pub fn wclprice(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<f64>> {
    indicators::wclprice(&high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Doji Pattern (CDLDOJI)
///
/// Open and close are virtually the same.
///
/// @param open - Open prices array
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param doji_pct - Doji threshold percentage (default: 0.1)
/// @returns Array with 100 where pattern detected, 0 otherwise
#[napi]
pub fn cdl_doji(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    doji_pct: f64,
) -> Result<Vec<i32>> {
    candlestick::doji(&open, &high, &low, &close, doji_pct)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Dragonfly Doji (CDLDRAGONFLYDOJI)
///
/// Doji with long lower shadow and little to no upper shadow.
#[napi]
pub fn cdl_dragonfly_doji(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    doji_pct: f64,
) -> Result<Vec<i32>> {
    candlestick::dragonfly_doji(&open, &high, &low, &close, doji_pct)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Gravestone Doji (CDLGRAVESTONEDOJI)
///
/// Doji with long upper shadow and little to no lower shadow.
#[napi]
pub fn cdl_gravestone_doji(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    doji_pct: f64,
) -> Result<Vec<i32>> {
    candlestick::gravestone_doji(&open, &high, &low, &close, doji_pct)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Long-Legged Doji (CDLLONGLEGGEDDOJI)
///
/// Doji with long upper and lower shadows.
#[napi]
pub fn cdl_long_legged_doji(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    doji_pct: f64,
) -> Result<Vec<i32>> {
    candlestick::long_legged_doji(&open, &high, &low, &close, doji_pct)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Hammer (CDLHAMMER)
///
/// Small body at the top, long lower shadow. Bullish reversal pattern.
#[napi]
pub fn cdl_hammer(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::hammer(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Inverted Hammer (CDLINVERTEDHAMMER)
///
/// Small body at the bottom, long upper shadow. Bullish reversal pattern.
#[napi]
pub fn cdl_inverted_hammer(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::inverted_hammer(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Hanging Man (CDLHANGINGMAN)
///
/// Same shape as Hammer but appears after uptrend. Bearish reversal.
#[napi]
pub fn cdl_hanging_man(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::hanging_man(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Shooting Star (CDLSHOOTINGSTAR)
///
/// Same shape as Inverted Hammer but appears after uptrend. Bearish reversal.
#[napi]
pub fn cdl_shooting_star(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::shooting_star(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Engulfing Pattern (CDLENGULFING)
///
/// Two-candle pattern where second candle engulfs the first.
/// Returns 100 for bullish, -100 for bearish.
#[napi]
pub fn cdl_engulfing(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::engulfing(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Harami Pattern (CDLHARAMI)
///
/// Two-candle pattern where second candle is contained within the first.
/// Returns 100 for bullish, -100 for bearish.
#[napi]
pub fn cdl_harami(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::harami(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Morning Star (CDLMORNINGSTAR)
///
/// Three-candle bullish reversal pattern.
#[napi]
pub fn cdl_morning_star(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::morning_star(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Evening Star (CDLEVENINGSTAR)
///
/// Three-candle bearish reversal pattern.
#[napi]
pub fn cdl_evening_star(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::evening_star(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Three White Soldiers (CDLTHREEWHITESOLDIERS)
///
/// Three consecutive bullish candles with higher closes.
#[napi]
pub fn cdl_three_white_soldiers(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::three_white_soldiers(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Three Black Crows (CDLTHREEBLACKCROWS)
///
/// Three consecutive bearish candles with lower closes.
#[napi]
pub fn cdl_three_black_crows(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::three_black_crows(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Marubozu (CDLMARUBOZU)
///
/// A candle with no shadows. Returns 100 for bullish, -100 for bearish.
#[napi]
pub fn cdl_marubozu(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    shadow_pct: f64,
) -> Result<Vec<i32>> {
    candlestick::marubozu(&open, &high, &low, &close, shadow_pct)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Darvas Box breakout pattern.
#[napi]
pub fn darvas_box(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    lookback: Option<u32>,
    confirmation: Option<u32>,
) -> Result<DarvasBoxResult> {
    let lb = lookback.unwrap_or(5) as usize;
    let conf = confirmation.unwrap_or(3) as usize;
    indicators::darvas_box(&high, &low, &close, lb, conf)
        .map(|r| DarvasBoxResult {
            boxTop: r.box_top.into_raw_vec(),
            boxBottom: r.box_bottom.into_raw_vec(),
            signal: r.signal.into_raw_vec(),
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Renko bricks construction.
#[napi]
pub fn renko(high: Vec<f64>, low: Vec<f64>, box_size: f64) -> Result<RenkoResult> {
    indicators::renko(&high, &low, box_size)
        .map(|r| RenkoResult {
            bricks: r.bricks.into_raw_vec(),
            direction: r.direction.into_raw_vec(),
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Kagi line construction.
#[napi]
pub fn kagi(close: Vec<f64>, reversal: f64) -> Result<KagiResult> {
    indicators::kagi(&close, reversal)
        .map(|r| KagiResult {
            kagi: r.kagi.into_raw_vec(),
            direction: r.direction.into_raw_vec(),
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Point & Figure X/O columns.
#[napi]
pub fn point_and_figure(
    high: Vec<f64>,
    low: Vec<f64>,
    box_size: f64,
    reversal: u32,
) -> Result<PnfResult> {
    indicators::point_and_figure(&high, &low, box_size, reversal as usize)
        .map(|r| PnfResult {
            pnf: r.pnf.into_raw_vec(),
            columnType: r.column_type.into_raw_vec(),
            newColumn: r.new_column.into_raw_vec(),
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Three Line Break chart.
#[napi]
pub fn three_line_break(close: Vec<f64>, lines: u32) -> Result<ThreeLineBreakResult> {
    indicators::three_line_break(&close, lines as usize)
        .map(|r| ThreeLineBreakResult {
            line: r.line.into_raw_vec(),
            direction: r.direction.into_raw_vec(),
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Williams Alligator (5/8/13 SMMA, Bill Williams).
#[napi]
pub fn williams_alligator(close: Vec<f64>) -> Result<WilliamsAlligatorResult> {
    indicators::williams_alligator(&close)
        .map(|r| WilliamsAlligatorResult {
            jaw: r.jaw.into_raw_vec(),
            teeth: r.teeth.into_raw_vec(),
            lips: r.lips.into_raw_vec(),
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Heikin-Ashi candlestick construction.
#[napi]
pub fn heikin_ashi(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<HeikinAshiResult> {
    indicators::heikin_ashi(&open, &high, &low, &close)
        .map(|r| HeikinAshiResult {
            haOpen: r.ha_open.into_raw_vec(),
            haHigh: r.ha_high.into_raw_vec(),
            haLow: r.ha_low.into_raw_vec(),
            haClose: r.ha_close.into_raw_vec(),
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

