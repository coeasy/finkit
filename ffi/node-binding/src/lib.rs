#![allow(
    non_snake_case,
    dead_code,
    missing_docs,
    missing_debug_implementations,
    deprecated
)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::{HashMap, HashSet};

use finkit::calendar::{MarketCalendarPreset, SessionWindow, TimeZoneSpec, TradingCalendar};
use finkit::chan::{ChanConfig, ChanVariant};
use finkit::chan_mtf::ChanMultiConfig;
use finkit::composite::{CompositeDefinition, CompositeEngine, CompositeExpr, CompositeOp};
use finkit::factors::FactorContext;
use finkit::indicators;
use finkit::math::moving_avg;
use finkit::patterns::{candlestick, chart};
use finkit_visualization::interaction::ReplayState;

mod streaming;
mod sweep;
mod transforms;

#[napi(object)]
pub struct CalendarSessionNapi {
    pub session_day: i64,
    pub session_index: u32,
    pub open_timestamp: i64,
    pub close_timestamp: i64,
    pub source: Option<String>,
    pub revision: Option<String>,
}

#[napi(object)]
pub struct CalendarSessionOverrideNapi {
    pub date: String,
    pub sessions: Vec<CalendarSessionWindowNapi>,
}

#[napi(object)]
pub struct CalendarSessionWindowNapi {
    pub open_seconds: u32,
    pub close_seconds: u32,
}

/// One node in a dependency-aware custom composite-indicator graph.
#[napi(object)]
pub struct CompositeDefinitionNapi {
    pub name: String,
    pub function: String,
    pub inputs: Vec<String>,
    pub params: Vec<f64>,
}

/// Resolve an exchange session for a Unix timestamp using a configurable
/// market preset, timezone, holiday list and session overrides.
#[napi]
pub fn resolve_market_session(
    market: String,
    timestamp: i64,
    timezone: Option<String>,
    holidays: Option<Vec<String>>,
    sessions: Option<Vec<CalendarSessionWindowNapi>>,
    special_sessions: Option<Vec<CalendarSessionOverrideNapi>>,
) -> Result<Option<CalendarSessionNapi>> {
    let preset = MarketCalendarPreset::parse(&market)
        .map_err(|error| Error::new(Status::InvalidArg, format!("{error}")))?;
    let mut calendar = TradingCalendar::for_market(preset);
    if let Some(timezone) = timezone {
        calendar = calendar.with_timezone(
            TimeZoneSpec::parse(&timezone)
                .map_err(|error| Error::new(Status::InvalidArg, format!("{error}")))?,
        );
    }
    if let Some(holidays) = holidays {
        for holiday in holidays {
            calendar
                .add_holiday(&holiday)
                .map_err(|error| Error::new(Status::InvalidArg, format!("{error}")))?;
        }
    }
    if let Some(sessions) = sessions {
        let sessions: Vec<SessionWindow> = sessions
            .into_iter()
            .map(|session| SessionWindow::new(session.open_seconds, session.close_seconds))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| Error::new(Status::InvalidArg, format!("{error}")))?;
        calendar = calendar.with_sessions(&sessions);
    }
    if let Some(special_sessions) = special_sessions {
        for override_item in special_sessions {
            let sessions: Vec<SessionWindow> = override_item
                .sessions
                .into_iter()
                .map(|session| SessionWindow::new(session.open_seconds, session.close_seconds))
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|error| Error::new(Status::InvalidArg, format!("{error}")))?;
            calendar
                .set_special_sessions(&override_item.date, &sessions)
                .map_err(|error| Error::new(Status::InvalidArg, format!("{error}")))?;
        }
    }
    let Some(session) = calendar
        .session_for_timestamp_local(timestamp)
        .map_err(|error| Error::new(Status::InvalidArg, format!("{error}")))?
    else {
        return Ok(None);
    };
    Ok(Some(CalendarSessionNapi {
        session_day: session.session_day,
        session_index: session.session_index as u32,
        open_timestamp: session.open_timestamp,
        close_timestamp: session.close_timestamp,
        source: calendar.source().map(str::to_string),
        revision: calendar.revision().map(str::to_string),
    }))
}

/// Resolve a session from a versioned JSON calendar definition.
#[napi]
pub fn resolve_market_session_config(
    config_json: String,
    timestamp: i64,
) -> Result<Option<CalendarSessionNapi>> {
    let calendar = TradingCalendar::from_json(&config_json)
        .map_err(|error| Error::new(Status::InvalidArg, format!("{error}")))?;
    let Some(session) = calendar
        .session_for_timestamp_local(timestamp)
        .map_err(|error| Error::new(Status::InvalidArg, format!("{error}")))?
    else {
        return Ok(None);
    };
    Ok(Some(CalendarSessionNapi {
        session_day: session.session_day,
        session_index: session.session_index as u32,
        open_timestamp: session.open_timestamp,
        close_timestamp: session.close_timestamp,
        source: calendar.source().map(str::to_string),
        revision: calendar.revision().map(str::to_string),
    }))
}

/// Resolve a session from an exchange-published annual CSV calendar.
#[napi]
pub fn resolve_market_session_csv(
    csv: String,
    market: String,
    timestamp: i64,
    timezone: Option<String>,
) -> Result<Option<CalendarSessionNapi>> {
    let calendar = TradingCalendar::from_csv(&csv, Some(&market), timezone.as_deref())
        .map_err(|error| Error::new(Status::InvalidArg, format!("{error}")))?;
    let Some(session) = calendar
        .session_for_timestamp_local(timestamp)
        .map_err(|error| Error::new(Status::InvalidArg, format!("{error}")))?
    else {
        return Ok(None);
    };
    Ok(Some(CalendarSessionNapi {
        session_day: session.session_day,
        session_index: session.session_index as u32,
        open_timestamp: session.open_timestamp,
        close_timestamp: session.close_timestamp,
        source: calendar.source().map(str::to_string),
        revision: calendar.revision().map(str::to_string),
    }))
}

/// Evaluate a dependency-aware graph of custom composite indicators.
///
/// Inputs may reference `open`, `high`, `low`, `close`, `volume`, another
/// definition name, or `const:<number>`. Built-in functions include SMA, EMA,
/// RSI, ATR, MACD, Bollinger bands, VWMA, returns, z-score, rolling statistics,
/// threshold/clip predicates and cross signals; element-wise arithmetic
/// functions are also accepted.
#[napi]
pub fn compute_composite(
    close: Vec<f64>,
    definitions: Vec<CompositeDefinitionNapi>,
    outputs: Option<Vec<String>>,
    open: Option<Vec<f64>>,
    high: Option<Vec<f64>>,
    low: Option<Vec<f64>>,
    volume: Option<Vec<f64>>,
) -> Result<HashMap<String, Vec<f64>>> {
    let names: HashSet<String> = definitions.iter().map(|item| item.name.clone()).collect();
    if names.len() != definitions.len() {
        return Err(Error::new(
            Status::InvalidArg,
            "composite definition names must be unique",
        ));
    }
    let definitions = definitions
        .into_iter()
        .map(|item| {
            let inputs = item
                .inputs
                .into_iter()
                .map(|input| composite_input_expression_napi(&input, &names))
                .collect::<Result<Vec<_>>>()?;
            let expression = match item.function.to_ascii_lowercase().as_str() {
                "add" => CompositeExpr::Op {
                    op: CompositeOp::Add,
                    inputs,
                },
                "sub" => CompositeExpr::Op {
                    op: CompositeOp::Sub,
                    inputs,
                },
                "mul" => CompositeExpr::Op {
                    op: CompositeOp::Mul,
                    inputs,
                },
                "div" => CompositeExpr::Op {
                    op: CompositeOp::Div,
                    inputs,
                },
                "min" => CompositeExpr::Op {
                    op: CompositeOp::Min,
                    inputs,
                },
                "max" => CompositeExpr::Op {
                    op: CompositeOp::Max,
                    inputs,
                },
                "weighted_average" | "weightedaverage" => {
                    CompositeExpr::call("weighted_average", inputs, item.params)
                }
                _ => CompositeExpr::call(item.function, inputs, item.params),
            };
            Ok(CompositeDefinition::new(item.name, expression))
        })
        .collect::<Result<Vec<_>>>()?;
    let output_names = outputs.unwrap_or_else(|| {
        definitions
            .iter()
            .map(|definition| definition.name.clone())
            .collect()
    });
    let output_refs: Vec<&str> = output_names.iter().map(String::as_str).collect();
    let mut context = FactorContext::new();
    context
        .insert("close", close)
        .map_err(|error| Error::new(Status::InvalidArg, error.to_string()))?;
    for (name, values) in [
        ("open", open),
        ("high", high),
        ("low", low),
        ("volume", volume),
    ] {
        if let Some(values) = values {
            context
                .insert(name, values)
                .map_err(|error| Error::new(Status::InvalidArg, error.to_string()))?;
        }
    }
    CompositeEngine::new()
        .evaluate(&definitions, &output_refs, &context)
        .map(|values| values.into_iter().collect())
        .map_err(|error| Error::new(Status::InvalidArg, error.to_string()))
}

fn composite_input_expression_napi(
    input: &str,
    definition_names: &HashSet<String>,
) -> Result<CompositeExpr> {
    if let Some(value) = input.strip_prefix("const:") {
        let value = value.parse::<f64>().map_err(|_| {
            Error::new(
                Status::InvalidArg,
                format!("invalid composite constant: {input}"),
            )
        })?;
        return Ok(CompositeExpr::Constant(value));
    }
    if definition_names.contains(input) {
        Ok(CompositeExpr::reference(input))
    } else {
        Ok(CompositeExpr::series(input))
    }
}

#[cfg(feature = "formula")]
use finkit::formula::{parse_formula, FormulaContext, FormulaEngine, FormulaError};
#[cfg(feature = "formula")]
use ndarray::Array1;

#[cfg(feature = "formula")]
fn formula_error_to_napi(e: FormulaError) -> napi::Error {
    match e {
        FormulaError::ParseError(msg) => {
            napi::Error::new(napi::Status::InvalidArg, format!("Parse error: {}", msg))
        }
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
        FormulaError::RuntimeError(msg) => napi::Error::new(
            napi::Status::GenericFailure,
            format!("Runtime error: {}", msg),
        ),
        FormulaError::InvalidParameter(msg) => napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid parameter: {}", msg),
        ),
        FormulaError::InsufficientData(msg) => napi::Error::new(
            napi::Status::InvalidArg,
            format!("Insufficient data: {}", msg),
        ),
        FormulaError::InvalidOperation(msg) => napi::Error::new(
            napi::Status::InvalidArg,
            format!("Invalid operation: {}", msg),
        ),
        FormulaError::UnsupportedFunction(msg) => napi::Error::new(
            napi::Status::InvalidArg,
            format!("Unsupported function: {}", msg),
        ),
        FormulaError::Timeout { elapsed_ms } => napi::Error::new(
            napi::Status::GenericFailure,
            format!("Execution timeout after {}ms", elapsed_ms),
        ),
        FormulaError::MemoryLimit { used, limit } => napi::Error::new(
            napi::Status::GenericFailure,
            format!(
                "Memory limit exceeded: used {} bytes, limit {} bytes",
                used, limit
            ),
        ),
    }
}

use finkit::error::TaError;

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
    pub timestamps: Option<Vec<i64>>,
    pub opens: Vec<f64>,
    pub highs: Vec<f64>,
    pub lows: Vec<f64>,
    pub closes: Vec<f64>,
    pub volumes: Vec<f64>,
}

#[napi(object)]
pub struct KlineQuoteNapi {
    pub date: String,
    pub timestamp: Option<i64>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl From<KlineDataNapi> for finkit_visualization::data::KlineData {
    fn from(data: KlineDataNapi) -> Self {
        let mut result = Self::new(
            data.dates,
            data.opens,
            data.highs,
            data.lows,
            data.closes,
            data.volumes,
        );
        if let Some(timestamps) = data.timestamps {
            result.timestamps = timestamps;
        }
        result
    }
}

impl From<finkit_visualization::data::KlineData> for KlineDataNapi {
    fn from(data: finkit_visualization::data::KlineData) -> Self {
        Self {
            dates: data.dates,
            timestamps: (!data.timestamps.is_empty()).then_some(data.timestamps),
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
    timestamps: Option<Vec<i64>>,
) -> KlineDataNapi {
    KlineDataNapi {
        dates,
        timestamps,
        opens,
        highs,
        lows,
        closes,
        volumes,
    }
}

#[napi]
pub fn kline_data_validate(data: KlineDataNapi) -> bool {
    let inner: finkit_visualization::data::KlineData = data.into();
    inner.validate()
}

#[napi]
pub fn kline_data_validate_ohlcv(data: KlineDataNapi) -> bool {
    let inner: finkit_visualization::data::KlineData = data.into();
    inner.validate_ohlcv()
}

#[napi]
pub fn kline_data_validation_errors(data: KlineDataNapi) -> Vec<String> {
    let inner: finkit_visualization::data::KlineData = data.into();
    inner.validation_errors()
}

#[napi]
pub struct KlineChartNapi {
    inner: finkit_visualization::chart::KlineChart,
    data: Option<finkit_visualization::data::KlineData>,
    indicators: Vec<finkit_visualization::config::IndicatorConfig>,
    replay: ReplayState,
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
        let inner_data: finkit_visualization::data::KlineData = data.into();
        let lang = match language.as_str() {
            "zh-CN" | "zh" => finkit_visualization::language::Language::ZhCn,
            _ => finkit_visualization::language::Language::EnUs,
        };
        let config = finkit_visualization::config::ChartConfigBuilder::new()
            .with_title(&title)
            .with_language(lang)
            .with_dimensions(width, height)
            .build();
        let mut chart = finkit_visualization::chart::KlineChart::new(config);
        chart.set_data(inner_data.clone());
        chart
            .build_draw_list(&inner_data, &[])
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;
        let data_len = inner_data.len();
        Ok(Self {
            inner: chart,
            data: Some(inner_data),
            indicators: Vec::new(),
            replay: ReplayState::new(data_len, 200),
        })
    }

    #[napi]
    pub fn add_ma(&mut self, periods: Vec<u32>) {
        self.indicators
            .push(finkit_visualization::config::IndicatorConfig::new(
                finkit_visualization::config::IndicatorType::MA,
                periods.into_iter().map(|value| value as f64).collect(),
            ));
        let _ = self.rebuild();
    }

    #[napi]
    pub fn add_macd(&mut self, fast: u32, slow: u32, signal: u32) {
        self.indicators
            .push(finkit_visualization::config::IndicatorConfig::new(
                finkit_visualization::config::IndicatorType::MACD,
                vec![fast as f64, slow as f64, signal as f64],
            ));
        let _ = self.rebuild();
    }

    #[napi]
    pub fn add_rsi(&mut self, period: u32) {
        self.indicators
            .push(finkit_visualization::config::IndicatorConfig::new(
                finkit_visualization::config::IndicatorType::RSI,
                vec![period as f64],
            ));
        let _ = self.rebuild();
    }

    #[napi]
    pub fn add_boll(&mut self, period: u32, nb_dev: f64) {
        self.indicators
            .push(finkit_visualization::config::IndicatorConfig::new(
                finkit_visualization::config::IndicatorType::BOLL,
                vec![period as f64, nb_dev],
            ));
        let _ = self.rebuild();
    }

    #[napi]
    pub fn add_custom_indicator(&mut self, name: String, values: Vec<f64>) -> Result<()> {
        self.set_custom_indicator(name, values)
    }

    /// Replaces or registers a custom indicator series without duplicating its
    /// chart definition. This is intended for real-time recalculation after a
    /// new bar is appended or the current bar is revised.
    #[napi]
    pub fn set_custom_indicator(&mut self, name: String, values: Vec<f64>) -> Result<()> {
        let data_len = self.data.as_ref().map(|data| data.len()).unwrap_or(0);
        if name.trim().is_empty() || values.len() != data_len {
            return Err(Error::new(
                Status::InvalidArg,
                format!(
                    "custom indicator '{}' has {} values, expected {}",
                    name,
                    values.len(),
                    data_len
                ),
            ));
        }
        self.inner
            .set_custom_indicator_series(name.clone(), values)
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))?;
        if !self.indicators.iter().any(|indicator| {
            matches!(
                &indicator.indicator_type,
                finkit_visualization::config::IndicatorType::Custom(existing) if existing == &name
            )
        }) {
            self.indicators
                .push(finkit_visualization::config::IndicatorConfig::new(
                    finkit_visualization::config::IndicatorType::Custom(name),
                    vec![],
                ));
        }
        self.rebuild()?;
        Ok(())
    }

    #[napi]
    pub fn add_event_marker(
        &mut self,
        index: u32,
        label: String,
        value: Option<f64>,
        color: Option<String>,
    ) -> Result<()> {
        let mut marker = finkit_visualization::chart::EventMarker::new(index as usize, label)
            .with_color(finkit_visualization::primitive::Color::from_hex(
                color.as_deref().unwrap_or("#f59e0b"),
            ));
        if let Some(value) = value {
            marker = marker.with_value(value);
        }
        self.inner.add_event_marker(marker);
        self.inner
            .render_incremental()
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))?;
        Ok(())
    }

    #[napi]
    pub fn set_viewport(
        &mut self,
        start: u32,
        end: u32,
        pixel_width: u32,
        pixel_height: u32,
        overscan_bars: u32,
        follow_latest: bool,
    ) -> Result<()> {
        let viewport = if end == 0 {
            finkit_visualization::viewport::Viewport::full()
        } else {
            finkit_visualization::viewport::Viewport::new(start as usize, end as usize)
                .with_pixels(pixel_width, pixel_height)
                .with_overscan(overscan_bars as usize)
                .with_follow_latest(follow_latest)
        };
        self.inner.set_viewport(viewport);
        self.rebuild()
    }

    #[napi]
    pub fn set_lod_policy(&mut self, level: String) -> Result<()> {
        let policy = match level.to_ascii_lowercase().as_str() {
            "auto" => finkit_visualization::viewport::LodPolicy::Auto,
            "raw" => finkit_visualization::viewport::LodPolicy::Fixed(
                finkit_visualization::viewport::LodLevel::Raw,
            ),
            "balanced" => finkit_visualization::viewport::LodPolicy::Fixed(
                finkit_visualization::viewport::LodLevel::Balanced,
            ),
            "overview" => finkit_visualization::viewport::LodPolicy::Fixed(
                finkit_visualization::viewport::LodLevel::Overview,
            ),
            value => {
                return Err(Error::new(
                    Status::InvalidArg,
                    format!("unknown LOD policy: {value}"),
                ))
            }
        };
        self.inner.set_lod_policy(policy);
        self.rebuild()
    }

    #[napi]
    pub fn set_layer_visible(&mut self, layer: String, visible: bool) -> Result<bool> {
        let changed = self.inner.set_layer_visible(&layer, visible);
        self.rebuild()?;
        Ok(changed)
    }

    #[napi]
    pub fn set_interaction(
        &mut self,
        enabled: bool,
        show_crosshair: bool,
        show_data_window: bool,
        enable_pan_zoom: bool,
        enable_keyboard: bool,
    ) {
        self.inner
            .set_interaction_config(finkit_visualization::config::InteractionConfig {
                enabled,
                show_crosshair,
                show_data_window,
                enable_pan_zoom,
                enable_keyboard,
            });
    }

    #[napi]
    pub fn set_replay_window(&mut self, window: u32, cursor: Option<u32>) -> Result<Vec<u32>> {
        let total = self.data.as_ref().map(|data| data.len()).unwrap_or(0);
        self.replay.window = window.max(1) as usize;
        if let Some(cursor) = cursor {
            self.replay.seek(cursor as usize, total);
        } else {
            self.replay.reset(total);
        }
        let (start, end) = self.replay.visible_range(total);
        self.inner
            .set_viewport(finkit_visualization::viewport::Viewport::new(start, end));
        self.rebuild()?;
        Ok(vec![start as u32, end as u32])
    }

    #[napi]
    pub fn replay_next(&mut self) -> Result<Option<Vec<u32>>> {
        let total = self.data.as_ref().map(|data| data.len()).unwrap_or(0);
        if self.replay.advance(total).is_none() {
            return Ok(None);
        }
        let (start, end) = self.replay.visible_range(total);
        self.inner
            .set_viewport(finkit_visualization::viewport::Viewport::new(start, end));
        self.rebuild()?;
        Ok(Some(vec![start as u32, end as u32]))
    }

    #[napi]
    pub fn add_chan(
        &mut self,
        min_stroke_bars: u32,
        show_labels: bool,
        variant: String,
        stroke_policy: String,
        center_policy: String,
        signal_min_strength: f64,
        show_multi_timeframe_annotations: bool,
    ) -> Result<()> {
        let variant_name = variant.to_ascii_lowercase();
        let chan_variant = match variant_name.as_str() {
            "conservative" => ChanVariant::Conservative,
            "aggressive" => ChanVariant::Aggressive,
            _ => ChanVariant::Standard,
        };
        let mut render_config = self.inner.config().chan.clone();
        render_config.enabled = true;
        render_config.min_stroke_bars = min_stroke_bars.max(1) as usize;
        render_config.show_labels = show_labels;
        render_config.variant = variant;
        render_config.stroke_policy = stroke_policy;
        render_config.center_policy = center_policy;
        render_config.signal_min_strength = signal_min_strength;
        render_config.show_multi_timeframe_annotations = show_multi_timeframe_annotations;
        self.inner.set_chan_render_config(render_config);
        self.inner
            .analyze_and_set_chan(
                ChanConfig {
                    min_stroke_bars: min_stroke_bars.max(1) as usize,
                    ..ChanConfig::default()
                }
                .with_variant(chan_variant),
            )
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))?;
        self.rebuild()
    }

    #[napi]
    pub fn add_chan_multi(&mut self, factors: Vec<u32>, variant: String) -> Result<()> {
        let variant_name = variant.to_ascii_lowercase();
        let chan_variant = match variant_name.as_str() {
            "conservative" => ChanVariant::Conservative,
            "aggressive" => ChanVariant::Aggressive,
            _ => ChanVariant::Standard,
        };
        let mut render_config = self.inner.config().chan.clone();
        render_config.enabled = true;
        render_config.variant = variant;
        render_config.show_multi_timeframe_annotations = true;
        self.inner.set_chan_render_config(render_config);
        self.inner
            .analyze_and_set_chan_multi(ChanMultiConfig {
                factors: factors.into_iter().map(|factor| factor as usize).collect(),
                chan: ChanConfig::default().with_variant(chan_variant),
                ..ChanMultiConfig::default()
            })
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))?;
        self.rebuild()
    }

    /// Update Chan signal/structure thresholds and reanalyze the active chart.
    #[napi]
    pub fn set_chan_thresholds(
        &mut self,
        min_stroke_change_ratio: f64,
        min_fractal_range_ratio: f64,
        signal_min_strength: f64,
        center_break_ratio: f64,
    ) -> Result<()> {
        let mut render_config = self.inner.config().chan.clone();
        render_config.min_stroke_change_ratio = min_stroke_change_ratio;
        render_config.min_fractal_range_ratio = min_fractal_range_ratio;
        render_config.signal_min_strength = signal_min_strength;
        render_config.center_break_ratio = center_break_ratio;
        self.inner.set_chan_render_config(render_config);
        if self.inner.config().chan.enabled {
            let data = self
                .inner
                .data()
                .cloned()
                .ok_or_else(|| Error::new(Status::InvalidArg, "chart has no data"))?;
            let analysis =
                finkit_visualization::chart::chan::analyze_configured(&data, self.inner.config())
                    .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))?;
            self.inner.set_chan_analysis(analysis);
        }
        self.rebuild()
    }

    #[napi]
    pub fn append_kline(
        &mut self,
        date: String,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Result<()> {
        self.inner
            .append_kline(&date, open, high, low, close, volume)
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))?;
        self.data = self.inner.data().cloned();
        self.rebuild()
    }

    #[napi]
    pub fn update_last_kline(
        &mut self,
        close: f64,
        high: Option<f64>,
        low: Option<f64>,
        volume: Option<f64>,
    ) -> Result<()> {
        self.inner
            .update_last_kline(close, high, low, volume)
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))?;
        self.data = self.inner.data().cloned();
        self.rebuild()
    }

    #[napi]
    pub fn upsert_kline(
        &mut self,
        date: String,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        timestamp: Option<i64>,
    ) -> Result<String> {
        let update = match timestamp {
            Some(timestamp) => self
                .inner
                .upsert_kline_with_timestamp(timestamp, &date, open, high, low, close, volume),
            None => self
                .inner
                .upsert_kline(&date, open, high, low, close, volume),
        }
        .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))?;
        self.data = self.inner.data().cloned();
        self.rebuild()?;
        Ok(match update.kind {
            finkit_visualization::chart::KlineUpdateKind::Appended => "appended",
            finkit_visualization::chart::KlineUpdateKind::Updated => "updated",
        }
        .to_string())
    }

    /// Apply many live quotes and rebuild the chart once at the end.
    #[napi]
    pub fn upsert_klines(&mut self, updates: Vec<KlineQuoteNapi>) -> Result<Vec<String>> {
        let bars: Vec<finkit_visualization::data::KlineBar> = updates
            .into_iter()
            .map(|update| {
                let bar = finkit_visualization::data::KlineBar::new(
                    update.date,
                    update.open,
                    update.high,
                    update.low,
                    update.close,
                    update.volume,
                );
                update
                    .timestamp
                    .map_or(bar.clone(), |timestamp| bar.with_timestamp(timestamp))
            })
            .collect();
        let result = self
            .inner
            .upsert_klines(&bars)
            .map_err(|e| Error::new(Status::InvalidArg, format!("{}", e)))?;
        self.data = self.inner.data().cloned();
        self.rebuild()?;
        Ok(result
            .into_iter()
            .map(|update| match update.kind {
                finkit_visualization::chart::KlineUpdateKind::Appended => "appended",
                finkit_visualization::chart::KlineUpdateKind::Updated => "updated",
            })
            .map(str::to_string)
            .collect())
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

    #[napi]
    pub fn save_as_canvas_html(&mut self, path: String) -> Result<()> {
        self.inner
            .save_as_canvas_html(&path)
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))
    }

    #[napi]
    pub fn to_canvas_html(&mut self) -> Result<String> {
        self.inner
            .to_canvas_html_string()
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))
    }

    #[napi]
    pub fn save_as_webgl_html(&mut self, path: String) -> Result<()> {
        self.inner
            .save_as_webgl_html(&path)
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))
    }

    #[napi]
    pub fn save_as_webgpu_html(&mut self, path: String) -> Result<()> {
        self.inner
            .save_as_webgpu_html(&path)
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))
    }

    #[napi]
    pub fn to_webgl_html(&mut self) -> Result<String> {
        self.inner
            .to_webgl_html_string()
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))
    }

    /// Return an explicitly WebGPU-preferred HTML document with WebGL2/Canvas fallback.
    #[napi]
    pub fn to_webgpu_html(&mut self) -> Result<String> {
        self.inner
            .to_webgpu_html_string()
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))
    }

    #[napi]
    pub fn save_as_html(&mut self, path: String) -> Result<()> {
        self.inner
            .save_as_html(&path)
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))
    }

    #[napi]
    pub fn to_html(&mut self) -> Result<String> {
        self.inner
            .to_html_string()
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))
    }

    #[napi]
    pub fn to_json(&mut self) -> Result<String> {
        self.inner
            .to_json_string()
            .map_err(|e| Error::new(Status::GenericFailure, format!("{}", e)))
    }
}

impl KlineChartNapi {
    fn rebuild(&mut self) -> Result<()> {
        let Some(data) = self.data.clone() else {
            return Err(Error::new(Status::InvalidArg, "chart has no data"));
        };
        self.inner
            .build_draw_list(&data, &self.indicators)
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

    let result = engine
        .eval(&source, &mut ctx)
        .map_err(formula_error_to_napi)?;

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

    let multi = engine
        .eval_multi(&source, &mut ctx)
        .map_err(formula_error_to_napi)?;

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

    let _result = engine
        .eval(&source, &mut ctx)
        .map_err(formula_error_to_napi)?;

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

    let (_result, debugger) = engine
        .eval_with_debug(&source, &mut ctx)
        .map_err(formula_error_to_napi)?;

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
    use finkit::formula::templates::FormulaTemplates;

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

    let result = engine
        .eval_jit(&source, &mut ctx)
        .map_err(formula_error_to_napi)?;

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

    let result = engine
        .eval_simd(&source, &mut ctx)
        .map_err(formula_error_to_napi)?;

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
