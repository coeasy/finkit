#![allow(missing_docs)]
#![allow(missing_debug_implementations)]
#![allow(deprecated)]

use wasm_bindgen::prelude::*;

use finkit::formula::{
    parse_formula, DrawCommand, FormulaContext, FormulaEngine, FormulaTemplates,
};
use finkit::indicators;
use finkit::math::moving_avg;
use finkit::patterns::{candlestick, chart};
use ndarray::Array1;

mod streaming;
mod transforms;

#[wasm_bindgen(start)]
pub fn _start() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

fn to_js(e: impl std::fmt::Display) -> JsError {
    JsError::new(&format!("{}", e))
}

// ───────────────────── Moving Averages ─────────────────────

#[wasm_bindgen]
pub fn sma(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    moving_avg::sma(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sma_basic() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = moving_avg::sma(&input, 3).unwrap();
        let vals = result.into_raw_vec_and_offset().0;
        assert!((vals[0] - 2.0).abs() < 1e-10);
        assert!((vals[1] - 3.0).abs() < 1e-10);
        assert!((vals[2] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_ema_basic() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = moving_avg::ema(&input, 3).unwrap();
        assert!(result.len() == input.len());
        // First value should be SMA seed (average of first 3)
        assert!((result[0] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_rsi_basic() {
        let input = vec![
            44.0, 44.34, 44.09, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 46.08,
        ];
        let result = indicators::momentum::rsi(&input, 5).unwrap();
        assert_eq!(result.len(), input.len());
        // After warmup period, RSI should be a valid percentage
        if let Some(&v) = result.last() {
            assert!(v >= 0.0 && v <= 100.0);
        }
    }
}

#[wasm_bindgen]
pub fn ema(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    moving_avg::ema(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn wma(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    moving_avg::wma(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn dema(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    moving_avg::dema(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn tema(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    moving_avg::tema(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn kama(
    input: Vec<f64>,
    period: usize,
    fast_period: usize,
    slow_period: usize,
) -> Result<Vec<f64>, JsError> {
    moving_avg::kama(&input, period, fast_period, slow_period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn t3(input: Vec<f64>, period: usize, vfactor: f64) -> Result<Vec<f64>, JsError> {
    indicators::t3(&input, period, vfactor)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

// ───────────────────── Momentum ─────────────────────

#[wasm_bindgen]
pub fn rsi(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::rsi(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct MacdResult {
    pub macd: Vec<f64>,
    pub signal: Vec<f64>,
    pub hist: Vec<f64>,
}

#[wasm_bindgen]
pub fn macd(
    input: Vec<f64>,
    fast: usize,
    slow: usize,
    signal: usize,
) -> Result<MacdResult, JsError> {
    indicators::macd(&input, fast, slow, signal)
        .map(|r| MacdResult {
            macd: r.macd.into_raw_vec_and_offset().0,
            signal: r.signal.into_raw_vec_and_offset().0,
            hist: r.hist.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn adx(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::adx(&high, &low, &close, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn cci(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::cci(&high, &low, &close, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn mom(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::mom(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn roc(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::roc(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn willr(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::willr(&high, &low, &close, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn apo(input: Vec<f64>, fast_period: usize, slow_period: usize) -> Result<Vec<f64>, JsError> {
    indicators::apo(&input, fast_period, slow_period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn bop(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<f64>, JsError> {
    indicators::bop(&open, &high, &low, &close)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn cmo(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::cmo(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn dx(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::dx(&high, &low, &close, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn mfi(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::mfi(&high, &low, &close, &volume, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn minus_di(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::minus_di(&high, &low, &close, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn plus_di(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::plus_di(&high, &low, &close, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn minus_dm(high: Vec<f64>, low: Vec<f64>) -> Result<Vec<f64>, JsError> {
    indicators::minus_dm(&high, &low)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn plus_dm(high: Vec<f64>, low: Vec<f64>) -> Result<Vec<f64>, JsError> {
    indicators::plus_dm(&high, &low)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn trix(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::trix(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct StochResult {
    pub k: Vec<f64>,
    pub d: Vec<f64>,
}

#[wasm_bindgen]
pub fn stoch(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    k_period: usize,
    k_slow: usize,
    d_period: usize,
) -> Result<StochResult, JsError> {
    indicators::stoch(&high, &low, &close, k_period, k_slow, d_period)
        .map(|r| StochResult {
            k: r.k.into_raw_vec_and_offset().0,
            d: r.d.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct AroonResult {
    pub aroon_up: Vec<f64>,
    pub aroon_down: Vec<f64>,
}

#[wasm_bindgen]
pub fn aroon(high: Vec<f64>, low: Vec<f64>, period: usize) -> Result<AroonResult, JsError> {
    indicators::aroon(&high, &low, period)
        .map(|r| AroonResult {
            aroon_up: r.aroon_up.into_raw_vec_and_offset().0,
            aroon_down: r.aroon_down.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct ElderRayResult {
    pub bull_power: Vec<f64>,
    pub bear_power: Vec<f64>,
}

#[wasm_bindgen]
pub fn elder_ray(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    period: usize,
) -> Result<ElderRayResult, JsError> {
    indicators::elder_ray(&high, &low, &close, &volume, period)
        .map(|r| ElderRayResult {
            bull_power: r.bull_power.into_raw_vec_and_offset().0,
            bear_power: r.bear_power.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

// ───────────────────── Overlap ─────────────────────

#[wasm_bindgen(getter_with_clone)]
pub struct BbandsResult {
    pub upper: Vec<f64>,
    pub middle: Vec<f64>,
    pub lower: Vec<f64>,
}

#[wasm_bindgen]
pub fn bbands(
    input: Vec<f64>,
    period: usize,
    nb_dev_up: f64,
    nb_dev_dn: f64,
) -> Result<BbandsResult, JsError> {
    indicators::bbands(&input, period, nb_dev_up, nb_dev_dn)
        .map(|r| BbandsResult {
            upper: r.upper.into_raw_vec_and_offset().0,
            middle: r.middle.into_raw_vec_and_offset().0,
            lower: r.lower.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn midpoint(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::midpoint(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn midprice(high: Vec<f64>, low: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::midprice(&high, &low, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct SarResultWasm {
    pub sar: Vec<f64>,
    pub af: Vec<f64>,
}

#[wasm_bindgen]
pub fn sar(
    high: Vec<f64>,
    low: Vec<f64>,
    acceleration: f64,
    maximum: f64,
) -> Result<SarResultWasm, JsError> {
    indicators::sar(&high, &low, acceleration, maximum)
        .map(|r| SarResultWasm {
            sar: r.sar.into_raw_vec_and_offset().0,
            af: r.af.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

// ───────────────────── Volatility ─────────────────────

#[wasm_bindgen]
pub fn atr(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::atr(&high, &low, &close, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn natr(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::natr(&high, &low, &close, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn trange(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<f64>, JsError> {
    indicators::trange(&high, &low, &close)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

// ───────────────────── Volume ─────────────────────

#[wasm_bindgen]
pub fn ad(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
) -> Result<Vec<f64>, JsError> {
    indicators::ad(&high, &low, &close, &volume)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn adosc(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    fast_period: usize,
    slow_period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::adosc(&high, &low, &close, &volume, fast_period, slow_period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn obv(close: Vec<f64>, volume: Vec<f64>) -> Result<Vec<f64>, JsError> {
    indicators::obv(&close, &volume)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn vwap(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
) -> Result<Vec<f64>, JsError> {
    indicators::vwap(&high, &low, &close, &volume)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

// ───────────────────── Price Transform ─────────────────────

#[wasm_bindgen]
pub fn avgprice(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<f64>, JsError> {
    indicators::avgprice(&open, &high, &low, &close)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn medprice(high: Vec<f64>, low: Vec<f64>) -> Result<Vec<f64>, JsError> {
    indicators::medprice(&high, &low)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn typprice(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<f64>, JsError> {
    indicators::typprice(&high, &low, &close)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn wclprice(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> Result<Vec<f64>, JsError> {
    indicators::wclprice(&high, &low, &close)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

// ───────────────────── Statistics ─────────────────────

#[wasm_bindgen]
pub fn zscore(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::zscore(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn percent_rank(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::percent_rank(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn beta(asset: Vec<f64>, benchmark: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::beta(&asset, &benchmark, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn correlation(
    input_a: Vec<f64>,
    input_b: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::correlation(&input_a, &input_b, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn std_dev(input: Vec<f64>, period: usize, nb_dev: f64) -> Result<Vec<f64>, JsError> {
    indicators::std_dev(&input, period, nb_dev)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn linear_reg(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::linear_reg(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn tsf(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::tsf(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

// ───────────────────── Cycle ─────────────────────

#[wasm_bindgen]
pub fn ht_dcperiod(input: Vec<f64>) -> Result<Vec<f64>, JsError> {
    indicators::ht_dcperiod(&input)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn ht_dcphase(input: Vec<f64>) -> Result<Vec<f64>, JsError> {
    indicators::ht_dcphase(&input)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn ht_trendmode(input: Vec<f64>) -> Result<Vec<f64>, JsError> {
    indicators::ht_trendmode(&input)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn ht_trendline(input: Vec<f64>) -> Result<Vec<f64>, JsError> {
    indicators::ht_trendline(&input)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

// ───────────────────── Composite Indicators ─────────────────────

#[wasm_bindgen(getter_with_clone)]
pub struct DonchianResultWasm {
    pub upper: Vec<f64>,
    pub lower: Vec<f64>,
    pub middle: Vec<f64>,
    pub width: Vec<f64>,
}

#[wasm_bindgen]
pub fn donchian(
    high: Vec<f64>,
    low: Vec<f64>,
    period: usize,
) -> Result<DonchianResultWasm, JsError> {
    indicators::donchian(&high, &low, period)
        .map(|r| DonchianResultWasm {
            upper: r.upper.into_raw_vec_and_offset().0,
            lower: r.lower.into_raw_vec_and_offset().0,
            middle: r.middle.into_raw_vec_and_offset().0,
            width: r.width.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct IchimokuResultWasm {
    pub tenkan_sen: Vec<f64>,
    pub kijun_sen: Vec<f64>,
    pub senkou_span_a: Vec<f64>,
    pub senkou_span_b: Vec<f64>,
    pub chikou_span: Vec<f64>,
}

#[wasm_bindgen]
pub fn ichimoku(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    tenkan: usize,
    kijun: usize,
    senkou: usize,
    displacement: usize,
) -> Result<IchimokuResultWasm, JsError> {
    indicators::ichimoku(&high, &low, &close, tenkan, kijun, senkou, displacement)
        .map(|r| IchimokuResultWasm {
            tenkan_sen: r.tenkan_sen.into_raw_vec_and_offset().0,
            kijun_sen: r.kijun_sen.into_raw_vec_and_offset().0,
            senkou_span_a: r.senkou_span_a.into_raw_vec_and_offset().0,
            senkou_span_b: r.senkou_span_b.into_raw_vec_and_offset().0,
            chikou_span: r.chikou_span.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct SuperTrendResultWasm {
    pub direction: Vec<i32>,
    pub trend_line: Vec<f64>,
    pub upper_band: Vec<f64>,
    pub lower_band: Vec<f64>,
}

#[wasm_bindgen]
pub fn supertrend(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
    multiplier: f64,
) -> Result<SuperTrendResultWasm, JsError> {
    indicators::supertrend(&high, &low, &close, period, multiplier)
        .map(|r| SuperTrendResultWasm {
            direction: r.direction.into_raw_vec_and_offset().0,
            trend_line: r.trend_line.into_raw_vec_and_offset().0,
            upper_band: r.upper_band.into_raw_vec_and_offset().0,
            lower_band: r.lower_band.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

// ───────────────────── Candlestick Patterns ─────────────────────

#[wasm_bindgen]
pub fn pattern_doji(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    doji_pct: f64,
) -> Result<Vec<i32>, JsError> {
    candlestick::doji(&open, &high, &low, &close, doji_pct)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn pattern_hammer(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>, JsError> {
    candlestick::hammer(&open, &high, &low, &close)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn pattern_engulfing(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>, JsError> {
    candlestick::engulfing(&open, &high, &low, &close)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn pattern_morning_star(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>, JsError> {
    candlestick::morning_star(&open, &high, &low, &close)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn pattern_evening_star(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>, JsError> {
    candlestick::evening_star(&open, &high, &low, &close)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn pattern_shooting_star(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>, JsError> {
    candlestick::shooting_star(&open, &high, &low, &close)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn pattern_hanging_man(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>, JsError> {
    candlestick::hanging_man(&open, &high, &low, &close)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn pattern_inverted_hammer(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>, JsError> {
    candlestick::inverted_hammer(&open, &high, &low, &close)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn pattern_dark_cloud_cover(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>, JsError> {
    candlestick::dark_cloud_cover(&open, &high, &low, &close)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn pattern_piercing(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<Vec<i32>, JsError> {
    candlestick::piercing(&open, &high, &low, &close)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

// ───────────────────── Chart Patterns ─────────────────────

#[wasm_bindgen]
pub fn pattern_double_top(
    high: Vec<f64>,
    window: usize,
    tolerance: f64,
) -> Result<Vec<i32>, JsError> {
    chart::double_top(&high, window, tolerance)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn pattern_double_bottom(
    low: Vec<f64>,
    window: usize,
    tolerance: f64,
) -> Result<Vec<i32>, JsError> {
    chart::double_bottom(&low, window, tolerance)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn pattern_head_and_shoulders_top(
    high: Vec<f64>,
    window: usize,
    tolerance: f64,
) -> Result<Vec<i32>, JsError> {
    chart::head_and_shoulders_top(&high, window, tolerance)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn pattern_head_and_shoulders_bottom(
    low: Vec<f64>,
    window: usize,
    tolerance: f64,
) -> Result<Vec<i32>, JsError> {
    chart::head_and_shoulders_bottom(&low, window, tolerance)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn pattern_ascending_triangle(
    high: Vec<f64>,
    low: Vec<f64>,
    window: usize,
    tolerance: f64,
) -> Result<Vec<i32>, JsError> {
    chart::ascending_triangle(&high, &low, window, tolerance)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn pattern_descending_triangle(
    high: Vec<f64>,
    low: Vec<f64>,
    window: usize,
    tolerance: f64,
) -> Result<Vec<i32>, JsError> {
    chart::descending_triangle(&high, &low, window, tolerance)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

// ───────────────────── Formula System ─────────────────────

fn f64_to_json(v: f64) -> serde_json::Value {
    if v.is_finite() {
        serde_json::Value::from(v)
    } else {
        serde_json::Value::Null
    }
}

fn arr_to_json(arr: &Array1<f64>) -> serde_json::Value {
    serde_json::Value::Array(arr.iter().map(|v| f64_to_json(*v)).collect())
}

fn draw_command_to_js(cmd: &DrawCommand) -> JsValue {
    let val = match cmd {
        DrawCommand::Text {
            condition,
            price,
            text,
            color,
        } => serde_json::json!({
            "type": "Text",
            "condition": arr_to_json(condition),
            "price": arr_to_json(price),
            "text": text,
            "color": color
        }),
        DrawCommand::Icon {
            condition,
            price,
            icon_type,
            color,
        } => serde_json::json!({
            "type": "Icon",
            "condition": arr_to_json(condition),
            "price": arr_to_json(price),
            "iconType": icon_type,
            "color": color
        }),
        DrawCommand::StickLine {
            condition,
            price1,
            price2,
            width,
            empty,
            color,
        } => serde_json::json!({
            "type": "StickLine",
            "condition": arr_to_json(condition),
            "price1": arr_to_json(price1),
            "price2": arr_to_json(price2),
            "width": width,
            "empty": empty,
            "color": color
        }),
        DrawCommand::Line {
            cond1,
            price1,
            cond2,
            price2,
            expand,
            color,
        } => serde_json::json!({
            "type": "Line",
            "cond1": arr_to_json(cond1),
            "price1": arr_to_json(price1),
            "cond2": arr_to_json(cond2),
            "price2": arr_to_json(price2),
            "expand": expand,
            "color": color
        }),
        DrawCommand::Band {
            val1,
            color1,
            val2,
            color2,
        } => serde_json::json!({
            "type": "Band",
            "val1": arr_to_json(val1),
            "color1": color1,
            "val2": arr_to_json(val2),
            "color2": color2
        }),
        DrawCommand::KLine {
            open,
            high,
            low,
            close,
        } => serde_json::json!({
            "type": "KLine",
            "open": arr_to_json(open),
            "high": arr_to_json(high),
            "low": arr_to_json(low),
            "close": arr_to_json(close)
        }),
        DrawCommand::Rect {
            x1,
            y1,
            x2,
            y2,
            color,
        } => serde_json::json!({
            "type": "Rect",
            "x1": arr_to_json(x1),
            "y1": arr_to_json(y1),
            "x2": arr_to_json(x2),
            "y2": arr_to_json(y2),
            "color": color
        }),
        DrawCommand::FillRgn {
            cond,
            price1,
            price2,
            color,
        } => serde_json::json!({
            "type": "FillRgn",
            "cond": arr_to_json(cond),
            "price1": arr_to_json(price1),
            "price2": arr_to_json(price2),
            "color": color
        }),
        DrawCommand::PartLine { cond, price, color } => serde_json::json!({
            "type": "PartLine",
            "cond": arr_to_json(cond),
            "price": arr_to_json(price),
            "color": color
        }),
        DrawCommand::PolyLine { cond, price, color } => serde_json::json!({
            "type": "PolyLine",
            "cond": arr_to_json(cond),
            "price": arr_to_json(price),
            "color": color
        }),
        DrawCommand::Background { cond, color } => serde_json::json!({
            "type": "Background",
            "cond": arr_to_json(cond),
            "color": color
        }),
        DrawCommand::SlopeLine {
            cond1,
            price1,
            cond2,
            price2,
            color,
        } => serde_json::json!({
            "type": "SlopeLine",
            "cond1": arr_to_json(cond1),
            "price1": arr_to_json(price1),
            "cond2": arr_to_json(cond2),
            "price2": arr_to_json(price2),
            "color": color
        }),
        DrawCommand::TextFix { x, y, text, color } => serde_json::json!({
            "type": "TextFix",
            "x": x,
            "y": y,
            "text": text,
            "color": color
        }),
        DrawCommand::Number {
            condition,
            price,
            number,
            precision,
            color,
        } => serde_json::json!({
            "type": "Number",
            "condition": arr_to_json(condition),
            "price": arr_to_json(price),
            "number": arr_to_json(number),
            "precision": precision,
            "color": color
        }),
        DrawCommand::VertLine { condition, color } => serde_json::json!({
            "type": "VertLine",
            "condition": arr_to_json(condition),
            "color": color
        }),
    };
    serde_wasm_bindgen::to_value(&val).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
pub fn formula_eval(
    source: &str,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
) -> Result<Vec<f64>, JsError> {
    let open = Array1::from_vec(open);
    let high = Array1::from_vec(high);
    let low = Array1::from_vec(low);
    let close = Array1::from_vec(close);
    let volume = Array1::from_vec(volume);
    let mut ctx = FormulaContext::new(open, high, low, close, volume, None);
    let mut engine = FormulaEngine::new();
    engine
        .eval(source, &mut ctx)
        .map(|a| a.to_vec())
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct FormulaMultiResult {
    pub names: Vec<String>,
    pub values: Vec<JsValue>,
    pub draw_commands: Vec<JsValue>,
}

#[wasm_bindgen]
pub fn formula_eval_multi(
    source: &str,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
) -> Result<FormulaMultiResult, JsError> {
    let open = Array1::from_vec(open);
    let high = Array1::from_vec(high);
    let low = Array1::from_vec(low);
    let close = Array1::from_vec(close);
    let volume = Array1::from_vec(volume);
    let mut ctx = FormulaContext::new(open, high, low, close, volume, None);
    let mut engine = FormulaEngine::new();
    let multi = engine.eval_multi(source, &mut ctx).map_err(to_js)?;
    let mut names = Vec::new();
    let mut values = Vec::new();
    for (name, arr) in &multi.outputs {
        names.push(name.clone());
        values.push(serde_wasm_bindgen::to_value(&arr.to_vec()).unwrap_or(JsValue::NULL));
    }
    let draw_commands = ctx
        .draw_commands
        .borrow()
        .commands
        .iter()
        .map(|cmd| draw_command_to_js(cmd))
        .collect();
    Ok(FormulaMultiResult {
        names,
        values,
        draw_commands,
    })
}

#[wasm_bindgen]
pub fn formula_validate(source: &str) -> bool {
    parse_formula(source).is_ok()
}

#[wasm_bindgen(getter_with_clone)]
pub struct FormulaTemplateInfo {
    pub name: String,
    pub description: String,
    pub category: String,
    pub source: String,
}

#[wasm_bindgen]
pub fn formula_get_template(name: &str) -> Result<FormulaTemplateInfo, JsError> {
    let engine = FormulaEngine::new();
    let tmpl = engine
        .get_template(name)
        .ok_or_else(|| JsError::new(&format!("Template not found: {}", name)))?;
    Ok(FormulaTemplateInfo {
        name: tmpl.name.clone(),
        description: tmpl.description.clone(),
        category: format!("{:?}", tmpl.category),
        source: tmpl.source.clone(),
    })
}

#[wasm_bindgen]
pub fn formula_search_templates(keyword: &str) -> Vec<FormulaTemplateInfo> {
    let engine = FormulaEngine::new();
    engine
        .search_templates(keyword)
        .iter()
        .map(|t| FormulaTemplateInfo {
            name: t.name.clone(),
            description: t.description.clone(),
            category: format!("{:?}", t.category),
            source: t.source.clone(),
        })
        .collect()
}

#[wasm_bindgen]
pub fn formula_list_categories() -> Vec<String> {
    FormulaTemplates::categories()
        .iter()
        .map(|c| format!("{:?}", c))
        .collect()
}

// ───────────────────── Extended Overlap Studies ─────────────────────

#[wasm_bindgen]
pub fn hma(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::hma(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn alma(
    input: Vec<f64>,
    period: usize,
    offset_factor: f64,
    sigma: f64,
) -> Result<Vec<f64>, JsError> {
    indicators::alma(&input, period, offset_factor, sigma)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn vidya(
    input: Vec<f64>,
    short_period: usize,
    long_period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::vidya(&input, short_period, long_period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn frama(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::frama(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn jma(input: Vec<f64>, period: usize, phase: f64, power: f64) -> Result<Vec<f64>, JsError> {
    indicators::jma(&input, period, phase, power)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct MamaResultWasm {
    pub mama: Vec<f64>,
    pub fama: Vec<f64>,
}

#[wasm_bindgen]
pub fn mama(input: Vec<f64>, fast_limit: f64, slow_limit: f64) -> Result<MamaResultWasm, JsError> {
    indicators::mama(&input, fast_limit, slow_limit)
        .map(|r| MamaResultWasm {
            mama: r.mama.into_raw_vec_and_offset().0,
            fama: r.fama.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct SarextResultWasm {
    pub sar: Vec<f64>,
    pub af: Vec<f64>,
}

#[wasm_bindgen]
pub fn sarext(
    high: Vec<f64>,
    low: Vec<f64>,
    start_value: f64,
    offset_on_reverse: f64,
    af_init_long: f64,
    af_long: f64,
    af_max_long: f64,
    af_init_short: f64,
    af_short: f64,
    af_max_short: f64,
) -> Result<SarextResultWasm, JsError> {
    indicators::sarext(
        &high,
        &low,
        start_value,
        offset_on_reverse,
        af_init_long,
        af_long,
        af_max_long,
        af_init_short,
        af_short,
        af_max_short,
    )
    .map(|r| SarextResultWasm {
        sar: r.sar.into_raw_vec_and_offset().0,
        af: r.af.into_raw_vec_and_offset().0,
    })
    .map_err(to_js)
}

#[wasm_bindgen]
pub fn efficiency_ratio(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::efficiency_ratio(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn trima(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    moving_avg::trima(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

// ───────────────────── Extended Momentum Indicators ─────────────────────

#[wasm_bindgen]
pub fn adxr(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::adxr(&high, &low, &close, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn aroonosc(high: Vec<f64>, low: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::aroonosc(&high, &low, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct MacdExtResult {
    pub macd: Vec<f64>,
    pub signal: Vec<f64>,
    pub hist: Vec<f64>,
}

#[wasm_bindgen]
pub fn macdext(
    input: Vec<f64>,
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> Result<MacdExtResult, JsError> {
    indicators::macdext(
        &input,
        fast_period,
        indicators::MaType::Ema,
        slow_period,
        indicators::MaType::Ema,
        signal_period,
        indicators::MaType::Ema,
    )
    .map(|r| MacdExtResult {
        macd: r.macd.into_raw_vec_and_offset().0,
        signal: r.signal.into_raw_vec_and_offset().0,
        hist: r.hist.into_raw_vec_and_offset().0,
    })
    .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct MacdFixResult {
    pub macd: Vec<f64>,
    pub signal: Vec<f64>,
    pub hist: Vec<f64>,
}

#[wasm_bindgen]
pub fn macdfix(input: Vec<f64>) -> Result<MacdFixResult, JsError> {
    indicators::macdfix(&input)
        .map(|r| MacdFixResult {
            macd: r.macd.into_raw_vec_and_offset().0,
            signal: r.signal.into_raw_vec_and_offset().0,
            hist: r.hist.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn ppo(input: Vec<f64>, fast_period: usize, slow_period: usize) -> Result<Vec<f64>, JsError> {
    indicators::ppo(&input, fast_period, slow_period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn rocp(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::rocp(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn rocr(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::rocr(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn rocr100(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::rocr100(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct StochFResult {
    pub k: Vec<f64>,
    pub d: Vec<f64>,
}

#[wasm_bindgen]
pub fn stochf(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    fastk_period: usize,
    fastd_period: usize,
) -> Result<StochFResult, JsError> {
    indicators::stochf(&high, &low, &close, fastk_period, fastd_period)
        .map(|r| StochFResult {
            k: r.k.into_raw_vec_and_offset().0,
            d: r.d.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct StochRsiResult {
    pub k: Vec<f64>,
    pub d: Vec<f64>,
}

#[wasm_bindgen]
pub fn stochrsi(
    input: Vec<f64>,
    rsi_period: usize,
    stoch_period: usize,
    fastk_period: usize,
    fastd_period: usize,
) -> Result<StochRsiResult, JsError> {
    indicators::stochrsi(&input, rsi_period, stoch_period, fastk_period, fastd_period)
        .map(|r| StochRsiResult {
            k: r.k.into_raw_vec_and_offset().0,
            d: r.d.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn ultosc(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period1: usize,
    period2: usize,
    period3: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::ultosc(&high, &low, &close, period1, period2, period3)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn ao(
    high: Vec<f64>,
    low: Vec<f64>,
    fast_period: usize,
    slow_period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::ao(&high, &low, fast_period, slow_period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct FisherResult {
    pub fisher: Vec<f64>,
    pub signal: Vec<f64>,
}

#[wasm_bindgen]
pub fn fisher(high: Vec<f64>, low: Vec<f64>, period: usize) -> Result<FisherResult, JsError> {
    indicators::fisher(&high, &low, period)
        .map(|r| FisherResult {
            fisher: r.fisher.into_raw_vec_and_offset().0,
            signal: r.signal.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn tsi(input: Vec<f64>, long_period: usize, short_period: usize) -> Result<Vec<f64>, JsError> {
    indicators::tsi(&input, long_period, short_period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn coppock(
    input: Vec<f64>,
    roc_period1: usize,
    roc_period2: usize,
    wma_period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::coppock(&input, roc_period1, roc_period2, wma_period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct KstResult {
    pub kst: Vec<f64>,
    pub signal: Vec<f64>,
}

#[wasm_bindgen]
pub fn kst(
    input: Vec<f64>,
    roc1: usize,
    roc2: usize,
    roc3: usize,
    roc4: usize,
    sma1: usize,
    sma2: usize,
    sma3: usize,
    sma4: usize,
    sig_period: usize,
) -> Result<KstResult, JsError> {
    indicators::kst(
        &input, roc1, roc2, roc3, roc4, sma1, sma2, sma3, sma4, sig_period,
    )
    .map(|r| KstResult {
        kst: r.kst.into_raw_vec_and_offset().0,
        signal: r.signal.into_raw_vec_and_offset().0,
    })
    .map_err(to_js)
}

#[wasm_bindgen]
pub fn stc(
    input: Vec<f64>,
    fast_period: usize,
    slow_period: usize,
    cycle: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::stc(&input, fast_period, slow_period, cycle)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn chop(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::chop(&high, &low, &close, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn connors_rsi(
    input: Vec<f64>,
    rsi_period: usize,
    streak_period: usize,
    rank_period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::connors_rsi(&input, rsi_period, streak_period, rank_period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct RviResult {
    pub rvi: Vec<f64>,
    pub signal: Vec<f64>,
}

#[wasm_bindgen]
pub fn rvi(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    open: Vec<f64>,
    period: usize,
) -> Result<RviResult, JsError> {
    indicators::rvi(&high, &low, &close, &open, period)
        .map(|r| RviResult {
            rvi: r.rvi.into_raw_vec_and_offset().0,
            signal: r.signal.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct VortexResult {
    pub plus_vi: Vec<f64>,
    pub minus_vi: Vec<f64>,
}

#[wasm_bindgen]
pub fn vortex(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
) -> Result<VortexResult, JsError> {
    indicators::vortex(&high, &low, &close, period)
        .map(|r| VortexResult {
            plus_vi: r.vi_plus.into_raw_vec_and_offset().0,
            minus_vi: r.vi_minus.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn inertia(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    rvi_period: usize,
    tsf_period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::inertia(&open, &high, &low, &close, rvi_period, tsf_period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct SqueezeMomentumResult {
    pub momentum: Vec<f64>,
    pub squeeze_on: Vec<f64>,
    pub squeeze_off: Vec<f64>,
}

#[wasm_bindgen]
pub fn squeeze_momentum(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    bb_period: usize,
    bb_mult: f64,
    kc_period: usize,
    kc_mult: f64,
) -> Result<SqueezeMomentumResult, JsError> {
    indicators::squeeze_momentum(&high, &low, &close, bb_period, bb_mult, kc_period, kc_mult)
        .map(|r| SqueezeMomentumResult {
            momentum: r.momentum.into_raw_vec_and_offset().0,
            squeeze_on: r.squeeze_on.into_raw_vec_and_offset().0,
            squeeze_off: r.squeeze_off.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn qstick(open: Vec<f64>, close: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::qstick(&open, &close, period, indicators::MaType::Sma)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

// ───────────────────── Classic Chart Patterns (FTA-native) ─────────────────────

#[wasm_bindgen(getter_with_clone)]
pub struct DarvasBoxResult {
    pub box_top: Vec<f64>,
    pub box_bottom: Vec<f64>,
    pub signal: Vec<i32>,
}

#[wasm_bindgen]
pub fn darvas_box(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    lookback: usize,
    confirmation: usize,
) -> Result<DarvasBoxResult, JsError> {
    indicators::darvas_box(&high, &low, &close, lookback, confirmation)
        .map(|r| DarvasBoxResult {
            box_top: r.box_top.into_raw_vec_and_offset().0,
            box_bottom: r.box_bottom.into_raw_vec_and_offset().0,
            signal: r.signal.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct RenkoResult {
    pub bricks: Vec<f64>,
    pub direction: Vec<i32>,
}

#[wasm_bindgen]
pub fn renko(high: Vec<f64>, low: Vec<f64>, box_size: f64) -> Result<RenkoResult, JsError> {
    indicators::renko(&high, &low, box_size)
        .map(|r| RenkoResult {
            bricks: r.bricks.into_raw_vec_and_offset().0,
            direction: r.direction.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct KagiResult {
    pub kagi: Vec<f64>,
    pub direction: Vec<i32>,
}

#[wasm_bindgen]
pub fn kagi(close: Vec<f64>, reversal: f64) -> Result<KagiResult, JsError> {
    indicators::kagi(&close, reversal)
        .map(|r| KagiResult {
            kagi: r.kagi.into_raw_vec_and_offset().0,
            direction: r.direction.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct PnfResult {
    pub pnf: Vec<f64>,
    pub column_type: Vec<i32>,
    pub new_column: Vec<i32>,
}

#[wasm_bindgen]
pub fn point_and_figure(
    high: Vec<f64>,
    low: Vec<f64>,
    box_size: f64,
    reversal: usize,
) -> Result<PnfResult, JsError> {
    indicators::point_and_figure(&high, &low, box_size, reversal)
        .map(|r| PnfResult {
            pnf: r.pnf.into_raw_vec_and_offset().0,
            column_type: r.column_type.into_raw_vec_and_offset().0,
            new_column: r.new_column.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct ThreeLineBreakResult {
    pub line: Vec<f64>,
    pub direction: Vec<i32>,
}

#[wasm_bindgen]
pub fn three_line_break(close: Vec<f64>, lines: usize) -> Result<ThreeLineBreakResult, JsError> {
    indicators::three_line_break(&close, lines)
        .map(|r| ThreeLineBreakResult {
            line: r.line.into_raw_vec_and_offset().0,
            direction: r.direction.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct WilliamsAlligatorResult {
    pub jaw: Vec<f64>,
    pub teeth: Vec<f64>,
    pub lips: Vec<f64>,
}

#[wasm_bindgen]
pub fn williams_alligator(close: Vec<f64>) -> Result<WilliamsAlligatorResult, JsError> {
    indicators::williams_alligator(&close)
        .map(|r| WilliamsAlligatorResult {
            jaw: r.jaw.into_raw_vec_and_offset().0,
            teeth: r.teeth.into_raw_vec_and_offset().0,
            lips: r.lips.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct HeikinAshiResult {
    pub ha_open: Vec<f64>,
    pub ha_high: Vec<f64>,
    pub ha_low: Vec<f64>,
    pub ha_close: Vec<f64>,
}

#[wasm_bindgen]
pub fn heikin_ashi(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
) -> Result<HeikinAshiResult, JsError> {
    indicators::heikin_ashi(&open, &high, &low, &close)
        .map(|r| HeikinAshiResult {
            ha_open: r.ha_open.into_raw_vec_and_offset().0,
            ha_high: r.ha_high.into_raw_vec_and_offset().0,
            ha_low: r.ha_low.into_raw_vec_and_offset().0,
            ha_close: r.ha_close.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn chande_forecast_oscillator(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::chande_forecast_oscillator(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

// ───────────────────── Extended Volatility Indicators ─────────────────────

#[wasm_bindgen]
pub fn mass_index(
    high: Vec<f64>,
    low: Vec<f64>,
    ema_period: usize,
    sum_period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::mass_index(&high, &low, ema_period, sum_period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn ulcer_index(input: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::ulcer_index(&input, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn garman_klass_volatility(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::garman_klass_volatility(&open, &high, &low, &close, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn parkinson_volatility(
    high: Vec<f64>,
    low: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::parkinson_volatility(&high, &low, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn rogers_satchell_volatility(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::rogers_satchell_volatility(&open, &high, &low, &close, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn yang_zhang_volatility(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::yang_zhang_volatility(&open, &high, &low, &close, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn realized_volatility(close: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::realized_volatility(&close, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn semivariance(close: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::semivariance(&close, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn sortino_ratio(
    close: Vec<f64>,
    period: usize,
    risk_free_rate: f64,
) -> Result<Vec<f64>, JsError> {
    indicators::sortino_ratio(&close, period, risk_free_rate)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn calmar_ratio(equity: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::calmar_ratio(&equity, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn information_ratio(
    asset: Vec<f64>,
    benchmark: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::information_ratio(&asset, &benchmark, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn max_drawdown(equity: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::max_drawdown(&equity, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct KeltnerResult {
    pub upper: Vec<f64>,
    pub middle: Vec<f64>,
    pub lower: Vec<f64>,
}

#[wasm_bindgen]
pub fn keltner_channel(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    ema_period: usize,
    atr_period: usize,
    multiplier: f64,
) -> Result<KeltnerResult, JsError> {
    indicators::keltner_channel(&high, &low, &close, ema_period, atr_period, multiplier)
        .map(|r| KeltnerResult {
            upper: r.upper.into_raw_vec_and_offset().0,
            middle: r.middle.into_raw_vec_and_offset().0,
            lower: r.lower.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn adr(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::adr(&high, &low, &close, period, indicators::AdrMode::Absolute)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn chaikin_volatility(
    high: Vec<f64>,
    low: Vec<f64>,
    ema_period: usize,
    roc_period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::chaikin_volatility(&high, &low, ema_period, roc_period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn historical_volatility(
    close: Vec<f64>,
    period: usize,
    annualization: f64,
) -> Result<Vec<f64>, JsError> {
    indicators::historical_volatility(&close, period, annualization)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

// ───────────────────── Extended Volume Indicators ─────────────────────

#[wasm_bindgen]
pub fn cmf(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::cmf(&high, &low, &close, &volume, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn force_index(close: Vec<f64>, volume: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::force_index(&close, &volume, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn eom(
    high: Vec<f64>,
    low: Vec<f64>,
    volume: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::eom(&high, &low, &volume, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct KvoResult {
    pub kvo: Vec<f64>,
    pub signal: Vec<f64>,
}

#[wasm_bindgen]
pub fn kvo(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> Result<KvoResult, JsError> {
    indicators::kvo(
        &high,
        &low,
        &close,
        &volume,
        fast_period,
        slow_period,
        signal_period,
    )
    .map(|r| KvoResult {
        kvo: r.kvo.into_raw_vec_and_offset().0,
        signal: r.signal.into_raw_vec_and_offset().0,
    })
    .map_err(to_js)
}

#[wasm_bindgen]
pub fn nvi(close: Vec<f64>, volume: Vec<f64>) -> Result<Vec<f64>, JsError> {
    indicators::nvi(&close, &volume)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn pvi(close: Vec<f64>, volume: Vec<f64>) -> Result<Vec<f64>, JsError> {
    indicators::pvi(&close, &volume)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen(getter_with_clone)]
pub struct VwmacdResult {
    pub macd: Vec<f64>,
    pub signal: Vec<f64>,
    pub hist: Vec<f64>,
}

#[wasm_bindgen]
pub fn vwmacd(
    close: Vec<f64>,
    volume: Vec<f64>,
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> Result<VwmacdResult, JsError> {
    indicators::vwmacd(&close, &volume, fast_period, slow_period, signal_period)
        .map(|r| VwmacdResult {
            macd: r.macd.into_raw_vec_and_offset().0,
            signal: r.signal.into_raw_vec_and_offset().0,
            hist: r.hist.into_raw_vec_and_offset().0,
        })
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn pvt(close: Vec<f64>, volume: Vec<f64>) -> Result<Vec<f64>, JsError> {
    indicators::pvt(&close, &volume)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn twiggs_money_flow(
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    period: usize,
) -> Result<Vec<f64>, JsError> {
    indicators::twiggs_money_flow(&high, &low, &close, &volume, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn vzo(close: Vec<f64>, volume: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::vzo(&close, &volume, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn volume_momentum(volume: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::volume_momentum(&volume, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

#[wasm_bindgen]
pub fn volume_roc(volume: Vec<f64>, period: usize) -> Result<Vec<f64>, JsError> {
    indicators::volume_roc(&volume, period)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

// ───────────────────── Extended Statistics Indicators ─────────────────────

#[wasm_bindgen]
pub fn var(input: Vec<f64>, period: usize, nb_dev: f64) -> Result<Vec<f64>, JsError> {
    indicators::var(&input, period, nb_dev)
        .map(|a| a.into_raw_vec_and_offset().0)
        .map_err(to_js)
}

// ───────────────────── Streaming API (O(1) per bar) ─────────────────────
//
// A unified streaming facade backed by `finkit::streaming::indicators::*`.
// Each export returns a JS object with a `next(value) -> number | null` method,
// mirroring the Rust `StreamingIndicator` trait. This lets WASM clients compute
// indicators incrementally with O(1) per-bar cost.

#[wasm_bindgen]
pub struct StreamingSmaHandle {
    inner: finkit::streaming::indicators::StreamingSma,
}

#[wasm_bindgen]
impl StreamingSmaHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(period: usize) -> Result<StreamingSmaHandle, JsError> {
        if period == 0 {
            return Err(JsError::new("period must be > 0"));
        }
        Ok(Self {
            inner: finkit::streaming::indicators::StreamingSma::new(period),
        })
    }
    #[wasm_bindgen(js_name = next)]
    pub fn next(&mut self, value: f64) -> Option<f64> {
        finkit::streaming::StreamingIndicator::next(&mut self.inner, value)
    }
    #[wasm_bindgen(js_name = value)]
    pub fn value(&self) -> Option<f64> {
        finkit::streaming::StreamingIndicator::value(&self.inner)
    }
    #[wasm_bindgen(js_name = reset)]
    pub fn reset(&mut self) {
        finkit::streaming::StreamingIndicator::reset(&mut self.inner);
    }
    #[wasm_bindgen(js_name = isReady)]
    pub fn is_ready(&self) -> bool {
        finkit::streaming::StreamingIndicator::is_ready(&self.inner)
    }
}

#[wasm_bindgen]
pub struct StreamingEmaHandle {
    inner: finkit::streaming::indicators::StreamingEma,
}

#[wasm_bindgen]
impl StreamingEmaHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(period: usize) -> Result<StreamingEmaHandle, JsError> {
        if period == 0 {
            return Err(JsError::new("period must be > 0"));
        }
        Ok(Self {
            inner: finkit::streaming::indicators::StreamingEma::new(period),
        })
    }
    #[wasm_bindgen(js_name = next)]
    pub fn next(&mut self, value: f64) -> Option<f64> {
        finkit::streaming::StreamingIndicator::next(&mut self.inner, value)
    }
    #[wasm_bindgen(js_name = value)]
    pub fn value(&self) -> Option<f64> {
        finkit::streaming::StreamingIndicator::value(&self.inner)
    }
    #[wasm_bindgen(js_name = reset)]
    pub fn reset(&mut self) {
        finkit::streaming::StreamingIndicator::reset(&mut self.inner);
    }
    #[wasm_bindgen(js_name = isReady)]
    pub fn is_ready(&self) -> bool {
        finkit::streaming::StreamingIndicator::is_ready(&self.inner)
    }
}

#[wasm_bindgen]
pub struct StreamingRsiHandle {
    inner: finkit::streaming::indicators::StreamingRsi,
}

#[wasm_bindgen]
impl StreamingRsiHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(period: usize) -> Result<StreamingRsiHandle, JsError> {
        if period == 0 {
            return Err(JsError::new("period must be > 0"));
        }
        Ok(Self {
            inner: finkit::streaming::indicators::StreamingRsi::new(period),
        })
    }
    #[wasm_bindgen(js_name = next)]
    pub fn next(&mut self, value: f64) -> Option<f64> {
        finkit::streaming::StreamingIndicator::next(&mut self.inner, value)
    }
    #[wasm_bindgen(js_name = value)]
    pub fn value(&self) -> Option<f64> {
        finkit::streaming::StreamingIndicator::value(&self.inner)
    }
    #[wasm_bindgen(js_name = reset)]
    pub fn reset(&mut self) {
        finkit::streaming::StreamingIndicator::reset(&mut self.inner);
    }
    #[wasm_bindgen(js_name = isReady)]
    pub fn is_ready(&self) -> bool {
        finkit::streaming::StreamingIndicator::is_ready(&self.inner)
    }
}

#[wasm_bindgen]
pub struct StreamingAtrHandle {
    inner: finkit::streaming::indicators::StreamingAtr,
}

#[wasm_bindgen]
impl StreamingAtrHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(period: usize) -> Result<StreamingAtrHandle, JsError> {
        if period == 0 {
            return Err(JsError::new("period must be > 0"));
        }
        Ok(Self {
            inner: finkit::streaming::indicators::StreamingAtr::new(period),
        })
    }
    /// Feed a single bar: (high, low, close)
    pub fn next_bar(&mut self, high: f64, low: f64, close: f64) -> Option<f64> {
        finkit::streaming::StreamingIndicator::next(&mut self.inner, (high, low, close))
    }
    #[wasm_bindgen(js_name = value)]
    pub fn value(&self) -> Option<f64> {
        finkit::streaming::StreamingIndicator::value(&self.inner)
    }
    #[wasm_bindgen(js_name = reset)]
    pub fn reset(&mut self) {
        finkit::streaming::StreamingIndicator::reset(&mut self.inner);
    }
    #[wasm_bindgen(js_name = isReady)]
    pub fn is_ready(&self) -> bool {
        finkit::streaming::StreamingIndicator::is_ready(&self.inner)
    }
}

#[wasm_bindgen]
pub struct StreamingMacdHandle {
    inner: finkit::streaming::indicators::StreamingMacd,
}

#[wasm_bindgen(getter_with_clone)]
pub struct MacdStreamingOutput {
    pub macd: f64,
    pub signal: f64,
    pub hist: f64,
}

#[wasm_bindgen]
impl StreamingMacdHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(fast: usize, slow: usize, signal: usize) -> Result<StreamingMacdHandle, JsError> {
        if fast == 0 || slow == 0 || signal == 0 {
            return Err(JsError::new("fast/slow/signal must be > 0"));
        }
        Ok(Self {
            inner: finkit::streaming::indicators::StreamingMacd::new(fast, slow, signal),
        })
    }
    pub fn next(&mut self, value: f64) -> Option<MacdStreamingOutput> {
        finkit::streaming::StreamingIndicator::next(&mut self.inner, value).map(|o| {
            MacdStreamingOutput {
                macd: o.macd,
                signal: o.signal,
                hist: o.histogram,
            }
        })
    }
    #[wasm_bindgen(js_name = reset)]
    pub fn reset(&mut self) {
        finkit::streaming::StreamingIndicator::reset(&mut self.inner);
    }
    #[wasm_bindgen(js_name = isReady)]
    pub fn is_ready(&self) -> bool {
        finkit::streaming::StreamingIndicator::is_ready(&self.inner)
    }
}

#[wasm_bindgen]
pub struct StreamingBollHandle {
    inner: finkit::streaming::indicators::StreamingBoll,
}

#[wasm_bindgen(getter_with_clone)]
pub struct BollStreamingOutput {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

#[wasm_bindgen]
impl StreamingBollHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(
        period: usize,
        nb_dev_up: f64,
        nb_dev_dn: f64,
    ) -> Result<StreamingBollHandle, JsError> {
        if period == 0 {
            return Err(JsError::new("period must be > 0"));
        }
        Ok(Self {
            inner: finkit::streaming::indicators::StreamingBoll::new(period, nb_dev_up, nb_dev_dn),
        })
    }
    pub fn next(&mut self, value: f64) -> Option<BollStreamingOutput> {
        finkit::streaming::StreamingIndicator::next(&mut self.inner, value).map(|o| {
            BollStreamingOutput {
                upper: o.upper,
                middle: o.middle,
                lower: o.lower,
            }
        })
    }
    #[wasm_bindgen(js_name = reset)]
    pub fn reset(&mut self) {
        finkit::streaming::StreamingIndicator::reset(&mut self.inner);
    }
    #[wasm_bindgen(js_name = isReady)]
    pub fn is_ready(&self) -> bool {
        finkit::streaming::StreamingIndicator::is_ready(&self.inner)
    }
}

#[wasm_bindgen]
pub struct StreamingObvHandle {
    inner: finkit::streaming::indicators::StreamingObv,
}

#[wasm_bindgen]
impl StreamingObvHandle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> StreamingObvHandle {
        Self {
            inner: finkit::streaming::indicators::StreamingObv::new(),
        }
    }
    /// Feed a single bar: (close, volume)
    pub fn next_bar(&mut self, close: f64, volume: f64) -> Option<f64> {
        let bar = finkit::streaming::OhlcvBar::new(0.0, 0.0, 0.0, close, volume);
        finkit::streaming::StreamingIndicator::next(&mut self.inner, &bar)
    }
    #[wasm_bindgen(js_name = reset)]
    pub fn reset(&mut self) {
        finkit::streaming::StreamingIndicator::reset(&mut self.inner);
    }
}

#[wasm_bindgen]
pub struct StreamingVwapHandle {
    inner: finkit::streaming::indicators::StreamingVwap,
}

#[wasm_bindgen]
impl StreamingVwapHandle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> StreamingVwapHandle {
        Self {
            inner: finkit::streaming::indicators::StreamingVwap::new(),
        }
    }
    /// Feed a single bar: (high, low, close, volume)
    pub fn next_bar(&mut self, high: f64, low: f64, close: f64, volume: f64) -> Option<f64> {
        let bar = finkit::streaming::OhlcvBar::new(0.0, high, low, close, volume);
        finkit::streaming::StreamingIndicator::next(&mut self.inner, &bar)
    }
    #[wasm_bindgen(js_name = reset)]
    pub fn reset(&mut self) {
        finkit::streaming::StreamingIndicator::reset(&mut self.inner);
    }
}

#[wasm_bindgen]
pub struct StreamingAdxHandle {
    inner: finkit::streaming::indicators::StreamingAdx,
}

#[wasm_bindgen]
impl StreamingAdxHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(period: usize) -> Result<StreamingAdxHandle, JsError> {
        if period == 0 {
            return Err(JsError::new("period must be > 0"));
        }
        Ok(Self {
            inner: finkit::streaming::indicators::StreamingAdx::new(period),
        })
    }
    /// Feed a single bar: (high, low, close)
    pub fn next_bar(&mut self, high: f64, low: f64, close: f64) -> Option<f64> {
        finkit::streaming::StreamingIndicator::next(&mut self.inner, (high, low, close))
    }
    #[wasm_bindgen(js_name = reset)]
    pub fn reset(&mut self) {
        finkit::streaming::StreamingIndicator::reset(&mut self.inner);
    }
    #[wasm_bindgen(js_name = isReady)]
    pub fn is_ready(&self) -> bool {
        finkit::streaming::StreamingIndicator::is_ready(&self.inner)
    }
}

#[wasm_bindgen]
pub struct StreamingStochHandle {
    inner: finkit::streaming::indicators::StreamingStoch,
}

#[wasm_bindgen(getter_with_clone)]
pub struct StochStreamingOutput {
    pub k: f64,
    pub d: f64,
}

#[wasm_bindgen]
impl StreamingStochHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(
        fast_k: usize,
        slow_k: usize,
        slow_d: usize,
    ) -> Result<StreamingStochHandle, JsError> {
        if fast_k == 0 || slow_k == 0 || slow_d == 0 {
            return Err(JsError::new("periods must be > 0"));
        }
        Ok(Self {
            inner: finkit::streaming::indicators::StreamingStoch::new(fast_k, slow_k, slow_d),
        })
    }
    /// Feed a single bar: (high, low, close)
    pub fn next_bar(&mut self, high: f64, low: f64, close: f64) -> Option<StochStreamingOutput> {
        finkit::streaming::StreamingIndicator::next(&mut self.inner, (high, low, close))
            .map(|o| StochStreamingOutput { k: o.k, d: o.d })
    }
    #[wasm_bindgen(js_name = reset)]
    pub fn reset(&mut self) {
        finkit::streaming::StreamingIndicator::reset(&mut self.inner);
    }
    #[wasm_bindgen(js_name = isReady)]
    pub fn is_ready(&self) -> bool {
        finkit::streaming::StreamingIndicator::is_ready(&self.inner)
    }
}

#[wasm_bindgen]
pub struct StreamingSuperTrendHandle {
    inner: finkit::streaming::indicators::StreamingSuperTrend,
}

#[wasm_bindgen(getter_with_clone)]
pub struct SuperTrendStreamingOutput {
    pub supertrend: f64,
    pub direction: i32,
}

#[wasm_bindgen]
impl StreamingSuperTrendHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(period: usize, multiplier: f64) -> Result<StreamingSuperTrendHandle, JsError> {
        if period == 0 {
            return Err(JsError::new("period must be > 0"));
        }
        Ok(Self {
            inner: finkit::streaming::indicators::StreamingSuperTrend::new(period, multiplier),
        })
    }
    /// Feed a single bar: (high, low, close)
    pub fn next_bar(
        &mut self,
        high: f64,
        low: f64,
        close: f64,
    ) -> Option<SuperTrendStreamingOutput> {
        let bar = finkit::streaming::OhlcvBar::new(0.0, high, low, close, 0.0);
        self.inner.next(&bar).map(|o| SuperTrendStreamingOutput {
            supertrend: o.supertrend,
            direction: o.direction,
        })
    }
    #[wasm_bindgen(js_name = reset)]
    pub fn reset(&mut self) {
        self.inner.reset();
    }
    #[wasm_bindgen(js_name = isReady)]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }
}
