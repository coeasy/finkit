#![allow(missing_docs)]
#![allow(missing_debug_implementations)]
// `into_raw_vec` is deprecated in ndarray 0.16 in favour of
// `into_raw_vec_and_offset`, but the offset is always 0 for 1D arrays and
// rewriting the 100+ call sites in the FFI surface is noisy. Suppress here
// so the deprecation churn stays contained to the `core` crate.
#![allow(deprecated)]

use ::finkit::calendar::{MarketCalendarPreset, SessionWindow, TimeZoneSpec, TradingCalendar};

use ::finkit::chan::{
    analyze as analyze_chan, CenterPolicy, ChanConfig, ChanVariant, FractalKind, FractalPolicy,
    StrokePolicy,
};

use ::finkit::chan_mtf::{
    analyze_multi as analyze_chan_multi,
    analyze_multi_timestamps_with_calendar as analyze_chan_multi_timestamps_calendar,
    analyze_multi_timestamps_with_origin as analyze_chan_multi_timestamps, ChanMultiConfig,
};

use ::finkit::composite::{CompositeDefinition, CompositeEngine, CompositeExpr, CompositeOp};

use ::finkit::factors::FactorContext;

use ::finkit::indicators;

use ::finkit::indicators::PivotMethod;

use ::finkit::math::moving_avg;

use ::finkit::patterns::{candlestick, chart};

use finkit_visualization::chart::EventMarker;

use finkit_visualization::config::{
    ChartConfig, ChartConfigBuilder, IndicatorConfig, IndicatorType,
};

use finkit_visualization::data::KlineData;

use finkit_visualization::error::VisualizationError;

use finkit_visualization::interaction::ReplayState;

use finkit_visualization::language::Language;

use finkit_visualization::primitive::Color;

use finkit_visualization::viewport::{LodLevel, LodPolicy, Viewport};

#[cfg(feature = "formula")]
use formula_plan::{PyCompiledFormula, PyFormulaRegistry};

use numpy::{PyArray1, PyReadonlyArray1};

use pyo3::marker::Ungil;

use pyo3::prelude::*;

mod compat_api;

mod factor_library;

mod features;

#[cfg(feature = "formula")]
mod formula_plan;

mod research_api;

mod streaming;

mod sweep;

mod transforms;

#[cfg(feature = "formula")]
use ::finkit::formula::{
    parse_formula, FormulaContext, FormulaDialect, FormulaEngine, FormulaError,
};

#[cfg(feature = "formula")]
use ndarray::Array1;

include!("generated.rs");

// Buckets split out of this crate root; see each module's header.
mod cdl_patterns;
mod chan;
mod chart_patterns;
mod charts;
mod composite;
mod conversion;
mod dmi_extra;
#[cfg(feature = "formula")]
mod formula_contract;
#[cfg(feature = "formula")]
mod formula_eval_api;
mod json_api;
mod market_session;
mod price_extra;
mod talib_compat;

// One facade per bucket: `#[pymodule]` below registers these by name.
pub use cdl_patterns::*;
pub use chan::*;
pub use chart_patterns::*;
pub use charts::*;
pub use composite::*;
pub use conversion::*;
pub use dmi_extra::*;
#[cfg(feature = "formula")]
pub use formula_contract::*;
#[cfg(feature = "formula")]
pub use formula_eval_api::*;
pub use json_api::*;
pub use market_session::*;
pub use price_extra::*;
pub use talib_compat::*;
fn compute_single_indicator(
    open: Option<&[f64]>,
    high: Option<&[f64]>,
    low: Option<&[f64]>,
    close: &[f64],
    volume: Option<&[f64]>,
    secondary: Option<&[f64]>,
    req: &IndicatorRequest,
) -> IndicatorResult {
    let name = req.name.to_lowercase();
    let params = &req.params;

    match name.as_str() {
        "sma" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            moving_avg::sma(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "ema" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            moving_avg::ema(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "wma" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            moving_avg::wma(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "dema" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            moving_avg::dema(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "tema" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            moving_avg::tema(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "kama" => {
            let period = params.first().copied().unwrap_or(10.0) as usize;
            let fast = params.get(1).copied().unwrap_or(2.0) as usize;
            let slow = params.get(2).copied().unwrap_or(30.0) as usize;
            moving_avg::kama(close, period, fast, slow)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "t3" => {
            let period = params.first().copied().unwrap_or(5.0) as usize;
            let vfactor = params.get(1).copied().unwrap_or(0.7);
            indicators::t3(close, period, vfactor)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "sarext" => match (high, low) {
            (Some(h), Some(l)) => {
                let start_value = params.first().copied().unwrap_or(0.0);
                let offset_on_reverse = params.get(1).copied().unwrap_or(0.0);
                let af_init_long = params.get(2).copied().unwrap_or(0.02);
                let af_long = params.get(3).copied().unwrap_or(0.02);
                let af_max_long = params.get(4).copied().unwrap_or(0.2);
                let af_init_short = params.get(5).copied().unwrap_or(0.02);
                let af_short = params.get(6).copied().unwrap_or(0.02);
                let af_max_short = params.get(7).copied().unwrap_or(0.2);
                indicators::sarext(
                    h,
                    l,
                    start_value,
                    offset_on_reverse,
                    af_init_long,
                    af_long,
                    af_max_long,
                    af_init_short,
                    af_short,
                    af_max_short,
                )
                .map(|res| IndicatorResult::Single(res.sar.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("SAREXT requires high and low data".to_string()),
        },
        "accbands" => {
            let period = params.first().copied().unwrap_or(20.0) as usize;
            match (high, low) {
                (Some(h), Some(l)) => indicators::accbands(h, l, close, period)
                    .map(|res| {
                        IndicatorResult::Triple(
                            res.upper.into_raw_vec(),
                            res.middle.into_raw_vec(),
                            res.lower.into_raw_vec(),
                        )
                    })
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
                _ => IndicatorResult::Error("ACCBANDS requires high and low data".to_string()),
            }
        }
        "imi" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            match open {
                Some(o) => indicators::imi(o, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
                None => IndicatorResult::Error("IMI requires open data".to_string()),
            }
        }
        "nvi" => match volume {
            Some(v) => indicators::nvi(close, v)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            None => IndicatorResult::Error("NVI requires volume data".to_string()),
        },
        "pvi" => match volume {
            Some(v) => indicators::pvi(close, v)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            None => IndicatorResult::Error("PVI requires volume data".to_string()),
        },
        "rsi" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::rsi(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "mom" => {
            let period = params.first().copied().unwrap_or(10.0) as usize;
            indicators::mom(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "roc" => {
            let period = params.first().copied().unwrap_or(10.0) as usize;
            indicators::roc(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "cmo" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::cmo(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "trix" => {
            let period = params.first().copied().unwrap_or(30.0) as usize;
            indicators::trix(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "apo" => {
            let fast = params.first().copied().unwrap_or(12.0) as usize;
            let slow = params.get(1).copied().unwrap_or(26.0) as usize;
            indicators::apo(close, fast, slow)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "macd" => {
            let fast = params.first().copied().unwrap_or(12.0) as usize;
            let slow = params.get(1).copied().unwrap_or(26.0) as usize;
            let signal = params.get(2).copied().unwrap_or(9.0) as usize;
            let mut macd_line = vec![0.0; close.len()];
            let mut signal_line = vec![0.0; close.len()];
            let mut histogram = vec![0.0; close.len()];
            indicators::macd_fast_into(
                close,
                fast,
                slow,
                signal,
                &mut macd_line,
                &mut signal_line,
                &mut histogram,
            )
            .map(|()| IndicatorResult::Triple(macd_line, signal_line, histogram))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "bollinger_bands" | "bbands" => {
            let period = params.first().copied().unwrap_or(5.0) as usize;
            let nbdevup = params.get(1).copied().unwrap_or(2.0);
            let nbdevdn = params.get(2).copied().unwrap_or(2.0);
            indicators::bbands(close, period, nbdevup, nbdevdn)
                .map(|res| {
                    IndicatorResult::Triple(
                        res.upper.into_raw_vec(),
                        res.middle.into_raw_vec(),
                        res.lower.into_raw_vec(),
                    )
                })
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "midpoint" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::midpoint(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "ht_dcperiod" => indicators::ht_dcperiod(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "ht_dcphase" => indicators::ht_dcphase(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "ht_phasor" => indicators::ht_phasor(close)
            .map(|res| IndicatorResult::Double(res.0.into_raw_vec(), res.1.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "ht_sine" => indicators::ht_sine(close)
            .map(|res| IndicatorResult::Double(res.0.into_raw_vec(), res.1.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "ht_trendmode" => indicators::ht_trendmode(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "ht_trendline" => indicators::ht_trendline(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),

        "ma" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::ma(close, period, indicators::MaType::Sma)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "trima" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            moving_avg::trima(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "mavp" => match secondary {
            Some(periods) => {
                let min_period = params.first().copied().unwrap_or(2.0) as usize;
                let max_period = params.get(1).copied().unwrap_or(30.0) as usize;
                moving_avg::mavp(close, periods, min_period, max_period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            None => IndicatorResult::Error(
                "MAVP requires a periods array as secondary data".to_string(),
            ),
        },
        "macdext" => {
            let fast = params.first().copied().unwrap_or(12.0) as usize;
            let slow = params.get(2).copied().unwrap_or(26.0) as usize;
            let signal = params.get(4).copied().unwrap_or(9.0) as usize;
            indicators::macdext(
                close,
                fast,
                indicators::MaType::Sma,
                slow,
                indicators::MaType::Sma,
                signal,
                indicators::MaType::Sma,
            )
            .map(|res| {
                IndicatorResult::Triple(
                    res.macd.into_raw_vec(),
                    res.signal.into_raw_vec(),
                    res.hist.into_raw_vec(),
                )
            })
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "macdfix" => indicators::macdfix(close)
            .map(|res| {
                IndicatorResult::Triple(
                    res.macd.into_raw_vec(),
                    res.signal.into_raw_vec(),
                    res.hist.into_raw_vec(),
                )
            })
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "adxr" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::adxr(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("ADXR requires high and low data".to_string()),
        },
        "aroonosc" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::aroonosc(h, l, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("AroonOsc requires high and low data".to_string()),
        },
        "ppo" => {
            let fast = params.first().copied().unwrap_or(12.0) as usize;
            let slow = params.get(1).copied().unwrap_or(26.0) as usize;
            indicators::ppo(close, fast, slow)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "rocp" => {
            let period = params.first().copied().unwrap_or(10.0) as usize;
            indicators::rocp(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "rocr" => {
            let period = params.first().copied().unwrap_or(10.0) as usize;
            indicators::rocr(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "rocr100" => {
            let period = params.first().copied().unwrap_or(10.0) as usize;
            indicators::rocr100(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "stochf" => match (high, low) {
            (Some(h), Some(l)) => {
                let fastk = params.first().copied().unwrap_or(5.0) as usize;
                let fastd = params.get(1).copied().unwrap_or(3.0) as usize;
                indicators::stochf(h, l, close, fastk, fastd)
                    .map(|res| IndicatorResult::Double(res.k.into_raw_vec(), res.d.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("STOCHF requires high and low data".to_string()),
        },
        "stochrsi" => {
            let rsi_period = params.first().copied().unwrap_or(14.0) as usize;
            let stoch_period = params.get(1).copied().unwrap_or(5.0) as usize;
            let fastk = params.get(2).copied().unwrap_or(3.0) as usize;
            let fastd = params.get(3).copied().unwrap_or(0.0) as usize;
            indicators::stochrsi(close, rsi_period, stoch_period, fastk, fastd)
                .map(|res| IndicatorResult::Double(res.k.into_raw_vec(), res.d.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "ultosc" => match (high, low) {
            (Some(h), Some(l)) => {
                let p1 = params.first().copied().unwrap_or(7.0) as usize;
                let p2 = params.get(1).copied().unwrap_or(14.0) as usize;
                let p3 = params.get(2).copied().unwrap_or(28.0) as usize;
                indicators::ultosc(h, l, close, p1, p2, p3)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("ULTOSC requires high and low data".to_string()),
        },
        "avgdev" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::avgdev(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "linearreg_angle" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::linearreg_angle(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "linearreg_intercept" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::linearreg_intercept(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "linearreg_slope" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::linearreg_slope(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "var" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::var(close, period, 1.0)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "beta" => match secondary {
            Some(other) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::beta(close, other, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            None => IndicatorResult::Error("BETA requires secondary data".to_string()),
        },
        "correl" | "correlation" => match secondary {
            Some(other) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::correlation(close, other, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            None => IndicatorResult::Error("CORREL requires secondary data".to_string()),
        },
        "acos" => indicators::acos(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "asin" => indicators::asin(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "atan" => indicators::atan(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "ceil" => indicators::ceil(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "cos" => indicators::cos(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "cosh" => indicators::cosh(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "exp" => indicators::exp(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "floor" => indicators::floor(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "ln" => indicators::ln(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "log10" => indicators::log10(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "sin" => indicators::sin(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "sinh" => indicators::sinh(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "sqrt" => indicators::sqrt(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "tan" => indicators::tan(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "tanh" => indicators::tanh(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "add" => match secondary {
            Some(other) => indicators::add(close, other)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            None => IndicatorResult::Error("ADD requires secondary data".to_string()),
        },
        "div" => match secondary {
            Some(other) => indicators::div(close, other)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            None => IndicatorResult::Error("DIV requires secondary data".to_string()),
        },
        "mult" => match secondary {
            Some(other) => indicators::mult(close, other)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            None => IndicatorResult::Error("MULT requires secondary data".to_string()),
        },
        "sub" => match secondary {
            Some(other) => indicators::sub(close, other)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            None => IndicatorResult::Error("SUB requires secondary data".to_string()),
        },
        "max" => {
            let period = params.first().copied().unwrap_or(30.0) as usize;
            indicators::max(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "min" => {
            let period = params.first().copied().unwrap_or(30.0) as usize;
            indicators::min(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "sum" => {
            let period = params.first().copied().unwrap_or(30.0) as usize;
            indicators::sum(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "maxindex" => {
            let period = params.first().copied().unwrap_or(30.0) as usize;
            indicators::maxindex(close, period)
                .map(|arr| {
                    IndicatorResult::Single(
                        arr.into_raw_vec()
                            .into_iter()
                            .map(|value| value as f64)
                            .collect(),
                    )
                })
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "minindex" => {
            let period = params.first().copied().unwrap_or(30.0) as usize;
            indicators::minindex(close, period)
                .map(|arr| {
                    IndicatorResult::Single(
                        arr.into_raw_vec()
                            .into_iter()
                            .map(|value| value as f64)
                            .collect(),
                    )
                })
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "minmax" => {
            let period = params.first().copied().unwrap_or(30.0) as usize;
            indicators::minmax(close, period)
                .map(|(min_values, max_values)| {
                    IndicatorResult::Double(min_values.into_raw_vec(), max_values.into_raw_vec())
                })
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "minmaxindex" => {
            let period = params.first().copied().unwrap_or(30.0) as usize;
            indicators::minmaxindex(close, period)
                .map(|(min_values, max_values)| {
                    IndicatorResult::Double(
                        min_values
                            .into_raw_vec()
                            .into_iter()
                            .map(|value| value as f64)
                            .collect(),
                        max_values
                            .into_raw_vec()
                            .into_iter()
                            .map(|value| value as f64)
                            .collect(),
                    )
                })
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "zscore" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::zscore(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "linear_reg" | "linreg" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::linear_reg(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "tsf" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::tsf(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "std_dev" => {
            let period = params.first().copied().unwrap_or(5.0) as usize;
            let nb_dev = params.get(1).copied().unwrap_or(1.0);
            indicators::std_dev(close, period, nb_dev)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "percent_rank" => {
            let period = params.first().copied().unwrap_or(10.0) as usize;
            indicators::percent_rank(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "adx" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::adx(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("ADX requires high and low data".to_string()),
        },
        "aroon" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::aroon(h, l, period)
                    .map(|res| {
                        IndicatorResult::Double(
                            res.aroon_up.into_raw_vec(),
                            res.aroon_down.into_raw_vec(),
                        )
                    })
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("Aroon requires high and low data".to_string()),
        },
        "cci" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::cci(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("CCI requires high and low data".to_string()),
        },
        "willr" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::willr(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("WillR requires high and low data".to_string()),
        },
        "dx" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::dx(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("DX requires high and low data".to_string()),
        },
        "minus_di" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::minus_di(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("MinusDI requires high and low data".to_string()),
        },
        "plus_di" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::plus_di(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("PlusDI requires high and low data".to_string()),
        },
        "minus_dm" => match (high, low) {
            (Some(h), Some(l)) => indicators::minus_dm(h, l)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("MinusDM requires high and low data".to_string()),
        },
        "plus_dm" => match (high, low) {
            (Some(h), Some(l)) => indicators::plus_dm(h, l)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("PlusDM requires high and low data".to_string()),
        },
        "stoch" => match (high, low) {
            (Some(h), Some(l)) => {
                let fastk = params.first().copied().unwrap_or(5.0) as usize;
                let slowk = params.get(1).copied().unwrap_or(3.0) as usize;
                let slowd = params.get(2).copied().unwrap_or(3.0) as usize;
                indicators::stoch(h, l, close, fastk, slowk, slowd)
                    .map(|res| IndicatorResult::Double(res.k.into_raw_vec(), res.d.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("Stoch requires high and low data".to_string()),
        },
        "atr" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::atr(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("ATR requires high and low data".to_string()),
        },
        "natr" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::natr(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("NATR requires high and low data".to_string()),
        },
        "trange" => match (high, low) {
            (Some(h), Some(l)) => indicators::trange(h, l, close)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("TRange requires high and low data".to_string()),
        },
        "mfi" => match (high, low, volume) {
            (Some(h), Some(l), Some(v)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::mfi(h, l, close, v, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("MFI requires high, low and volume data".to_string()),
        },
        "obv" => match volume {
            Some(v) => indicators::obv(close, v)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("OBV requires volume data".to_string()),
        },
        "ad" => match (high, low, volume) {
            (Some(h), Some(l), Some(v)) => indicators::ad(h, l, close, v)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("AD requires high, low and volume data".to_string()),
        },
        "adosc" => match (high, low, volume) {
            (Some(h), Some(l), Some(v)) => {
                let fast = params.first().copied().unwrap_or(3.0) as usize;
                let slow = params.get(1).copied().unwrap_or(10.0) as usize;
                indicators::adosc(h, l, close, v, fast, slow)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("ADOSC requires high, low and volume data".to_string()),
        },
        "bop" => match (open, high, low) {
            (Some(o), Some(h), Some(l)) => indicators::bop(o, h, l, close)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("BOP requires open, high and low data".to_string()),
        },
        "avgprice" => match (open, high, low) {
            (Some(o), Some(h), Some(l)) => indicators::avgprice(o, h, l, close)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("AvgPrice requires open, high and low data".to_string()),
        },
        "medprice" => match (high, low) {
            (Some(h), Some(l)) => indicators::medprice(h, l)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("MedPrice requires high and low data".to_string()),
        },
        "typprice" => match (high, low) {
            (Some(h), Some(l)) => indicators::typprice(h, l, close)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("TypPrice requires high and low data".to_string()),
        },
        "wclprice" => match (high, low) {
            (Some(h), Some(l)) => indicators::wclprice(h, l, close)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("WclPrice requires high and low data".to_string()),
        },
        "mama" => {
            let fastlimit = params.first().copied().unwrap_or(0.5);
            let slowlimit = params.get(1).copied().unwrap_or(0.05);
            indicators::mama(close, fastlimit, slowlimit)
                .map(|res| {
                    IndicatorResult::Double(res.mama.into_raw_vec(), res.fama.into_raw_vec())
                })
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "sar" => match (high, low) {
            (Some(h), Some(l)) => {
                let acceleration = params.first().copied().unwrap_or(0.02);
                let maximum = params.get(1).copied().unwrap_or(0.2);
                indicators::sar(h, l, acceleration, maximum)
                    .map(|res| {
                        IndicatorResult::Double(res.sar.into_raw_vec(), res.af.into_raw_vec())
                    })
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("SAR requires high and low data".to_string()),
        },
        "midprice" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::midprice(h, l, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("MidPrice requires high and low data".to_string()),
        },
        "vortex" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::vortex(h, l, close, period)
                    .map(|res| {
                        IndicatorResult::Double(
                            res.vi_plus.into_raw_vec(),
                            res.vi_minus.into_raw_vec(),
                        )
                    })
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("Vortex requires high and low data".to_string()),
        },
        "vzo" => match volume {
            Some(v) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::vzo(close, v, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("VZO requires volume data".to_string()),
        },
        "volume_momentum" => match volume {
            Some(v) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::volume_momentum(v, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("VolumeMomentum requires volume data".to_string()),
        },
        "volume_roc" => match volume {
            Some(v) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::volume_roc(v, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("VolumeROC requires volume data".to_string()),
        },
        "chande_forecast_oscillator" | "cfo" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::chande_forecast_oscillator(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "twiggs_money_flow" => match (high, low, volume) {
            (Some(h), Some(l), Some(v)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::twiggs_money_flow(h, l, close, v, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error(
                "TwiggsMoneyFlow requires high, low and volume data".to_string(),
            ),
        },
        "inertia" => match (open, high, low) {
            (Some(o), Some(h), Some(l)) => {
                let rvi_period = params.first().copied().unwrap_or(10.0) as usize;
                let linreg_period = params.get(1).copied().unwrap_or(14.0) as usize;
                indicators::inertia(o, h, l, close, rvi_period, linreg_period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("Inertia requires open, high and low data".to_string()),
        },
        "darvas_box" => match (high, low) {
            (Some(h), Some(l)) => {
                let lookback = params.first().copied().unwrap_or(5.0) as usize;
                let confirmation = params.get(1).copied().unwrap_or(3.0) as usize;
                indicators::darvas_box(h, l, close, lookback, confirmation)
                    .map(|r| {
                        IndicatorResult::Triple(
                            r.box_top.into_raw_vec(),
                            r.box_bottom.into_raw_vec(),
                            r.signal.into_iter().map(|v| v as f64).collect(),
                        )
                    })
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("DarvasBox requires high and low data".to_string()),
        },
        "renko" => match (high, low) {
            (Some(h), Some(l)) => {
                let box_size = params.first().copied().unwrap_or(1.0);
                indicators::renko(h, l, box_size)
                    .map(|r| {
                        IndicatorResult::Double(
                            r.bricks.into_raw_vec(),
                            r.direction.into_iter().map(|v| v as f64).collect(),
                        )
                    })
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("Renko requires high and low data".to_string()),
        },
        "kagi" => {
            let reversal = params.first().copied().unwrap_or(1.0);
            indicators::kagi(close, reversal)
                .map(|r| {
                    IndicatorResult::Double(
                        r.kagi.into_raw_vec(),
                        r.direction.into_iter().map(|v| v as f64).collect(),
                    )
                })
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "point_and_figure" | "pnf" => match (high, low) {
            (Some(h), Some(l)) => {
                let box_size = params.first().copied().unwrap_or(1.0);
                let reversal = params.get(1).copied().unwrap_or(3.0) as usize;
                indicators::point_and_figure(h, l, box_size, reversal)
                    .map(|r| {
                        IndicatorResult::Triple(
                            r.pnf.into_raw_vec(),
                            r.column_type.into_iter().map(|v| v as f64).collect(),
                            r.new_column.into_iter().map(|v| v as f64).collect(),
                        )
                    })
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("PointAndFigure requires high and low data".to_string()),
        },
        "three_line_break" | "tlb" => {
            let lines = params.first().copied().unwrap_or(3.0) as usize;
            indicators::three_line_break(close, lines)
                .map(|r| {
                    IndicatorResult::Double(
                        r.line.into_raw_vec(),
                        r.direction.into_iter().map(|v| v as f64).collect(),
                    )
                })
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "williams_alligator" | "alligator" => indicators::williams_alligator(close)
            .map(|r| {
                IndicatorResult::Triple(
                    r.jaw.into_raw_vec(),
                    r.teeth.into_raw_vec(),
                    r.lips.into_raw_vec(),
                )
            })
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "heikin_ashi" | "ha" => match open {
            Some(o) => match (high, low) {
                (Some(h), Some(l)) => indicators::heikin_ashi(o, h, l, close)
                    .map(|r| {
                        IndicatorResult::Quad(
                            r.ha_open.into_raw_vec(),
                            r.ha_high.into_raw_vec(),
                            r.ha_low.into_raw_vec(),
                            r.ha_close.into_raw_vec(),
                        )
                    })
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
                _ => IndicatorResult::Error("HeikinAshi requires open, high and low".to_string()),
            },
            _ => IndicatorResult::Error("HeikinAshi requires open data".to_string()),
        },
        name if name.starts_with("cdl") => match (open, high, low) {
            (Some(o), Some(h), Some(l)) => match name {
                "cdl2crows" => pattern_result(candlestick::cdl_2crows(o, h, l, close)),
                "cdl3blackcrows" => pattern_result(candlestick::cdl_3black_crows(o, h, l, close)),
                "cdl3inside" => pattern_result(candlestick::cdl_3inside(o, h, l, close)),
                "cdl3linestrike" => pattern_result(candlestick::cdl_3linestrike(o, h, l, close)),
                "cdl3outside" => pattern_result(candlestick::cdl_3outside(o, h, l, close)),
                "cdl3starsinsouth" => {
                    pattern_result(candlestick::cdl_3starsinsouth(o, h, l, close))
                }
                "cdl3whitesoldiers" => {
                    pattern_result(candlestick::cdl_3white_soldiers(o, h, l, close))
                }
                "cdlabandonedbaby" => {
                    pattern_result(candlestick::cdl_abandoned_baby(o, h, l, close))
                }
                "cdladvanceblock" => pattern_result(candlestick::cdl_advanceblock(o, h, l, close)),
                "cdlbelthold" => pattern_result(candlestick::cdl_belthold(o, h, l, close)),
                "cdlbreakaway" => pattern_result(candlestick::cdl_breakaway(o, h, l, close)),
                "cdlclosingmarubozu" => {
                    pattern_result(candlestick::cdl_closingmarubozu(o, h, l, close))
                }
                "cdlconcealbabyswall" => {
                    pattern_result(candlestick::cdl_concealbabyswall(o, h, l, close))
                }
                "cdlcounterattack" => {
                    pattern_result(candlestick::cdl_counterattack(o, h, l, close))
                }
                "cdldarkcloudcover" => {
                    pattern_result(candlestick::cdl_darkcloudcover(o, h, l, close))
                }
                "cdldoji" => pattern_result(candlestick::cdl_doji(o, h, l, close)),
                "cdldojistar" => pattern_result(candlestick::cdl_doji_star(o, h, l, close)),
                "cdldragonflydoji" => {
                    pattern_result(candlestick::cdl_dragonflydoji(o, h, l, close))
                }
                "cdlengulfing" => pattern_result(candlestick::cdl_engulfing(o, h, l, close)),
                "cdleveningdojistar" => {
                    pattern_result(candlestick::cdl_eveningdojistar(o, h, l, close))
                }
                "cdleveningstar" => pattern_result(candlestick::cdl_eveningstar(o, h, l, close)),
                "cdlgapsidesidewhite" => {
                    pattern_result(candlestick::cdl_gap_side_white(o, h, l, close))
                }
                "cdlgravestonedoji" => {
                    pattern_result(candlestick::cdl_gravestonedoji(o, h, l, close))
                }
                "cdlhammer" => pattern_result(candlestick::cdl_hammer(o, h, l, close)),
                "cdlhangingman" => pattern_result(candlestick::cdl_hangingman(o, h, l, close)),
                "cdlharami" => pattern_result(candlestick::cdl_harami(o, h, l, close)),
                "cdlharamicross" => pattern_result(candlestick::cdl_haramicross(o, h, l, close)),
                "cdlhighwave" => pattern_result(candlestick::cdl_highwave(o, h, l, close)),
                "cdlhikkake" => pattern_result(candlestick::cdl_hikkake(o, h, l, close)),
                "cdlhikkakemod" => pattern_result(candlestick::cdl_hikkake_mod(o, h, l, close)),
                "cdlhomingpigeon" => pattern_result(candlestick::cdl_homing_pigeon(o, h, l, close)),
                "cdlidentical3crows" => {
                    pattern_result(candlestick::cdl_identical3crows(o, h, l, close))
                }
                "cdlinneck" => pattern_result(candlestick::cdl_inneck(o, h, l, close)),
                "cdlinvertedhammer" => {
                    pattern_result(candlestick::cdl_invertedhammer(o, h, l, close))
                }
                "cdlkicking" => pattern_result(candlestick::cdl_kicking(o, h, l, close)),
                "cdlkickingbylength" => {
                    pattern_result(candlestick::cdl_kickingbylength(o, h, l, close))
                }
                "cdlladderbottom" => pattern_result(candlestick::cdl_ladder_bottom(o, h, l, close)),
                "cdllongleggeddoji" => {
                    pattern_result(candlestick::cdl_longleggeddoji(o, h, l, close))
                }
                "cdllongline" => pattern_result(candlestick::cdl_longline(o, h, l, close)),
                "cdlmarubozu" => pattern_result(candlestick::cdl_marubozu(o, h, l, close)),
                "cdlmatchinglow" => pattern_result(candlestick::cdl_matchinglow(o, h, l, close)),
                "cdlmathold" => pattern_result(candlestick::cdl_mathold(o, h, l, close)),
                "cdlmorningdojistar" => {
                    pattern_result(candlestick::cdl_morningdojistar(o, h, l, close))
                }
                "cdlmorningstar" => pattern_result(candlestick::cdl_morningstar(o, h, l, close)),
                "cdlonneck" => pattern_result(candlestick::cdl_onneck(o, h, l, close)),
                "cdlpiercing" => pattern_result(candlestick::cdl_piercing(o, h, l, close)),
                "cdlrickshawman" => pattern_result(candlestick::cdl_rickshawman(o, h, l, close)),
                "cdlrisefall3methods" => {
                    pattern_result(candlestick::cdl_rise_fall_3methods(o, h, l, close))
                }
                "cdlseparatinglines" => {
                    pattern_result(candlestick::cdl_separatinglines(o, h, l, close))
                }
                "cdlshootingstar" => pattern_result(candlestick::cdl_shootingstar(o, h, l, close)),
                "cdlshortline" => pattern_result(candlestick::cdl_shortline(o, h, l, close)),
                "cdlspinningtop" => pattern_result(candlestick::cdl_spinningtop(o, h, l, close)),
                "cdlstalledpattern" => {
                    pattern_result(candlestick::cdl_stalledpattern(o, h, l, close))
                }
                "cdlsticksandwich" => {
                    pattern_result(candlestick::cdl_sticksandwich(o, h, l, close))
                }
                "cdltakuri" => pattern_result(candlestick::cdl_takuri(o, h, l, close)),
                "cdltasukigap" => pattern_result(candlestick::cdl_tasukigap(o, h, l, close)),
                "cdlthrusting" => pattern_result(candlestick::cdl_thrusting(o, h, l, close)),
                "cdltristar" => pattern_result(candlestick::cdl_tristar(o, h, l, close)),
                "cdlunique3river" => pattern_result(candlestick::cdl_unique3river(o, h, l, close)),
                "cdlupsidegap2crows" => {
                    pattern_result(candlestick::cdl_upsidegap2crows(o, h, l, close))
                }
                "cdlxsidegap3methods" => {
                    pattern_result(candlestick::cdl_xsidegap3methods(o, h, l, close))
                }
                _ => IndicatorResult::Error(format!("Unsupported candlestick function: {}", name)),
            },
            _ => IndicatorResult::Error(
                "Candlestick functions require open, high and low data".to_string(),
            ),
        },
        _ => IndicatorResult::Error(format!("Unknown indicator: {}", name)),
    }
}

#[pymodule]
fn finkit(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(operation_catalog_json, m)?)?;
    m.add_function(wrap_pyfunction!(factor_catalog_json, m)?)?;
    m.add_function(wrap_pyfunction!(operation_execute_json, m)?)?;
    m.add_function(wrap_pyfunction!(composite_execute_json, m)?)?;
    m.add_function(wrap_pyfunction!(composite_stream_execute_json, m)?)?;
    m.add_function(wrap_pyfunction!(factor_execute_json, m)?)?;
    m.add_function(wrap_pyfunction!(factor_cross_sectional_execute_json, m)?)?;
    m.add_function(wrap_pyfunction!(factor_stream_execute_json, m)?)?;
    m.add_class::<PyKlineData>()?;
    m.add_class::<PyKlineChart>()?;
    m.add_function(wrap_pyfunction!(chan_analyze, m)?)?;
    m.add_function(wrap_pyfunction!(chan_analyze_multi, m)?)?;
    m.add_function(wrap_pyfunction!(chan_analyze_multi_timestamps_py, m)?)?;
    m.add_function(wrap_pyfunction!(
        chan_analyze_multi_timestamps_calendar_py,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(resolve_market_session_py, m)?)?;
    m.add_function(wrap_pyfunction!(resolve_market_session_config_py, m)?)?;
    m.add_function(wrap_pyfunction!(resolve_market_session_csv_py, m)?)?;

    // Overlap Studies
    m.add_function(wrap_pyfunction!(sma, m)?)?;
    m.add_function(wrap_pyfunction!(ema, m)?)?;
    m.add_function(wrap_pyfunction!(wma, m)?)?;
    m.add_function(wrap_pyfunction!(dema, m)?)?;
    m.add_function(wrap_pyfunction!(tema, m)?)?;
    m.add_function(wrap_pyfunction!(kama, m)?)?;
    m.add_function(wrap_pyfunction!(mama, m)?)?;
    m.add_function(wrap_pyfunction!(t3, m)?)?;
    m.add_function(wrap_pyfunction!(bollinger_bands, m)?)?;
    m.add_function(wrap_pyfunction!(sar, m)?)?;
    m.add_function(wrap_pyfunction!(midpoint, m)?)?;
    m.add_function(wrap_pyfunction!(midprice, m)?)?;

    // Momentum Indicators
    m.add_function(wrap_pyfunction!(rsi, m)?)?;
    m.add_function(wrap_pyfunction!(macd, m)?)?;
    m.add_function(wrap_pyfunction!(stoch, m)?)?;
    m.add_function(wrap_pyfunction!(adx, m)?)?;
    m.add_function(wrap_pyfunction!(aroon, m)?)?;
    m.add_function(wrap_pyfunction!(cci, m)?)?;
    m.add_function(wrap_pyfunction!(mom, m)?)?;
    m.add_function(wrap_pyfunction!(roc, m)?)?;
    m.add_function(wrap_pyfunction!(willr, m)?)?;
    m.add_function(wrap_pyfunction!(apo, m)?)?;
    m.add_function(wrap_pyfunction!(bop, m)?)?;
    m.add_function(wrap_pyfunction!(cmo, m)?)?;
    m.add_function(wrap_pyfunction!(dx, m)?)?;
    m.add_function(wrap_pyfunction!(mfi, m)?)?;
    m.add_function(wrap_pyfunction!(minus_di, m)?)?;
    m.add_function(wrap_pyfunction!(minus_dm, m)?)?;
    m.add_function(wrap_pyfunction!(plus_di, m)?)?;
    m.add_function(wrap_pyfunction!(plus_dm, m)?)?;
    m.add_function(wrap_pyfunction!(trix, m)?)?;

    // Cycle Indicators (Hilbert Transform)
    m.add_function(wrap_pyfunction!(ht_dcperiod, m)?)?;
    m.add_function(wrap_pyfunction!(ht_dcphase, m)?)?;
    m.add_function(wrap_pyfunction!(ht_phasor, m)?)?;
    m.add_function(wrap_pyfunction!(ht_sine, m)?)?;
    m.add_function(wrap_pyfunction!(ht_trendmode, m)?)?;
    m.add_function(wrap_pyfunction!(ht_trendline, m)?)?;

    // Volume Indicators
    m.add_function(wrap_pyfunction!(obv, m)?)?;
    m.add_function(wrap_pyfunction!(ad, m)?)?;
    m.add_function(wrap_pyfunction!(adosc, m)?)?;

    // Volatility Indicators
    m.add_function(wrap_pyfunction!(atr, m)?)?;
    m.add_function(wrap_pyfunction!(natr, m)?)?;
    m.add_function(wrap_pyfunction!(trange, m)?)?;

    // Price Transforms
    m.add_function(wrap_pyfunction!(avgprice, m)?)?;
    m.add_function(wrap_pyfunction!(medprice, m)?)?;
    m.add_function(wrap_pyfunction!(typprice, m)?)?;
    m.add_function(wrap_pyfunction!(wclprice, m)?)?;

    // Statistics Functions
    m.add_function(wrap_pyfunction!(zscore, m)?)?;
    m.add_function(wrap_pyfunction!(percent_rank, m)?)?;
    m.add_function(wrap_pyfunction!(beta, m)?)?;
    m.add_function(wrap_pyfunction!(correlation, m)?)?;
    m.add_function(wrap_pyfunction!(std_dev, m)?)?;
    m.add_function(wrap_pyfunction!(var, m)?)?;
    m.add_function(wrap_pyfunction!(linear_reg, m)?)?;
    m.add_function(wrap_pyfunction!(tsf, m)?)?;

    // Candlestick Patterns
    m.add_function(wrap_pyfunction!(cdl_doji, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_dragonfly_doji, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_gravestone_doji, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_long_legged_doji, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_doji_4prices, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_hammer, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_inverted_hammer, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_hanging_man, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_shooting_star, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_engulfing, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_harami, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_harami_cross, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_morning_star, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_evening_star, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_morning_doji_star, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_evening_doji_star, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_marubozu, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_three_white_soldiers, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_three_black_crows, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_three_inside_up, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_three_outside_up, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_three_inside_down, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_three_outside_down, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_piercing, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_dark_cloud_cover, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_belt_hold, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_spinning_top, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_high_wave, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_rickshaw_man, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_short_line, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_long_line, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_kicking, m)?)?;

    // Chart Patterns
    m.add_function(wrap_pyfunction!(detect_head_shoulders, m)?)?;
    m.add_function(wrap_pyfunction!(detect_head_shoulders_bottom, m)?)?;
    m.add_function(wrap_pyfunction!(detect_double_top, m)?)?;
    m.add_function(wrap_pyfunction!(detect_double_bottom, m)?)?;
    m.add_function(wrap_pyfunction!(detect_triple_top, m)?)?;
    m.add_function(wrap_pyfunction!(detect_triple_bottom, m)?)?;
    m.add_function(wrap_pyfunction!(detect_ascending_triangle, m)?)?;
    m.add_function(wrap_pyfunction!(detect_descending_triangle, m)?)?;
    m.add_function(wrap_pyfunction!(detect_symmetrical_triangle, m)?)?;
    m.add_function(wrap_pyfunction!(detect_rising_wedge, m)?)?;
    m.add_function(wrap_pyfunction!(detect_falling_wedge, m)?)?;
    m.add_function(wrap_pyfunction!(detect_flag, m)?)?;
    m.add_function(wrap_pyfunction!(detect_pennant, m)?)?;
    m.add_function(wrap_pyfunction!(detect_rectangle, m)?)?;

    // Advanced Indicators
    m.add_function(wrap_pyfunction!(ichimoku, m)?)?;
    m.add_function(wrap_pyfunction!(supertrend, m)?)?;
    m.add_function(wrap_pyfunction!(vwap, m)?)?;
    m.add_function(wrap_pyfunction!(anchored_vwap, m)?)?;
    m.add_function(wrap_pyfunction!(vwap_bands, m)?)?;
    m.add_function(wrap_pyfunction!(elder_ray, m)?)?;
    m.add_function(wrap_pyfunction!(donchian, m)?)?;
    m.add_function(wrap_pyfunction!(pivot_points, m)?)?;
    m.add_function(wrap_pyfunction!(volume_profile, m)?)?;
    m.add_function(wrap_pyfunction!(fibonacci_retracement, m)?)?;

    // New Indicators (TASK-166~180)
    m.add_function(wrap_pyfunction!(vortex, m)?)?;
    m.add_function(wrap_pyfunction!(inertia, m)?)?;
    m.add_function(wrap_pyfunction!(vzo, m)?)?;
    m.add_function(wrap_pyfunction!(volume_momentum, m)?)?;
    m.add_function(wrap_pyfunction!(volume_roc, m)?)?;
    m.add_function(wrap_pyfunction!(chande_forecast_oscillator, m)?)?;
    m.add_function(wrap_pyfunction!(twiggs_money_flow, m)?)?;

    // Chart constructions (classic price-action family)
    //
    // These are FTA-native, not TA-Lib.  They are the only indicators whose
    // signature mixes a price line with a discrete state column, so they use
    // the integer-carrying helpers above instead of the plain f64 ones.
    m.add_function(wrap_pyfunction!(darvas_box, m)?)?;
    m.add_function(wrap_pyfunction!(renko, m)?)?;
    m.add_function(wrap_pyfunction!(kagi, m)?)?;
    m.add_function(wrap_pyfunction!(point_and_figure, m)?)?;
    m.add_function(wrap_pyfunction!(three_line_break, m)?)?;
    m.add_function(wrap_pyfunction!(williams_alligator, m)?)?;
    m.add_function(wrap_pyfunction!(heikin_ashi, m)?)?;

    // Formula System
    #[cfg(feature = "formula")]
    {
        m.add_class::<PyCompiledFormula>()?;
        m.add_class::<PyFormulaRegistry>()?;
        m.add_function(wrap_pyfunction!(formula_eval, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_dialect, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_contract_json, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_temporal_contract_json, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_panel_contract_json, m)?)?;
        m.add_function(wrap_pyfunction!(
            formula_eval_cross_sectional_contract_json,
            m
        )?)?;
        m.add_function(wrap_pyfunction!(formula_compatibility_report_json, m)?)?;
        m.add_function(wrap_pyfunction!(formula_stream_execute_json, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_bytecode, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_optimized, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_jit, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_simd, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_zero_copy, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_numpy_zero_copy, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_multi, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_draw, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_debug, m)?)?;
        m.add_function(wrap_pyfunction!(formula_validate, m)?)?;
        m.add_function(wrap_pyfunction!(formula_get_template, m)?)?;
        m.add_function(wrap_pyfunction!(formula_search_templates, m)?)?;
        m.add_function(wrap_pyfunction!(formula_list_categories, m)?)?;
    }

    // Factor research and generic quantitative evaluation
    research_api::register_research_api(m)?;

    // Shipped factor libraries (M0-3): `finkit.factor_library("alpha158")`.
    factor_library::register_factor_library(m)?;

    // Streaming Indicators
    streaming::register_streaming_classes(m)?;

    // Sweep API
    sweep::register_sweep_functions(m)?;

    // Transform Pipeline
    transforms::register_transform_classes(m)?;

    // Feature Engineering
    features::register_features_module(m)?;

    // Batch Computation (Single GIL Release)
    m.add_function(wrap_pyfunction!(compute_indicators, m)?)?;
    m.add_function(wrap_pyfunction!(compute_composite, m)?)?;

    // TA-Lib-compatible direct bindings for the remaining core indicators.
    compat_api::register(m)?;

    Ok(())
}
