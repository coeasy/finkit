//! TA-Lib compatibility layer and the indicator dispatcher's inputs.

use super::*;

#[derive(Debug, Clone)]
pub struct IndicatorRequest {
    pub name: String,
    pub params: Vec<f64>,
}

pub fn parse_indicator_requests(requests: Vec<(String, Vec<f64>)>) -> Vec<IndicatorRequest> {
    requests
        .into_iter()
        .map(|(name, params)| IndicatorRequest { name, params })
        .collect()
}

#[pyfunction]
#[pyo3(signature = (close, requests, open=None, high=None, low=None, volume=None, secondary=None, talib_compat=false))]
pub fn compute_indicators<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'_, f64>,
    requests: Vec<(String, Vec<f64>)>,
    open: Option<PyReadonlyArray1<'_, f64>>,
    high: Option<PyReadonlyArray1<'_, f64>>,
    low: Option<PyReadonlyArray1<'_, f64>>,
    volume: Option<PyReadonlyArray1<'_, f64>>,
    secondary: Option<PyReadonlyArray1<'_, f64>>,
    talib_compat: bool,
) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
    let close_slice = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;

    let open_slice = open
        .as_ref()
        .map(|arr| arr.as_slice())
        .transpose()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high_slice = high
        .as_ref()
        .map(|arr| arr.as_slice())
        .transpose()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low_slice = low
        .as_ref()
        .map(|arr| arr.as_slice())
        .transpose()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let volume_slice = volume
        .as_ref()
        .map(|arr| arr.as_slice())
        .transpose()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let secondary_slice = secondary
        .as_ref()
        .map(|arr| arr.as_slice())
        .transpose()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;

    let indicator_requests = parse_indicator_requests(requests);

    let results: Vec<(String, IndicatorResult)> = py.detach(|| {
        compute_all_indicators(
            open_slice,
            high_slice,
            low_slice,
            close_slice,
            volume_slice,
            secondary_slice,
            &indicator_requests,
            talib_compat,
        )
    });

    let dict = pyo3::types::PyDict::new(py);
    for (key, value) in results {
        match value {
            IndicatorResult::Single(arr) => {
                dict.set_item(key, PyArray1::from_vec(py, arr))?;
            }
            IndicatorResult::Double(a, b) => {
                dict.set_item(format!("{}_0", key), PyArray1::from_vec(py, a))?;
                dict.set_item(format!("{}_1", key), PyArray1::from_vec(py, b))?;
            }
            IndicatorResult::Triple(a, b, c) => {
                dict.set_item(format!("{}_0", key), PyArray1::from_vec(py, a))?;
                dict.set_item(format!("{}_1", key), PyArray1::from_vec(py, b))?;
                dict.set_item(format!("{}_2", key), PyArray1::from_vec(py, c))?;
            }
            IndicatorResult::Quad(a, b, c, d) => {
                dict.set_item(format!("{}_0", key), PyArray1::from_vec(py, a))?;
                dict.set_item(format!("{}_1", key), PyArray1::from_vec(py, b))?;
                dict.set_item(format!("{}_2", key), PyArray1::from_vec(py, c))?;
                dict.set_item(format!("{}_3", key), PyArray1::from_vec(py, d))?;
            }
            IndicatorResult::Error(msg) => {
                dict.set_item(format!("{}_error", key), msg)?;
            }
        }
    }
    Ok(dict)
}

pub enum IndicatorResult {
    Single(Vec<f64>),
    Double(Vec<f64>, Vec<f64>),
    Triple(Vec<f64>, Vec<f64>, Vec<f64>),
    Quad(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>),
    Error(String),
}

pub fn compute_all_indicators(
    open: Option<&[f64]>,
    high: Option<&[f64]>,
    low: Option<&[f64]>,
    close: &[f64],
    volume: Option<&[f64]>,
    secondary: Option<&[f64]>,
    requests: &[IndicatorRequest],
    talib_compat: bool,
) -> Vec<(String, IndicatorResult)> {
    let mut results = Vec::with_capacity(requests.len());

    for req in requests {
        let key = format!(
            "{}_{}",
            req.name,
            req.params
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join("_")
        );
        let result = if talib_compat && req.name.eq_ignore_ascii_case("ppo") {
            talib_percentage_oscillator(close, &req.params)
        } else if talib_compat && req.name.eq_ignore_ascii_case("macdfix") {
            talib_macdfix(close, &req.params)
        } else if talib_compat && req.name.eq_ignore_ascii_case("stochrsi") {
            talib_stochrsi(close, &req.params)
        } else if talib_compat && req.name.eq_ignore_ascii_case("beta") {
            match secondary {
                Some(other) => talib_beta(close, other, &req.params),
                None => IndicatorResult::Error("BETA requires secondary data".to_string()),
            }
        } else if talib_compat
            && matches!(
                req.name.to_ascii_lowercase().as_str(),
                "plus_dm" | "minus_dm"
            )
        {
            match (high, low) {
                (Some(high), Some(low)) => talib_directional_movement(
                    high,
                    low,
                    params_first_or(req, 14),
                    req.name.eq_ignore_ascii_case("plus_dm"),
                ),
                _ => IndicatorResult::Error(format!(
                    "{} requires high and low data",
                    req.name.to_ascii_uppercase()
                )),
            }
        } else {
            compute_single_indicator(open, high, low, close, volume, secondary, req)
        };
        let result = if talib_compat {
            apply_talib_compatibility(&req.name, &req.params, result)
        } else {
            result
        };
        results.push((key, result));
    }

    results
}

pub fn talib_percentage_oscillator(input: &[f64], params: &[f64]) -> IndicatorResult {
    let fast = params.first().copied().unwrap_or(12.0).max(1.0) as usize;
    let slow = params.get(1).copied().unwrap_or(26.0).max(1.0) as usize;
    let ma_type = params.get(2).copied().unwrap_or(0.0) as usize;
    let kind = match ma_type {
        0 => indicators::MaType::Sma,
        1 => indicators::MaType::Ema,
        2 => indicators::MaType::Wma,
        3 => indicators::MaType::Dema,
        4 => indicators::MaType::Tema,
        5 => indicators::MaType::Trima,
        6 => indicators::MaType::Kama,
        8 => indicators::MaType::T3,
        _ => indicators::MaType::Ema,
    };
    match (
        indicators::ma(input, fast, kind),
        indicators::ma(input, slow, kind),
    ) {
        (Ok(fast_ma), Ok(slow_ma)) => {
            let mut output = vec![f64::NAN; input.len()];
            for i in 0..input.len() {
                if fast_ma[i].is_finite() && slow_ma[i].is_finite() && slow_ma[i].abs() > 1e-15 {
                    output[i] = (fast_ma[i] - slow_ma[i]) / slow_ma[i] * 100.0;
                }
            }
            IndicatorResult::Single(output)
        }
        (Err(error), _) | (_, Err(error)) => IndicatorResult::Error(error.to_string()),
    }
}

pub fn talib_macdfix(input: &[f64], params: &[f64]) -> IndicatorResult {
    let signal_period = params.first().copied().unwrap_or(9.0).max(1.0) as usize;
    let lookback_signal = signal_period.saturating_sub(1);
    let lookback_total = 25 + lookback_signal;
    let mut macd = vec![f64::NAN; input.len()];
    let mut signal = vec![f64::NAN; input.len()];
    let mut hist = vec![f64::NAN; input.len()];
    if input.len() <= lookback_total {
        return IndicatorResult::Triple(macd, signal, hist);
    }

    let fast_k = 0.15;
    let slow_k = 0.075;
    let signal_k = 2.0 / (signal_period as f64 + 1.0);
    let mut today = 0usize;
    let mut slow_seed = 0.0;
    for _ in 0..(26 - 12) {
        slow_seed += input[today];
        today += 1;
    }
    let mut fast = 0.0;
    for _ in 0..12 {
        fast += input[today];
        slow_seed += input[today];
        today += 1;
    }
    let mut slow = slow_seed / 26.0;
    fast /= 12.0;

    // The first MACD value is the 26-bar seed endpoint (index 25). Advance
    // both fixed EMAs through the leading stable period before seeding the
    // signal line.
    let mut macd_value = fast - slow;
    let signal_seed_end = lookback_total - lookback_signal;
    while today <= signal_seed_end.saturating_sub(1) {
        let value = input[today];
        today += 1;
        fast = (value - fast).mul_add(fast_k, fast);
        slow = (value - slow).mul_add(slow_k, slow);
        macd_value = fast - slow;
    }

    let mut signal_value = macd_value;
    for _ in 1..signal_period {
        let value = input[today];
        today += 1;
        fast = (value - fast).mul_add(fast_k, fast);
        slow = (value - slow).mul_add(slow_k, slow);
        macd_value = fast - slow;
        signal_value += macd_value;
    }
    signal_value /= signal_period as f64;

    // Advance to the first public output bar. For signal_period=1 this loop
    // is intentionally empty and the signal equals the MACD line.
    while today <= lookback_total {
        let value = input[today];
        today += 1;
        fast = (value - fast).mul_add(fast_k, fast);
        slow = (value - slow).mul_add(slow_k, slow);
        macd_value = fast - slow;
        signal_value = if signal_period == 1 {
            macd_value
        } else {
            (macd_value - signal_value).mul_add(signal_k, signal_value)
        };
    }

    let mut index = lookback_total;
    // SAFETY-TERMINATION: `today` strictly increases each pass and the loop
    // breaks once it reaches `input.len()`.
    loop {
        macd[index] = macd_value;
        signal[index] = signal_value;
        hist[index] = macd_value - signal_value;
        if today >= input.len() {
            break;
        }
        let value = input[today];
        today += 1;
        fast = (value - fast).mul_add(fast_k, fast);
        slow = (value - slow).mul_add(slow_k, slow);
        macd_value = fast - slow;
        signal_value = if signal_period == 1 {
            macd_value
        } else {
            (macd_value - signal_value).mul_add(signal_k, signal_value)
        };
        index += 1;
    }
    IndicatorResult::Triple(macd, signal, hist)
}

pub fn talib_beta(market: &[f64], security: &[f64], params: &[f64]) -> IndicatorResult {
    let period = params.first().copied().unwrap_or(5.0).max(1.0) as usize;
    if market.len() != security.len() {
        return IndicatorResult::Error("BETA inputs must have equal lengths".to_string());
    }
    let mut output = vec![f64::NAN; market.len()];
    if period == 0 || market.len() <= period {
        return IndicatorResult::Single(output);
    }

    for i in period..market.len() {
        let start = i + 1 - period;
        let mut returns_x = Vec::with_capacity(period);
        let mut returns_y = Vec::with_capacity(period);
        for j in start..=i {
            let x = if market[j - 1] != 0.0 {
                (market[j] - market[j - 1]) / market[j - 1]
            } else {
                0.0
            };
            let y = if security[j - 1] != 0.0 {
                (security[j] - security[j - 1]) / security[j - 1]
            } else {
                0.0
            };
            returns_x.push(x);
            returns_y.push(y);
        }
        let shift_x = returns_x[0];
        let shift_y = returns_y[0];
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xx = 0.0;
        let mut sum_xy = 0.0;
        for (x, y) in returns_x.iter().zip(returns_y.iter()) {
            let x = *x - shift_x;
            let y = *y - shift_y;
            sum_x += x;
            sum_y += y;
            sum_xx += x * x;
            sum_xy += x * y;
        }
        let n = period as f64;
        let denominator = n * sum_xx - sum_x * sum_x;
        if denominator > 1e-14 * (n * sum_xx).abs() {
            output[i] = (n * sum_xy - sum_x * sum_y) / denominator;
        } else {
            output[i] = 0.0;
        }
    }
    IndicatorResult::Single(output)
}

pub fn talib_stochrsi(input: &[f64], params: &[f64]) -> IndicatorResult {
    let rsi_period = params.first().copied().unwrap_or(14.0).max(1.0) as usize;
    let fastk_period = params.get(1).copied().unwrap_or(5.0).max(1.0) as usize;
    let fastd_period = params.get(2).copied().unwrap_or(3.0).max(1.0) as usize;
    let rsi = match indicators::rsi(input, rsi_period) {
        Ok(values) => values,
        Err(error) => return IndicatorResult::Error(error.to_string()),
    };
    let rsi = rsi.as_slice().unwrap_or(&[]);
    let len = input.len();
    let raw_start = rsi_period + fastk_period - 1;
    let public_start = raw_start + fastd_period - 1;
    let mut raw = vec![f64::NAN; len];
    for i in raw_start..len {
        let window = &rsi[i + 1 - fastk_period..=i];
        if window.iter().all(|value| value.is_finite()) {
            let lowest = window.iter().copied().fold(f64::INFINITY, f64::min);
            let highest = window.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let range = highest - lowest;
            raw[i] = if range.abs() > 1e-15 {
                (rsi[i] - lowest) / range * 100.0
            } else {
                0.0
            };
        }
    }

    let mut out_k = vec![f64::NAN; len];
    let mut out_d = vec![f64::NAN; len];
    for i in public_start..len {
        out_k[i] = raw[i];
        let window = &raw[i + 1 - fastd_period..=i];
        if window.iter().all(|value| value.is_finite()) {
            out_d[i] = window.iter().sum::<f64>() / fastd_period as f64;
        }
    }
    IndicatorResult::Double(out_k, out_d)
}

pub fn params_first_or(req: &IndicatorRequest, default: usize) -> usize {
    req.params
        .first()
        .copied()
        .unwrap_or(default as f64)
        .max(1.0) as usize
}

pub fn talib_directional_movement(
    high: &[f64],
    low: &[f64],
    period: usize,
    plus: bool,
) -> IndicatorResult {
    if high.len() != low.len() || high.len() < period {
        return IndicatorResult::Error(
            "high and low must have equal lengths and contain at least one period".to_string(),
        );
    }
    let mut raw = vec![0.0; high.len()];
    for i in 1..high.len() {
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];
        raw[i] = if plus {
            if up_move > 0.0 && up_move > down_move {
                up_move
            } else {
                0.0
            }
        } else if down_move > 0.0 && down_move > up_move {
            down_move
        } else {
            0.0
        };
    }

    let mut output = vec![f64::NAN; high.len()];
    let first = period - 1;
    let mut smooth: f64 = raw[1..=first].iter().sum();
    output[first] = smooth;
    let inv_period = 1.0 / period as f64;
    for i in (first + 1)..high.len() {
        smooth = smooth - smooth * inv_period + raw[i];
        output[i] = smooth;
    }
    IndicatorResult::Single(output)
}

pub fn apply_talib_compatibility(
    name: &str,
    params: &[f64],
    result: IndicatorResult,
) -> IndicatorResult {
    let name = name.to_ascii_lowercase();
    if matches!(result, IndicatorResult::Error(_)) {
        return result;
    }

    let period = |index: usize, default: usize| {
        params
            .get(index)
            .copied()
            .unwrap_or(default as f64)
            .max(1.0) as usize
    };
    let lookback = match name.as_str() {
        "kama" => period(0, 10),
        "mama" => 32,
        "mavp" => period(1, 30).saturating_sub(1),
        "sar" | "sarext" => 1,
        "t3" => 6 * period(0, 5).saturating_sub(1),
        "dema" => 2 * period(0, 30).saturating_sub(1),
        "tema" => 3 * period(0, 30).saturating_sub(1),
        "ht_dcphase" | "ht_sine" | "ht_trendmode" | "ht_trendline" => 63,
        "adx" => 2 * period(0, 14).saturating_sub(1),
        "adxr" => 3 * period(0, 14).saturating_sub(1),
        "apo" | "ppo" => period(1, 26).saturating_sub(1),
        "aroon" | "aroonosc" => period(0, 14),
        "cmo" | "rsi" => period(0, 14),
        "macd" | "macdext" | "macdfix" => {
            let slow = if name == "macdfix" {
                26
            } else if name == "macdext" {
                period(2, 26)
            } else {
                period(1, 26)
            };
            let signal = if name == "macdfix" {
                9
            } else if name == "macdext" {
                period(4, 9)
            } else {
                period(2, 9)
            };
            slow + signal - 2
        }
        "stoch" => period(0, 5) + period(1, 3) + period(3, 3) - 3,
        "stochf" => period(0, 5) + period(1, 3) - 2,
        "stochrsi" => period(0, 14) + period(1, 5) + period(2, 3) - 2,
        "trix" => 3 * period(0, 30) - 2,
        "ultosc" => period(2, 28),
        "atr" | "natr" => period(0, 14),
        "trange" => 1,
        "adosc" => period(1, 10).saturating_sub(1),
        "beta" => period(0, 5),
        "correl"
        | "correlation"
        | "linearreg"
        | "linear_reg"
        | "linearreg_angle"
        | "linearreg_intercept"
        | "linearreg_slope"
        | "stddev"
        | "std_dev"
        | "tsf"
        | "var"
        | "max"
        | "min"
        | "minmax"
        | "sum"
        | "accbands"
        | "avgdev"
        | "imi" => period(0, 30).saturating_sub(1),
        "maxindex" | "minindex" | "minmaxindex" => 0,
        _ => 0,
    };

    // The native SAR result also carries its acceleration-factor trace. The
    // TA-Lib public function exposes only the SAR series.
    let result = if name == "sar" {
        match result {
            IndicatorResult::Double(sar, _) => IndicatorResult::Single(sar),
            other => other,
        }
    } else {
        result
    };

    match name.as_str() {
        "maxindex" | "minindex" | "minmaxindex" => {
            let p = period(0, 30);
            fn absolute_index(values: &mut [f64], period: usize) {
                for (i, value) in values.iter_mut().enumerate() {
                    if i < period.saturating_sub(1) || *value < 0.0 {
                        *value = 0.0;
                    } else {
                        *value += (i + 1 - period) as f64;
                    }
                }
            }
            match result {
                IndicatorResult::Single(mut values) => {
                    absolute_index(&mut values, p);
                    IndicatorResult::Single(values)
                }
                IndicatorResult::Double(mut first, mut second) => {
                    absolute_index(&mut first, p);
                    absolute_index(&mut second, p);
                    IndicatorResult::Double(first, second)
                }
                other => other,
            }
        }
        "aroon" => match result {
            IndicatorResult::Double(mut up, mut down) => {
                let end = lookback.min(up.len()).min(down.len());
                up[..end].fill(f64::NAN);
                down[..end].fill(f64::NAN);
                // TA-Lib returns (aroondown, aroonup), while the native
                // finkit result is (aroonup, aroondown).
                IndicatorResult::Double(down, up)
            }
            other => other,
        },
        "ht_trendmode" => match result {
            IndicatorResult::Single(mut values) => {
                let end = lookback.min(values.len());
                values[..end].fill(0.0);
                IndicatorResult::Single(values)
            }
            other => other,
        },
        _ => {
            fn mask(values: &mut [f64], lookback: usize) {
                let end = lookback.min(values.len());
                values[..end].fill(f64::NAN);
            }
            match result {
                IndicatorResult::Single(mut values) => {
                    mask(&mut values, lookback);
                    IndicatorResult::Single(values)
                }
                IndicatorResult::Double(mut first, mut second) => {
                    mask(&mut first, lookback);
                    mask(&mut second, lookback);
                    IndicatorResult::Double(first, second)
                }
                IndicatorResult::Triple(mut first, mut second, mut third) => {
                    mask(&mut first, lookback);
                    mask(&mut second, lookback);
                    mask(&mut third, lookback);
                    IndicatorResult::Triple(first, second, third)
                }
                IndicatorResult::Quad(mut first, mut second, mut third, mut fourth) => {
                    mask(&mut first, lookback);
                    mask(&mut second, lookback);
                    mask(&mut third, lookback);
                    mask(&mut fourth, lookback);
                    IndicatorResult::Quad(first, second, third, fourth)
                }
                other => other,
            }
        }
    }
}

pub fn pattern_result(result: ::finkit::Result<candlestick::PatternResult>) -> IndicatorResult {
    result
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
