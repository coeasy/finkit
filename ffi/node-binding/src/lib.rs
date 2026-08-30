#![allow(non_snake_case, dead_code, missing_docs, missing_debug_implementations, deprecated)]

use napi::bindgen_prelude::*;
use napi_derive::napi;

use alpha_ta_core::indicators;
use alpha_ta_core::math::moving_avg;
use alpha_ta_core::patterns::{candlestick, chart};

mod streaming;
mod sweep;
mod transforms;

#[cfg(feature = "formula")]
use ndarray::Array1;
#[cfg(feature = "formula")]
use alpha_ta_core::formula::{parse_formula, FormulaContext, FormulaEngine, FormulaError};
#[cfg(feature = "formula")]
use std::collections::HashMap;

#[cfg(feature = "formula")]
fn formula_error_to_napi(e: FormulaError) -> napi::Error {
    match e {
        FormulaError::ParseError(msg) => napi::Error::new(napi::Status::InvalidArg, format!("Parse error: {}", msg)),
        FormulaError::Parse { line, col, message } => napi::Error::new(
            napi::Status::InvalidArg,
            format!("Parse error at line {}, col {}: {}", line, col, message),
        ),
        FormulaError::UndefinedFunction { name } => napi::Error::new(
            napi::Status::InvalidArg,
            format!("Undefined function: {}", name),
        ),
        FormulaError::TypeMismatch { expected, actual } => napi::Error::new(
            napi::Status::InvalidArg,
            format!("Type mismatch: expected {}, got {}", expected, actual),
        ),
        FormulaError::RuntimeError(msg) => napi::Error::new(napi::Status::GenericFailure, format!("Runtime error: {}", msg)),
        FormulaError::InvalidParameter(msg) => napi::Error::new(napi::Status::InvalidArg, format!("Invalid parameter: {}", msg)),
        FormulaError::InsufficientData(msg) => napi::Error::new(napi::Status::InvalidArg, format!("Insufficient data: {}", msg)),
        FormulaError::InvalidOperation(msg) => napi::Error::new(napi::Status::InvalidArg, format!("Invalid operation: {}", msg)),
        FormulaError::UnsupportedFunction(msg) => napi::Error::new(napi::Status::InvalidArg, format!("Unsupported function: {}", msg)),
        FormulaError::Timeout { elapsed_ms } => napi::Error::new(napi::Status::GenericFailure, format!("Execution timeout after {}ms", elapsed_ms)),
        FormulaError::MemoryLimit { used, limit } => napi::Error::new(napi::Status::GenericFailure, format!("Memory limit exceeded: used {} bytes, limit {} bytes", used, limit)),
    }
}

use alpha_ta_core::error::TaError;

// ============================================================================
// Overlap Studies - Moving Averages
// ============================================================================


include!("generated.rs");












/// MESA Adaptive Moving Average (MAMA)
///
/// Uses the Hilbert Transform to adapt to market cycles.
///
/// @param close - Close prices array
/// @param fastlimit - Fast limit (default: 0.5)
/// @param slowlimit - Slow limit (default: 0.05)
/// @returns Object containing mama and fama arrays
#[napi(object)]
pub struct MamaResult {
    pub mama: Vec<f64>,
    pub fama: Vec<f64>,
}

/// Result of Darvas Box pattern detection.
#[napi(object)]
pub struct DarvasBoxResult {
    pub boxTop: Vec<f64>,
    pub boxBottom: Vec<f64>,
    pub signal: Vec<i32>,
}

/// Result of Renko brick construction.
#[napi(object)]
pub struct RenkoResult {
    pub bricks: Vec<f64>,
    pub direction: Vec<i32>,
}

/// Result of Kagi line construction.
#[napi(object)]
pub struct KagiResult {
    pub kagi: Vec<f64>,
    pub direction: Vec<i32>,
}

/// Result of Point & Figure chart.
#[napi(object)]
pub struct PnfResult {
    pub pnf: Vec<f64>,
    pub columnType: Vec<i32>,
    pub newColumn: Vec<i32>,
}

/// Result of Three Line Break chart.
#[napi(object)]
pub struct ThreeLineBreakResult {
    pub line: Vec<f64>,
    pub direction: Vec<i32>,
}

/// Result of Williams Alligator indicator.
#[napi(object)]
pub struct WilliamsAlligatorResult {
    pub jaw: Vec<f64>,
    pub teeth: Vec<f64>,
    pub lips: Vec<f64>,
}

/// Result of Heikin-Ashi candlestick.
#[napi(object)]
pub struct HeikinAshiResult {
    pub haOpen: Vec<f64>,
    pub haHigh: Vec<f64>,
    pub haLow: Vec<f64>,
    pub haClose: Vec<f64>,
}

// ============================================================================
// Classic stock-trading chart patterns (FTA-native, added 2026-06-06).
// ============================================================================



















// ============================================================================
// Momentum Indicators
// ============================================================================



/// MACD Result structure
#[napi(object)]
pub struct MacdResult {
    pub macd: Vec<f64>,
    pub signal: Vec<f64>,
    pub hist: Vec<f64>,
}



/// MACD with custom moving average type (async for large datasets)
///
/// @param close - Close prices array
/// @param fastperiod - Fast period
/// @param slowperiod - Slow period
/// @param signalperiod - Signal period
/// @returns Promise resolving to MacdResult
#[napi]
pub async fn macd_async(
    close: Vec<f64>,
    fastperiod: u32,
    slowperiod: u32,
    signalperiod: u32,
) -> Result<MacdResult> {
    let fast = fastperiod as usize;
    let slow = slowperiod as usize;
    let signal = signalperiod as usize;

    let result = napi::tokio::task::spawn_blocking(move || {
        indicators::macd(&close, fast, slow, signal).map(|res| MacdResult {
            macd: res.macd.into_raw_vec(),
            signal: res.signal.into_raw_vec(),
            hist: res.hist.into_raw_vec(),
        })
    })
    .await
    .map_err(|e| Error::new(Status::GenericFailure, format!("Task panicked: {}", e)))?
    .map_err(|e: TaError| Error::new(Status::InvalidArg, format!("{}", e)))?;

    Ok(result)
}

/// Stochastic Oscillator Result
#[napi(object)]
pub struct StochResult {
    pub k: Vec<f64>,
    pub d: Vec<f64>,
}











/// Aroon Indicator Result
#[napi(object)]
pub struct AroonResult {
    pub aroon_up: Vec<f64>,
    pub aroon_down: Vec<f64>,
}

















/// Directional Movement Index (DX)
///
/// Measures trend direction and strength.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param timeperiod - Number of periods
/// @returns Array of DX values
#[napi]
pub fn dx(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, timeperiod: u32) -> Result<Vec<f64>> {
    indicators::dx(&high, &low, &close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}



/// Minus Directional Indicator (MINUS_DI)
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param timeperiod - Number of periods
/// @returns Array of MINUS_DI values
#[napi]
pub fn minus_di(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    timeperiod: u32,
) -> Result<Vec<f64>> {
    indicators::minus_di(&high, &low, &close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Plus Directional Indicator (PLUS_DI)
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param timeperiod - Number of periods
/// @returns Array of PLUS_DI values
#[napi]
pub fn plus_di(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    timeperiod: u32,
) -> Result<Vec<f64>> {
    indicators::plus_di(&high, &low, &close, timeperiod as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}



// ============================================================================
// Volume Indicators
// ============================================================================





// ============================================================================
// Volatility Indicators
// ============================================================================

/// Bollinger Bands Result
#[napi(object)]
pub struct BbandsResult {
    pub upper: Vec<f64>,
    pub middle: Vec<f64>,
    pub lower: Vec<f64>,
}







// ============================================================================
// Cycle Indicators (Hilbert Transform)
// ============================================================================





/// Hilbert Transform - Phasor Components Result
#[napi(object)]
pub struct HtPhasorResult {
    pub in_phase: Vec<f64>,
    pub quadrature: Vec<f64>,
}



/// Hilbert Transform - Sine Wave Result
#[napi(object)]
pub struct HtSineResult {
    pub sine: Vec<f64>,
    pub lead_sine: Vec<f64>,
}







// ============================================================================
// Price Transforms
// ============================================================================









// ============================================================================
// Statistical Indicators
// ============================================================================















// ============================================================================
// Candlestick Patterns
// ============================================================================





















/// Harami Cross (CDLHARAMICROSS)
///
/// Harami where second candle is a Doji.
/// Returns 100 for bullish, -100 for bearish.
#[napi]
pub fn cdl_harami_cross(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::harami_cross(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}





/// Morning Doji Star (CDLMORNINGDOJISTAR)
///
/// Like Morning Star but second candle is a Doji.
#[napi]
pub fn cdl_morning_doji_star(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    doji_pct: f64,
) -> Result<Vec<i32>> {
    candlestick::morning_doji_star(&open, &high, &low, &close, doji_pct)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Evening Doji Star (CDLEVENINGDOJISTAR)
///
/// Like Evening Star but second candle is a Doji.
#[napi]
pub fn cdl_evening_doji_star(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    doji_pct: f64,
) -> Result<Vec<i32>> {
    candlestick::evening_doji_star(&open, &high, &low, &close, doji_pct)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}







/// Piercing Pattern (CDLPIERCING)
///
/// Two-candle bullish reversal pattern.
#[napi]
pub fn cdl_piercing(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::piercing(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Dark Cloud Cover (CDLDARKCLOUDCOVER)
///
/// Two-candle bearish reversal pattern.
#[napi]
pub fn cdl_dark_cloud_cover(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::dark_cloud_cover(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Belt Hold (CDLBELTHOLD)
///
/// Opens at its high (bearish) or low (bullish) with no shadow.
#[napi]
pub fn cdl_belt_hold(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::belt_hold(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Spinning Top (CDLSPINNINGTOP)
///
/// Small body with upper and lower shadows of similar length.
#[napi]
pub fn cdl_spinning_top(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::spinning_top(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// High Wave (CDLHIGHWAVE)
///
/// Similar to Spinning Top but with longer shadows.
#[napi]
pub fn cdl_high_wave(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::high_wave(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Rickshaw Man (CDLRICKSHAWMAN)
///
/// A Doji with long upper and lower shadows near midpoint.
#[napi]
pub fn cdl_rickshaw_man(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::rickshaw_man(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Tweezer Top (CDLTWEEZERTOP)
///
/// Two candles with matching highs after uptrend.
#[napi]
pub fn cdl_tweezer_top(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::tweezer_top(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Tweezer Bottom (CDLTWEEZERBOT)
///
/// Two candles with matching lows after downtrend.
#[napi]
pub fn cdl_tweezer_bot(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::tweezer_bot(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Kicking (CDLKICKING)
///
/// Two candles with gaps and opposite colors.
#[napi]
pub fn cdl_kicking(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>> {
    candlestick::kicking(&open, &high, &low, &close)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

// ============================================================================
// Chart Patterns
// ============================================================================

/// Head and Shoulders Pattern
#[napi(object)]
pub struct HeadShouldersPattern {
    pub left: u32,
    pub head: u32,
    pub right: u32,
}

/// Detect Head and Shoulders Top Pattern
///
/// @param high - High prices array
/// @param min_bars - Minimum bars between peaks
/// @param head_ratio - Head to shoulder ratio threshold (default: 1.1)
/// @returns Array of detected patterns with left, head, right indices
#[napi]
pub fn detect_head_shoulders(high: Vec<f64>, min_bars: u32, head_ratio: f64) -> Result<Vec<u32>> {
    chart::head_and_shoulders_top(&high, min_bars as usize, head_ratio)
        .map(|arr| {
            arr.iter()
                .enumerate()
                .filter(|(_, &v)| v == 1)
                .map(|(i, _)| i as u32)
                .collect()
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Detect Double Top Pattern
///
/// @param high - High prices array
/// @param lookback - Lookback period for peak detection
/// @param tolerance - Price tolerance for matching peaks (percentage)
/// @returns Array of indices where double tops are detected
#[napi]
pub fn detect_double_top(high: Vec<f64>, lookback: u32, tolerance: f64) -> Result<Vec<u32>> {
    chart::double_top(&high, lookback as usize, tolerance)
        .map(|arr| arr.into_raw_vec().into_iter().map(|v| v as u32).collect())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Detect Double Bottom Pattern
///
/// @param low - Low prices array
/// @param lookback - Lookback period for trough detection
/// @param tolerance - Price tolerance for matching troughs (percentage)
/// @returns Array of indices where double bottoms are detected
#[napi]
pub fn detect_double_bottom(low: Vec<f64>, lookback: u32, tolerance: f64) -> Result<Vec<u32>> {
    chart::double_bottom(&low, lookback as usize, tolerance)
        .map(|arr| arr.into_raw_vec().into_iter().map(|v| v as u32).collect())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Detect Head and Shoulders Bottom (Inverse) Pattern
///
/// @param low - Low prices array
/// @param min_bars - Minimum bars between troughs
/// @param head_ratio - Head to shoulder ratio threshold
/// @returns Array of detected inverse H&S patterns
#[napi]
pub fn detect_head_shoulders_bottom(
    low: Vec<f64>,
    min_bars: u32,
    head_ratio: f64,
) -> Result<Vec<u32>> {
    chart::head_and_shoulders_bottom(&low, min_bars as usize, head_ratio)
        .map(|arr| {
            arr.iter()
                .enumerate()
                .filter(|(_, &v)| v == 1)
                .map(|(i, _)| i as u32)
                .collect()
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Detect Triple Top Pattern
///
/// @param high - High prices array
/// @param lookback - Lookback period
/// @param tolerance - Price tolerance (percentage)
/// @returns Array of indices where triple tops are detected
#[napi]
pub fn detect_triple_top(high: Vec<f64>, lookback: u32, tolerance: f64) -> Result<Vec<u32>> {
    chart::triple_top(&high, lookback as usize, tolerance)
        .map(|arr| arr.into_raw_vec().into_iter().map(|v| v as u32).collect())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Detect Triple Bottom Pattern
///
/// @param low - Low prices array
/// @param lookback - Lookback period
/// @param tolerance - Price tolerance (percentage)
/// @returns Array of indices where triple bottoms are detected
#[napi]
pub fn detect_triple_bottom(low: Vec<f64>, lookback: u32, tolerance: f64) -> Result<Vec<u32>> {
    chart::triple_bottom(&low, lookback as usize, tolerance)
        .map(|arr| arr.into_raw_vec().into_iter().map(|v| v as u32).collect())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

// ============================================================================
// Advanced Indicators
// ============================================================================

/// Ichimoku Cloud Result
///
/// Contains all five lines of the Ichimoku Kinko Hyo indicator.
#[napi(object)]
pub struct IchimokuResult {
    /// Tenkan-sen (Conversion Line) = (9-period high + 9-period low) / 2
    pub tenkanSen: Vec<f64>,
    /// Kijun-sen (Base Line) = (26-period high + 26-period low) / 2
    pub kijunSen: Vec<f64>,
    /// Senkou Span A (Leading Span A) = (Tenkan-sen + Kijun-sen) / 2, shifted forward
    pub senkouSpanA: Vec<f64>,
    /// Senkou Span B (Leading Span B) = (52-period high + 52-period low) / 2, shifted forward
    pub senkouSpanB: Vec<f64>,
    /// Chikou Span (Lagging Span) = Close price, shifted backward
    pub chikouSpan: Vec<f64>,
}

/// Ichimoku Cloud (Ichimoku Kinko Hyo)
///
/// A comprehensive indicator that shows support and resistance, identifies trend direction,
/// gauges momentum, and provides trading signals.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param tenkan_period - Tenkan-sen period (default: 9)
/// @param kijun_period - Kijun-sen period (default: 26)
/// @param senkou_b_period - Senkou Span B period (default: 52)
/// @returns Object containing tenkanSen, kijunSen, senkouSpanA, senkouSpanB, chikouSpan arrays
#[napi]
pub fn ichimoku(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    tenkan_period: u32,
    kijun_period: u32,
    senkou_b_period: u32,
) -> Result<IchimokuResult> {
    let displacement = kijun_period as usize;
    indicators::ichimoku(
        &high,
        &low,
        &close,
        tenkan_period as usize,
        kijun_period as usize,
        senkou_b_period as usize,
        displacement,
    )
    .map(|res| IchimokuResult {
        tenkanSen: res.tenkan_sen.into_raw_vec(),
        kijunSen: res.kijun_sen.into_raw_vec(),
        senkouSpanA: res.senkou_span_a.into_raw_vec(),
        senkouSpanB: res.senkou_span_b.into_raw_vec(),
        chikouSpan: res.chikou_span.into_raw_vec(),
    })
    .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// SuperTrend Trend Following Result
///
/// Contains trend direction, trend line, and upper/lower bands.
#[napi(object)]
pub struct SuperTrendResult {
    /// Trend direction: 1 for bullish (up), -1 for bearish (down)
    pub direction: Vec<i32>,
    /// SuperTrend trend line (current support/resistance line)
    pub trendLine: Vec<f64>,
    /// Upper band
    pub upperBand: Vec<f64>,
    /// Lower band
    pub lowerBand: Vec<f64>,
}

/// SuperTrend Trend Following Indicator
///
/// A volatility-based trend following indicator that calculates bands based on ATR.
/// Returns the trend direction, trend line, and upper/lower bands.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param period - ATR calculation period (default: 10)
/// @param multiplier - ATR multiplier (default: 3.0)
/// @returns Object containing direction, trendLine, upperBand, lowerBand arrays
#[napi]
pub fn supertrend(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: u32,
    multiplier: f64,
) -> Result<SuperTrendResult> {
    indicators::supertrend(&high, &low, &close, period as usize, multiplier)
        .map(|res| SuperTrendResult {
            direction: res.direction.into_raw_vec(),
            trendLine: res.trend_line.into_raw_vec(),
            upperBand: res.upper_band.into_raw_vec(),
            lowerBand: res.lower_band.into_raw_vec(),
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Volume Weighted Average Price (VWAP)
///
/// A trading benchmark that represents the average price a security has traded at
/// throughout the day, based on both volume and price.
///
/// Formula: VWAP = Σ(Typical Price × Volume) / Σ(Volume)
/// where Typical Price = (High + Low + Close) / 3
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param volume - Volume array
/// @returns Array of VWAP values
#[napi]
pub fn vwap(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, volume: Vec<f64>) -> Result<Vec<f64>> {
    indicators::vwap(&high, &low, &close, &volume)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Anchored Volume Weighted Average Price (Anchored VWAP)
///
/// Similar to VWAP, but allows traders to specify a starting point (anchor) from which
/// the calculation begins. Useful for measuring average price from significant events.
///
/// Formula: Anchored VWAP = Σ(Typical Price × Volume) / Σ(Volume)
/// where the summation starts from start_index
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param volume - Volume array
/// @param start_index - The index from which to start calculating VWAP
/// @returns Array of Anchored VWAP values (NaN for indices before start_index)
#[napi]
pub fn anchored_vwap(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    start_index: u32,
) -> Result<Vec<f64>> {
    indicators::anchored_vwap(&high, &low, &close, &volume, start_index as usize)
        .map(|arr| arr.into_raw_vec())
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// VWAP Bands Result
///
/// Contains VWAP line with upper and lower bands based on standard deviation.
#[napi(object)]
pub struct VwapBandsResult {
    /// VWAP line (center band)
    pub vwap: Vec<f64>,
    /// Upper band (VWAP + nb_dev × std_dev)
    pub upper: Vec<f64>,
    /// Lower band (VWAP - nb_dev × std_dev)
    pub lower: Vec<f64>,
}

/// Volume Weighted Average Price Bands (VWAP Bands)
///
/// VWAP Bands consist of the VWAP line with upper and lower bands based on standard
/// deviation. These bands help identify overbought and oversold levels relative to
/// the volume-weighted average price.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param volume - Volume array
/// @param timeperiod - Lookback period for standard deviation calculation
/// @param nb_dev - Number of standard deviations for the bands (default: 2.0)
/// @returns Object containing vwap, upper, and lower arrays
#[napi]
pub fn vwap_bands(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    timeperiod: u32,
    nb_dev: f64,
) -> Result<VwapBandsResult> {
    indicators::vwap_bands(&high, &low, &close, &volume, timeperiod as usize, nb_dev)
        .map(|res| VwapBandsResult {
            vwap: res.vwap.into_raw_vec(),
            upper: res.upper.into_raw_vec(),
            lower: res.lower.into_raw_vec(),
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Elder-Ray Indicator Result
///
/// Contains Force Index, Bull Power, and Bear Power.
#[napi(object)]
pub struct ElderRayResult {
    /// Force Index: (Close - Close[1]) × Volume
    pub forceIndex: Vec<f64>,
    /// Bull Power: High - EMA(Close, period)
    pub bullPower: Vec<f64>,
    /// Bear Power: Low - EMA(Close, period)
    pub bearPower: Vec<f64>,
}

/// Elder-Ray Indicator (ELDER-RAY)
///
/// Developed by Alexander Elder, this indicator uses three components to evaluate
/// the balance of power between bulls and bears in the market.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param volume - Volume array
/// @param period - EMA lookback period for Bull/Bear Power calculation
/// @returns Object containing forceIndex, bullPower, bearPower arrays
#[napi]
pub fn elder_ray(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    period: u32,
) -> Result<ElderRayResult> {
    indicators::elder_ray(&high, &low, &close, &volume, period as usize)
        .map(|res| ElderRayResult {
            forceIndex: res.force_index.into_raw_vec(),
            bullPower: res.bull_power.into_raw_vec(),
            bearPower: res.bear_power.into_raw_vec(),
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Donchian Channel Result
///
/// Contains upper, lower, middle bands and width.
#[napi(object)]
pub struct DonchianResult {
    /// Upper Band - N-period highest high
    pub upper: Vec<f64>,
    /// Lower Band - N-period lowest low
    pub lower: Vec<f64>,
    /// Middle Band - (Upper + Lower) / 2
    pub middle: Vec<f64>,
    /// Width - Upper - Lower
    pub width: Vec<f64>,
}

/// Donchian Channel (DONCHIAN)
///
/// A trend-following indicator that displays the highest and lowest prices over a given period.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param period - Lookback period
/// @returns Object containing upper, lower, middle, width arrays
#[napi]
pub fn donchian(high: Vec<f64>, low: Vec<f64>, period: u32) -> Result<DonchianResult> {
    indicators::donchian(&high, &low, period as usize)
        .map(|res| DonchianResult {
            upper: res.upper.into_raw_vec(),
            lower: res.lower.into_raw_vec(),
            middle: res.middle.into_raw_vec(),
            width: res.width.into_raw_vec(),
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Pivot Points calculation method enum for Node.js
#[napi]
pub enum PivotMethod {
    /// Standard (Floor) Pivots - Traditional floor trader pivots
    Standard = 0,
    /// Fibonacci Pivots - Uses Fibonacci ratios
    Fibonacci = 1,
    /// Woodie's Pivots - Gives more weight to the close
    Woodie = 2,
    /// Camarilla Pivots - Tighter ranges using specific ratios
    Camarilla = 3,
    /// DeMark Pivots - Simplified calculation based on open/close relationship
    DeMark = 4,
}

/// Pivot Points Result
///
/// Contains pivot point and support/resistance levels.
#[napi(object)]
pub struct PivotResult {
    /// Pivot Point
    pub pivot: Vec<f64>,
    /// Resistance Level 1
    pub r1: Vec<f64>,
    /// Resistance Level 2
    pub r2: Vec<f64>,
    /// Resistance Level 3
    pub r3: Vec<f64>,
    /// Support Level 1
    pub s1: Vec<f64>,
    /// Support Level 2
    pub s2: Vec<f64>,
    /// Support Level 3
    pub s3: Vec<f64>,
}

/// Pivot Points (PIVOT)
///
/// Calculates pivot points and support/resistance levels based on previous period's
/// high, low, and close prices.
///
/// @param high - High prices array (previous period highs)
/// @param low - Low prices array (previous period lows)
/// @param close - Close prices array (previous period closes)
/// @param method - Calculation method (0=Standard, 1=Fibonacci, 2=Woodie, 3=Camarilla, 4=DeMark)
/// @returns Object containing pivot, r1, r2, r3, s1, s2, s3 arrays
#[napi]
pub fn pivot_points(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    method: PivotMethod,
) -> Result<PivotResult> {
    let pivot_method = match method {
        PivotMethod::Standard => indicators::PivotMethod::Standard,
        PivotMethod::Fibonacci => indicators::PivotMethod::Fibonacci,
        PivotMethod::Woodie => indicators::PivotMethod::Woodie,
        PivotMethod::Camarilla => indicators::PivotMethod::Camarilla,
        PivotMethod::DeMark => indicators::PivotMethod::DeMark,
    };

    indicators::pivot_points(&high, &low, &close, pivot_method)
        .map(|res| PivotResult {
            pivot: res.pivot.into_raw_vec(),
            r1: res.r1.into_raw_vec(),
            r2: res.r2.into_raw_vec(),
            r3: res.r3.into_raw_vec(),
            s1: res.s1.into_raw_vec(),
            s2: res.s2.into_raw_vec(),
            s3: res.s3.into_raw_vec(),
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Volume Profile Result
///
/// Contains Point of Control (POC), Value Area High (VAH), and Value Area Low (VAL).
#[napi(object)]
pub struct VolumeProfileResult {
    /// Point of Control - price level with the highest traded volume
    pub poc: f64,
    /// Value Area High - upper boundary of the 70% value area
    pub vah: f64,
    /// Value Area Low - lower boundary of the 70% value area
    pub val: f64,
}

/// Volume Profile
///
/// Divides the price range into bins and calculates the volume traded at each price level.
/// Used to identify key price levels where significant trading activity occurred.
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param close - Close prices array
/// @param volume - Volume array
/// @param num_bins - Number of price bins for the profile
/// @returns Object containing poc, vah, val
#[napi]
pub fn volume_profile(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    num_bins: u32,
) -> Result<VolumeProfileResult> {
    indicators::volume_profile(&high, &low, &close, &volume, num_bins as usize)
        .map(|res| VolumeProfileResult {
            poc: res.poc,
            vah: res.vah,
            val: res.val,
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

/// Fibonacci Retracement Level
///
/// Contains the Fibonacci ratio and corresponding price level.
#[napi(object)]
pub struct FibLevel {
    /// Fibonacci ratio (e.g., 0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0, 1.272, 1.618)
    pub ratio: f64,
    /// Price level at the Fibonacci ratio
    pub price: f64,
}

/// Fibonacci Retracement Result
///
/// Contains all Fibonacci retracement and extension levels.
#[napi(object)]
pub struct FibonacciResult {
    /// Array of Fibonacci levels with ratio and price
    pub levels: Vec<FibLevel>,
    /// Trend direction: 1 for uptrend (low before high), -1 for downtrend
    pub trend: i32,
    /// Highest price in the range
    pub highPrice: f64,
    /// Lowest price in the range
    pub lowPrice: f64,
    /// Index of the highest price
    pub highIndex: u32,
    /// Index of the lowest price
    pub lowIndex: u32,
}

/// Fibonacci Retracement
///
/// Automatically detects the highest and lowest prices in the specified range,
/// determines trend direction, and calculates all standard Fibonacci retracement
/// and extension levels.
///
/// # Trend Detection
/// - **Uptrend** (trend = 1): Low point occurs before high point
///   - Retracement calculated from low to high
///   - Extensions above the high
///
/// - **Downtrend** (trend = -1): High point occurs before low point
///   - Retracement calculated from high to low
///   - Extensions below the low
///
/// # Fibonacci Levels
/// - Retracement: 0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0
/// - Extension: 1.272, 1.618
///
/// @param high - High prices array
/// @param low - Low prices array
/// @param start_index - Start index of the range (inclusive)
/// @param end_index - End index of the range (inclusive)
/// @returns Object containing levels, trend, highPrice, lowPrice, highIndex, lowIndex
#[napi]
pub fn fibonacci_retracement(
    high: Vec<f64>,
    low: Vec<f64>,
    start_index: u32,
    end_index: u32,
) -> Result<FibonacciResult> {
    indicators::fibonacci_retracement(&high, &low, start_index as usize, end_index as usize)
        .map(|res| FibonacciResult {
            levels: res
                .levels
                .into_iter()
                .map(|l| FibLevel {
                    ratio: l.ratio,
                    price: l.price,
                })
                .collect(),
            trend: res.trend,
            highPrice: res.high_price,
            lowPrice: res.low_price,
            highIndex: res.high_index as u32,
            lowIndex: res.low_index as u32,
        })
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))
}

#[napi(object)]
pub struct KlineDataNapi {
    pub dates: Vec<String>,
    pub opens: Vec<f64>,
    pub highs: Vec<f64>,
    pub lows: Vec<f64>,
    pub closes: Vec<f64>,
    pub volumes: Vec<f64>,
}

impl From<KlineDataNapi> for alpha_ta_visualization::data::KlineData {
    fn from(data: KlineDataNapi) -> Self {
        Self::new(
            data.dates,
            data.opens,
            data.highs,
            data.lows,
            data.closes,
            data.volumes,
        )
    }
}

impl From<alpha_ta_visualization::data::KlineData> for KlineDataNapi {
    fn from(data: alpha_ta_visualization::data::KlineData) -> Self {
        Self {
            dates: data.dates,
            opens: data.opens,
            highs: data.highs,
            lows: data.lows,
            closes: data.closes,
            volumes: data.volumes,
        }
    }
}

#[napi]
pub fn kline_data_new(
    dates: Vec<String>,
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    volumes: Vec<f64>,
) -> KlineDataNapi {
    KlineDataNapi {
        dates,
        opens,
        highs,
        lows,
        closes,
        volumes,
    }
}

#[napi]
pub fn kline_data_validate(data: KlineDataNapi) -> bool {
    let inner: alpha_ta_visualization::data::KlineData = data.into();
    inner.validate()
}

#[napi]
pub struct KlineChartNapi {
    inner: alpha_ta_visualization::chart::KlineChart,
    data: Option<alpha_ta_visualization::data::KlineData>,
}

#[napi]
impl KlineChartNapi {
    #[napi(constructor)]
    pub fn new(
        data: KlineDataNapi,
        language: String,
        title: String,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let inner_data: alpha_ta_visualization::data::KlineData = data.into();
        let lang = match language.as_str() {
            "zh-CN" | "zh" => alpha_ta_visualization::language::Language::ZhCn,
            _ => alpha_ta_visualization::language::Language::EnUs,
        };
        let config = alpha_ta_visualization::config::ChartConfigBuilder::new()
            .with_title(&title)
            .with_language(lang)
            .with_dimensions(width, height)
            .build();
        let mut chart = alpha_ta_visualization::chart::KlineChart::new(config);
        chart.set_data(inner_data.clone());
        chart
            .build_draw_list(&inner_data, &[])
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;
        Ok(Self {
            inner: chart,
            data: Some(inner_data),
        })
    }

    #[napi]
    pub fn add_ma(&mut self, periods: Vec<u32>) {
        if let Some(ref data) = self.data {
            let p: Vec<usize> = periods.iter().map(|&x| x as usize).collect();
            self.inner.add_ma(data, &p);
        }
    }

    #[napi]
    pub fn add_macd(&mut self, fast: u32, slow: u32, signal: u32) {
        if let Some(ref data) = self.data {
            self.inner
                .add_macd(data, fast as usize, slow as usize, signal as usize, 1);
        }
    }

    #[napi]
    pub fn add_rsi(&mut self, period: u32) {
        if let Some(ref data) = self.data {
            self.inner.add_rsi(data, period as usize, 1);
        }
    }

    #[napi]
    pub fn add_boll(&mut self, period: u32, nb_dev: f64) {
        if let Some(ref data) = self.data {
            self.inner.add_boll(data, period as usize, nb_dev);
        }
    }

    #[napi]
    pub fn save_as_svg(&mut self, path: String) -> Result<()> {
        self.inner
            .save_as_svg(&path)
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))
    }

    #[napi]
    pub fn to_svg(&mut self) -> Result<String> {
        self.inner
            .to_svg_string()
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))
    }
}

// ============================================================================
// New Indicators (TASK-166~180)
// ============================================================================















// ============================================================================
// Formula System
// ============================================================================

/// Execute a trading formula
///
/// This function compiles and executes a trading formula string similar to
/// TongDaXin (通达信) formula language.
///
/// @param source - Formula source code
/// @param open - Opening prices
/// @param high - High prices
/// @param low - Low prices
/// @param close - Closing prices
/// @param volume - Trading volume
/// @returns Object with output variable names as keys and arrays as values.
///          The special key "__result__" contains the final expression result.
///
/// @example
/// ```javascript
/// const result = formulaEval(
///     "MA5:=MA(C,5); MA10:=MA(C,10); CROSS(MA5, MA10)",
///     open, high, low, close, volume
/// );
/// console.log(result.MA5);
/// console.log(result.MA10);
/// console.log(result.__result__);
/// ```
#[napi]
#[cfg(feature = "formula")]
pub fn formula_eval(
    source: String,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
) -> Result<HashMap<String, Vec<f64>>> {
    let open_array = Array1::from_vec(open);
    let high_array = Array1::from_vec(high);
    let low_array = Array1::from_vec(low);
    let close_array = Array1::from_vec(close);
    let volume_array = Array1::from_vec(volume);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        None,
    );
    let mut engine = FormulaEngine::new();

    let result = engine.eval(&source, &mut ctx).map_err(formula_error_to_napi)?;

    let mut output = HashMap::new();

    for (name, value) in ctx.variables {
        output.insert(name.to_string(), value.to_vec());
    }

    output.insert("__result__".to_string(), result.to_vec());

    Ok(output)
}

#[napi(object)]
#[cfg(feature = "formula")]
pub struct FormulaMultiResult {
    pub names: Vec<String>,
    pub values: Vec<Vec<f64>>,
    pub __result__: Vec<f64>,
}

#[napi]
#[cfg(feature = "formula")]
pub fn formula_eval_multi(
    source: String,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
) -> Result<FormulaMultiResult> {
    let open_array = Array1::from_vec(open);
    let high_array = Array1::from_vec(high);
    let low_array = Array1::from_vec(low);
    let close_array = Array1::from_vec(close);
    let volume_array = Array1::from_vec(volume);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        None,
    );
    let mut engine = FormulaEngine::new();

    let multi = engine.eval_multi(&source, &mut ctx).map_err(formula_error_to_napi)?;

    let mut names = Vec::new();
    let mut values = Vec::new();
    for name in multi.names() {
        names.push(name.clone());
        if let Some(arr) = multi.get(name) {
            values.push(arr.to_vec());
        } else {
            values.push(vec![]);
        }
    }

    Ok(FormulaMultiResult {
        names,
        values,
        __result__: multi.final_value.to_vec(),
    })
}

#[napi]
#[cfg(feature = "formula")]
pub fn formula_eval_draw(
    source: String,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
) -> Result<String> {
    let open_array = Array1::from_vec(open);
    let high_array = Array1::from_vec(high);
    let low_array = Array1::from_vec(low);
    let close_array = Array1::from_vec(close);
    let volume_array = Array1::from_vec(volume);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        None,
    );
    let mut engine = FormulaEngine::new();

    let _result = engine.eval(&source, &mut ctx).map_err(formula_error_to_napi)?;

    let draw_commands = ctx.draw_commands.borrow();
    let json_value = serde_json::json!({
        "drawCommands": &draw_commands.commands,
    });
    let json_str = serde_json::to_string(&json_value).unwrap_or_else(|_| "{}".to_string());
    Ok(json_str)
}

#[napi]
#[cfg(feature = "formula")]
pub fn formula_eval_debug(
    source: String,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
) -> Result<String> {
    let open_array = Array1::from_vec(open);
    let high_array = Array1::from_vec(high);
    let low_array = Array1::from_vec(low);
    let close_array = Array1::from_vec(close);
    let volume_array = Array1::from_vec(volume);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        None,
    );
    let mut engine = FormulaEngine::new();

    let (_result, debugger) = engine.eval_with_debug(&source, &mut ctx).map_err(formula_error_to_napi)?;

    let json_value = serde_json::json!({
        "events": debugger.get_events(),
    });
    let json_str = serde_json::to_string(&json_value).unwrap_or_else(|_| "{}".to_string());
    Ok(json_str)
}

#[napi(object)]
#[cfg(feature = "formula")]
pub struct FormulaTemplateInfo {
    pub name: String,
    pub description: String,
    pub category: String,
    pub source: String,
}

#[napi]
#[cfg(feature = "formula")]
pub fn formula_get_template(name: String) -> Result<Option<FormulaTemplateInfo>> {
    let engine = FormulaEngine::new();
    match engine.get_template(&name) {
        Some(template) => Ok(Some(FormulaTemplateInfo {
            name: template.name.clone(),
            description: template.description.clone(),
            category: format!("{:?}", template.category),
            source: template.source.clone(),
        })),
        None => Ok(None),
    }
}

#[napi]
#[cfg(feature = "formula")]
pub fn formula_search_templates(keyword: String) -> Result<Vec<FormulaTemplateInfo>> {
    let engine = FormulaEngine::new();
    let templates = engine.search_templates(&keyword);
    Ok(templates
        .iter()
        .map(|t| FormulaTemplateInfo {
            name: t.name.clone(),
            description: t.description.clone(),
            category: format!("{:?}", t.category),
            source: t.source.clone(),
        })
        .collect())
}

#[napi]
#[cfg(feature = "formula")]
pub fn formula_list_categories() -> Result<Vec<String>> {
    use alpha_ta_core::formula::templates::FormulaTemplates;

    let categories = FormulaTemplates::categories();
    Ok(categories.iter().map(|c| format!("{:?}", c)).collect())
}

/// Validate a formula without executing
///
/// Checks if the formula syntax is valid without actually running it.
///
/// @param source - Formula source code to validate
/// @returns `true` if the formula is syntactically valid, `false` otherwise.
#[napi]
#[cfg(feature = "formula")]
pub fn formula_validate(source: String) -> bool {
    parse_formula(&source).is_ok()
}

/// Execute a trading formula with JIT compilation
///
/// Compiles the formula using Just-In-Time compilation for maximum execution speed.
/// This is ideal for formulas that need to be executed repeatedly with different data.
///
/// @param source - Formula source code
/// @param open - Opening prices
/// @param high - High prices
/// @param low - Low prices
/// @param close - Closing prices
/// @param volume - Trading volume
/// @returns Object with output variable names as keys and arrays as values.
///          The special key "__result__" contains the final expression result.
#[napi]
#[cfg(feature = "formula")]
pub fn formula_eval_jit(
    source: String,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
) -> Result<HashMap<String, Vec<f64>>> {
    let open_array = Array1::from_vec(open);
    let high_array = Array1::from_vec(high);
    let low_array = Array1::from_vec(low);
    let close_array = Array1::from_vec(close);
    let volume_array = Array1::from_vec(volume);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        None,
    );
    let mut engine = FormulaEngine::new();

    let result = engine.eval_jit(&source, &mut ctx).map_err(formula_error_to_napi)?;

    let mut output = HashMap::new();

    for (name, value) in ctx.variables {
        output.insert(name.to_string(), value.to_vec());
    }

    output.insert("__result__".to_string(), result.to_vec());

    Ok(output)
}

/// Execute a trading formula with SIMD optimization
///
/// Uses SIMD (Single Instruction Multiple Data) vectorization to accelerate
/// formula execution on supported hardware. Best suited for data-parallel
/// operations on large datasets.
///
/// @param source - Formula source code
/// @param open - Opening prices
/// @param high - High prices
/// @param low - Low prices
/// @param close - Closing prices
/// @param volume - Trading volume
/// @returns Object with output variable names as keys and arrays as values.
///          The special key "__result__" contains the final expression result.
#[napi]
#[cfg(feature = "formula")]
pub fn formula_eval_simd(
    source: String,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
) -> Result<HashMap<String, Vec<f64>>> {
    let open_array = Array1::from_vec(open);
    let high_array = Array1::from_vec(high);
    let low_array = Array1::from_vec(low);
    let close_array = Array1::from_vec(close);
    let volume_array = Array1::from_vec(volume);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        None,
    );
    let mut engine = FormulaEngine::new();

    let result = engine.eval_simd(&source, &mut ctx).map_err(formula_error_to_napi)?;

    let mut output = HashMap::new();

    for (name, value) in ctx.variables {
        output.insert(name.to_string(), value.to_vec());
    }

    output.insert("__result__".to_string(), result.to_vec());

    Ok(output)
}

/// Execute a trading formula with SIMD optimization
///
/// Minimizes memory allocations by operating directly on input buffers
/// without copying data. This provides the lowest latency execution path
/// for latency-sensitive applications.
///
/// @param source - Formula source code
/// @param open - Opening prices
/// @param high - High prices
/// @param low - Low prices
/// @param close - Closing prices
/// @param volume - Trading volume
/// @returns Object with output variable names as keys and arrays as values.
///          The special key "__result__" contains the final expression result.
#[napi]
#[cfg(feature = "formula")]
pub fn formula_eval_zero_copy(
    source: String,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
) -> Result<HashMap<String, Vec<f64>>> {
    let open_array = Array1::from_vec(open);
    let high_array = Array1::from_vec(high);
    let low_array = Array1::from_vec(low);
    let close_array = Array1::from_vec(close);
    let volume_array = Array1::from_vec(volume);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        None,
    );
    let mut engine = FormulaEngine::new();

    let result = engine
        .eval_zero_copy(&source, &mut ctx)
        .map_err(formula_error_to_napi)?;

    let mut output = HashMap::new();

    for (name, value) in ctx.variables {
        output.insert(name.to_string(), value.to_vec());
    }

    output.insert("__result__".to_string(), result.to_vec());

    Ok(output)
}
