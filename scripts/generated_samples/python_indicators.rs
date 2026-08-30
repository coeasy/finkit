// GENERATED FILE — do not edit by hand.
// Source of truth: docs/indicator_registry.json (ffi block).
// Regenerate with: python3 scripts/gen_binding.py --lang python --generate <path>

use pyo3::prelude::*;
use finkit::indicators;
use finkit::math::moving_avg;
use finkit::patterns::candlestick;

struct AlphaTaGenerated;

#[pymethods]
impl AlphaTaGenerated {
    #[pyo3(text_signature = "(input: Vec<f64>, period: i32)")]
    fn sma(&self, input: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = moving_avg::sma(&input, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32)")]
    fn ema(&self, input: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = moving_avg::ema(&input, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32)")]
    fn wma(&self, input: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = moving_avg::wma(&input, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32)")]
    fn dema(&self, input: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = moving_avg::dema(&input, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32)")]
    fn tema(&self, input: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = moving_avg::tema(&input, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32)")]
    fn kama(&self, input: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = moving_avg::kama(&input, period as usize, 2, 30).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, fast_limit: i32, slow_limit: i32)")]
    fn mama(&self, input: Vec<f64>, fast_limit: i32, slow_limit: i32) -> PyResult<Vec<f64>> {
        let result = indicators::mama(&input, fast_limit, slow_limit).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok((result.mama.into_raw_vec(), result.fama.into_raw_vec()))
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32, vfactor: i32)")]
    fn t3(&self, input: Vec<f64>, period: i32, vfactor: i32) -> PyResult<Vec<f64>> {
        let result = indicators::t3(&input, period as usize, vfactor).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32, nbdevup: i32, nbdevdn: i32)")]
    fn bbands(&self, input: Vec<f64>, period: i32, nbdevup: i32, nbdevdn: i32) -> PyResult<Vec<f64>> {
        let result = indicators::bbands(&input, period as usize, nbdevup, nbdevdn).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok((result.upper.into_raw_vec(), result.middle.into_raw_vec(), result.lower.into_raw_vec()))
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32)")]
    fn midpoint(&self, input: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::midpoint(&input, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, period: i32)")]
    fn midprice(&self, high: Vec<f64>, low: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::midprice(&high, &low, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, acceleration: i32, maximum: i32)")]
    fn sar(&self, high: Vec<f64>, low: Vec<f64>, acceleration: i32, maximum: i32) -> PyResult<Vec<f64>> {
        let result = indicators::sar(&high, &low, acceleration, maximum).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok((result.sar.into_raw_vec()))
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32)")]
    fn rsi(&self, input: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::rsi(&input, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, fast_period: i32, slow_period: i32, signal_period: i32)")]
    fn macd(&self, input: Vec<f64>, fast_period: i32, slow_period: i32, signal_period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::macd(&input, fast_period as usize, slow_period as usize, signal_period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok((result.macd.into_raw_vec(), result.signal.into_raw_vec(), result.hist.into_raw_vec()))
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, fastk_period: i32, slowk_period: i32, slowd_period: i32)")]
    fn stoch(&self, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, fastk_period: i32, slowk_period: i32, slowd_period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::stoch(&high, &low, &close, fastk_period as usize, slowk_period as usize, slowd_period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok((result.k.into_raw_vec(), result.d.into_raw_vec()))
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32)")]
    fn adx(&self, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::adx(&high, &low, &close, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, period: i32)")]
    fn aroon(&self, high: Vec<f64>, low: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::aroon(&high, &low, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok((result.aroon_up.into_raw_vec(), result.aroon_down.into_raw_vec()))
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32)")]
    fn cci(&self, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::cci(&high, &low, &close, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32)")]
    fn mom(&self, input: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::mom(&input, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32)")]
    fn roc(&self, input: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::roc(&input, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32)")]
    fn willr(&self, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::willr(&high, &low, &close, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, fast_period: i32, slow_period: i32)")]
    fn apo(&self, input: Vec<f64>, fast_period: i32, slow_period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::apo(&input, fast_period as usize, slow_period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>)")]
    fn bop(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> PyResult<Vec<f64>> {
        let result = indicators::bop(&open, &high, &low, &close).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32)")]
    fn cmo(&self, input: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::cmo(&input, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, volume: Vec<f64>, period: i32)")]
    fn mfi(&self, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, volume: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::mfi(&high, &low, &close, &volume, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32)")]
    fn trix(&self, input: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::trix(&input, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32)")]
    fn vortex(&self, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::vortex(&high, &low, &close, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok((result.vi_plus.into_raw_vec(), result.vi_minus.into_raw_vec()))
    }

    #[pyo3(text_signature = "(close: Vec<f64>, volume: Vec<f64>, period: i32)")]
    fn vzo(&self, close: Vec<f64>, volume: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::vzo(&close, &volume, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(volume: Vec<f64>, period: i32)")]
    fn volume_momentum(&self, volume: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::volume_momentum(&volume, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(volume: Vec<f64>, period: i32)")]
    fn volume_roc(&self, volume: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::volume_roc(&volume, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(close: Vec<f64>, period: i32)")]
    fn chande_forecast(&self, close: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::chande_forecast_oscillator(&close, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, volume: Vec<f64>, period: i32)")]
    fn twiggs_mf(&self, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, volume: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::twiggs_money_flow(&high, &low, &close, &volume, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, rvi_period: i32, linreg_period: i32)")]
    fn inertia(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, rvi_period: i32, linreg_period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::inertia(&open, &high, &low, &close, rvi_period as usize, linreg_period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32)")]
    fn atr(&self, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::atr(&high, &low, &close, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32)")]
    fn natr(&self, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::natr(&high, &low, &close, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>)")]
    fn trange(&self, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> PyResult<Vec<f64>> {
        let result = indicators::trange(&high, &low, &close).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(close: Vec<f64>, volume: Vec<f64>)")]
    fn obv(&self, close: Vec<f64>, volume: Vec<f64>) -> PyResult<Vec<f64>> {
        let result = indicators::obv(&close, &volume).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, volume: Vec<f64>)")]
    fn ad(&self, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, volume: Vec<f64>) -> PyResult<Vec<f64>> {
        let result = indicators::ad(&high, &low, &close, &volume).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, volume: Vec<f64>, fast_period: i32, slow_period: i32)")]
    fn adosc(&self, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, volume: Vec<f64>, fast_period: i32, slow_period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::adosc(&high, &low, &close, &volume, fast_period as usize, slow_period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>)")]
    fn ht_dcperiod(&self, input: Vec<f64>) -> PyResult<Vec<f64>> {
        let result = indicators::ht_dcperiod(&input).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>)")]
    fn ht_dcphase(&self, input: Vec<f64>) -> PyResult<Vec<f64>> {
        let result = indicators::ht_dcphase(&input).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>)")]
    fn ht_phasor(&self, input: Vec<f64>) -> PyResult<Vec<f64>> {
        let result = indicators::ht_phasor(&input).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>)")]
    fn ht_sine(&self, input: Vec<f64>) -> PyResult<Vec<f64>> {
        let result = indicators::ht_sine(&input).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>)")]
    fn ht_trendmode(&self, input: Vec<f64>) -> PyResult<Vec<f64>> {
        let result = indicators::ht_trendmode(&input).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>)")]
    fn ht_trendline(&self, input: Vec<f64>) -> PyResult<Vec<f64>> {
        let result = indicators::ht_trendline(&input).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32)")]
    fn zscore(&self, input: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::zscore(&input, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(asset: Vec<f64>, benchmark: Vec<f64>, period: i32)")]
    fn beta(&self, asset: Vec<f64>, benchmark: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::beta(&asset, &benchmark, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input_a: Vec<f64>, input_b: Vec<f64>, period: i32)")]
    fn correlation(&self, input_a: Vec<f64>, input_b: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::correlation(&input_a, &input_b, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32, nb_dev: i32)")]
    fn stddev(&self, input: Vec<f64>, period: i32, nb_dev: i32) -> PyResult<Vec<f64>> {
        let result = indicators::std_dev(&input, period as usize, nb_dev).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32)")]
    fn tsf(&self, input: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::tsf(&input, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32)")]
    fn linear_reg(&self, input: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::linear_reg(&input, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(input: Vec<f64>, period: i32)")]
    fn percent_rank(&self, input: Vec<f64>, period: i32) -> PyResult<Vec<f64>> {
        let result = indicators::percent_rank(&input, period as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>)")]
    fn avgprice(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> PyResult<Vec<f64>> {
        let result = indicators::avgprice(&open, &high, &low, &close).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>)")]
    fn medprice(&self, high: Vec<f64>, low: Vec<f64>) -> PyResult<Vec<f64>> {
        let result = indicators::medprice(&high, &low).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>)")]
    fn typprice(&self, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> PyResult<Vec<f64>> {
        let result = indicators::typprice(&high, &low, &close).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>)")]
    fn wclprice(&self, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> PyResult<Vec<f64>> {
        let result = indicators::wclprice(&high, &low, &close).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, doji_pct: i32)")]
    fn cdl_doji(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, doji_pct: i32) -> PyResult<Vec<i32>> {
        let result = candlestick::doji(&open, &high, &low, &close, doji_pct).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, doji_pct: i32)")]
    fn cdl_dragonfly_doji(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, doji_pct: i32) -> PyResult<Vec<i32>> {
        let result = candlestick::dragonfly_doji(&open, &high, &low, &close, doji_pct).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, doji_pct: i32)")]
    fn cdl_gravestone_doji(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, doji_pct: i32) -> PyResult<Vec<i32>> {
        let result = candlestick::gravestone_doji(&open, &high, &low, &close, doji_pct).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, doji_pct: i32)")]
    fn cdl_long_legged_doji(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, doji_pct: i32) -> PyResult<Vec<i32>> {
        let result = candlestick::long_legged_doji(&open, &high, &low, &close, doji_pct).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>)")]
    fn cdl_hammer(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> PyResult<Vec<i32>> {
        let result = candlestick::hammer(&open, &high, &low, &close).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>)")]
    fn cdl_inverted_hammer(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> PyResult<Vec<i32>> {
        let result = candlestick::inverted_hammer(&open, &high, &low, &close).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>)")]
    fn cdl_hanging_man(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> PyResult<Vec<i32>> {
        let result = candlestick::hanging_man(&open, &high, &low, &close).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>)")]
    fn cdl_shooting_star(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> PyResult<Vec<i32>> {
        let result = candlestick::shooting_star(&open, &high, &low, &close).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>)")]
    fn cdl_engulfing(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> PyResult<Vec<i32>> {
        let result = candlestick::engulfing(&open, &high, &low, &close).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>)")]
    fn cdl_harami(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> PyResult<Vec<i32>> {
        let result = candlestick::harami(&open, &high, &low, &close).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>)")]
    fn cdl_morning_star(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> PyResult<Vec<i32>> {
        let result = candlestick::morning_star(&open, &high, &low, &close).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>)")]
    fn cdl_evening_star(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> PyResult<Vec<i32>> {
        let result = candlestick::evening_star(&open, &high, &low, &close).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>)")]
    fn cdl_three_white_soldiers(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> PyResult<Vec<i32>> {
        let result = candlestick::three_white_soldiers(&open, &high, &low, &close).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>)")]
    fn cdl_three_black_crows(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> PyResult<Vec<i32>> {
        let result = candlestick::three_black_crows(&open, &high, &low, &close).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, shadow_pct: i32)")]
    fn cdl_marubozu(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, shadow_pct: i32) -> PyResult<Vec<i32>> {
        let result = candlestick::marubozu(&open, &high, &low, &close, shadow_pct).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, lookback: i32, confirmation: i32)")]
    fn darvas_box(&self, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>, lookback: i32, confirmation: i32) -> PyResult<Vec<i32>> {
        let result = indicators::darvas_box(&high, &low, &close, lb, conf) {
        Ok(r) => {
            if !out_top.is_null() {
                copy_result(out_top, &r.box_top, len);
            }
            if !out_bottom.is_null() {
                copy_result(out_bottom, &r.box_bottom, len);
            }
            if !out_signal.is_null( as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, box_size: i32)")]
    fn renko(&self, high: Vec<f64>, low: Vec<f64>, box_size: i32) -> PyResult<Vec<i32>> {
        let result = indicators::renko(&high, &low, box_size) {
        Ok(r) => {
            copy_result(out_bricks, &r.bricks, len);
            if !out_dir.is_null( as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(close: Vec<f64>, reversal: i32)")]
    fn kagi(&self, close: Vec<f64>, reversal: i32) -> PyResult<Vec<i32>> {
        let result = indicators::kagi(&close, reversal) {
        Ok(r) => {
            copy_result(out_kagi, &r.kagi, len);
            if !out_dir.is_null( as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(high: Vec<f64>, low: Vec<f64>, box_size: i32, reversal: i32)")]
    fn point_and_figure(&self, high: Vec<f64>, low: Vec<f64>, box_size: i32, reversal: i32) -> PyResult<Vec<i32>> {
        let result = indicators::point_and_figure(&high, &low, box_size, rev) {
        Ok(r) => {
            copy_result(out_pnf, &r.pnf, len);
            if !out_col.is_null() {
                copy_int_result(out_col, &r.column_type, len);
            }
            if !out_new.is_null( as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(close: Vec<f64>, lines: i32)")]
    fn three_line_break(&self, close: Vec<f64>, lines: i32) -> PyResult<Vec<i32>> {
        let result = indicators::three_line_break(&close, n) {
        Ok(r) => {
            copy_result(out_line, &r.line, len);
            if !out_dir.is_null( as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(close: Vec<f64>)")]
    fn williams_alligator(&self, close: Vec<f64>) -> PyResult<Vec<f64>> {
        let result = indicators::williams_alligator(&close).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }

    #[pyo3(text_signature = "(open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>)")]
    fn heikin_ashi(&self, open: Vec<f64>, high: Vec<f64>, low: Vec<f64>, close: Vec<f64>) -> PyResult<Vec<f64>> {
        let result = indicators::heikin_ashi(&open, &high, &low, c) {
        Ok(r) => {
            if !out_o.is_null() {
                copy_result(out_o, &r.ha_open, len);
            }
            if !out_h.is_null() {
                copy_result(out_h, &r.ha_high, len);
            }
            if !out_l.is_null() {
                copy_result(out_l, &r.ha_low, len);
            }
            if !out_c.is_null( as usize).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(result.into_raw_vec())
    }
}
