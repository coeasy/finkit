//! TA-Lib 0.8 profile indicators whose names or contracts are distinct from
//! the existing formula-oriented indicators.
//!
//! These functions deliberately keep TA-Lib's input/output shape and warm-up
//! rules.  They are used by the cross-language profile dispatcher as well as
//! the Rust API; formula-system aliases must not silently change their meaning.

use crate::error::{Result, TaError};
use crate::math::moving_avg::{ema, sma};
use crate::utils::{init_output, validate_input, validate_param};
use ndarray::Array1;

#[derive(Debug, Clone)]
pub struct EriResult {
    pub bullpower: Array1<f64>,
    pub bearpower: Array1<f64>,
}
#[derive(Debug, Clone)]
pub struct FractalResult {
    pub swinghigh: Array1<f64>,
    pub swinglow: Array1<f64>,
}
#[derive(Debug, Clone)]
pub struct KcResult {
    pub upperband: Array1<f64>,
    pub middleband: Array1<f64>,
    pub lowerband: Array1<f64>,
}
#[derive(Debug, Clone)]
pub struct KdjResult {
    pub k: Array1<f64>,
    pub d: Array1<f64>,
    pub j: Array1<f64>,
}
#[derive(Debug, Clone)]
pub struct SmiResult {
    pub smi: Array1<f64>,
    pub smisignal: Array1<f64>,
}

fn same_len(name: &str, xs: &[&[f64]]) -> Result<usize> {
    let len = xs.first().map(|x| x.len()).unwrap_or(0);
    if xs.iter().any(|x| x.len() != len) {
        return Err(TaError::InvalidParameter {
            name: name.to_string(),
            constraint: "all inputs must have the same length".to_string(),
        });
    }
    Ok(len)
}

fn period(name: &str, value: usize, min: usize) -> Result<()> {
    validate_param(name, &format!("at least {min}"), || value >= min)
}

fn rolling_sum(input: &[f64], p: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; input.len()];
    let mut sum = 0.0;
    for i in 0..input.len() {
        sum += input[i];
        if i >= p {
            sum -= input[i - p];
        }
        if i + 1 >= p {
            out[i] = sum;
        }
    }
    out
}

fn rolling_minmax(high: &[f64], low: &[f64], p: usize) -> (Vec<f64>, Vec<f64>) {
    let mut hi = vec![f64::NAN; high.len()];
    let mut lo = vec![f64::NAN; low.len()];
    for i in p.saturating_sub(1)..high.len() {
        let start = i + 1 - p;
        hi[i] = high[start..=i]
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        lo[i] = low[start..=i].iter().copied().fold(f64::INFINITY, f64::min);
    }
    (hi, lo)
}

/// Wilder/RMA used by the TA-Lib profile (SMA seed, alpha = 1 / period).
pub fn rma_profile(input: &[f64], p: usize) -> Result<Array1<f64>> {
    period("timeperiod", p, 1)?;
    validate_input(input.len(), p)?;
    let mut out = init_output(input.len());
    let mut value = input[..p].iter().sum::<f64>() / p as f64;
    out[p - 1] = value;
    let inv = 1.0 / p as f64;
    for i in p..input.len() {
        value += (input[i] - value) * inv;
        out[i] = value;
    }
    Ok(out)
}

pub fn ac(
    high: &[f64],
    low: &[f64],
    fast: usize,
    slow: usize,
    signal: usize,
) -> Result<Array1<f64>> {
    let len = same_len("high, low", &[high, low])?;
    period("fastperiod", fast, 2)?;
    period("slowperiod", slow, 2)?;
    period("signalperiod", signal, 2)?;
    validate_input(len, slow.max(fast) + signal - 1)?;
    let median: Vec<f64> = high.iter().zip(low).map(|(h, l)| (h + l) / 2.0).collect();
    let fast_sum = rolling_sum(&median, fast);
    let slow_sum = rolling_sum(&median, slow);
    let mut out = init_output(len);
    let mut osc = vec![f64::NAN; len];
    for i in slow.saturating_sub(1)..len {
        osc[i] = fast_sum[i] / fast as f64 - slow_sum[i] / slow as f64;
    }
    let first = slow.max(fast) - 1 + signal - 1;
    for i in first..len {
        out[i] = osc[i - signal + 1..=i].iter().sum::<f64>() / signal as f64;
        out[i] = osc[i] - out[i];
    }
    Ok(out)
}

pub fn adr(high: &[f64], low: &[f64], p: usize) -> Result<Array1<f64>> {
    let len = same_len("high, low", &[high, low])?;
    period("timeperiod", p, 1)?;
    validate_input(len, p)?;
    let range: Vec<f64> = high.iter().zip(low).map(|(h, l)| h - l).collect();
    Ok(Array1::from_vec(
        rolling_sum(&range, p)
            .into_iter()
            .map(|v| if v.is_nan() { v } else { v / p as f64 })
            .collect(),
    ))
}

pub fn cmou(close: &[f64], p: usize) -> Result<Array1<f64>> {
    period("timeperiod", p, 1)?;
    validate_input(close.len(), p + 1)?;
    let mut out = init_output(close.len());
    for i in p..close.len() {
        let mut up = 0.0;
        let mut down = 0.0;
        for j in i - p + 1..=i {
            let d = close[j] - close[j - 1];
            if d > 0.0 {
                up += d;
            } else {
                down -= d;
            }
        }
        let den = up + down;
        out[i] = if den == 0.0 {
            0.0
        } else {
            100.0 * (up - down) / den
        };
    }
    Ok(out)
}

pub fn cvi(high: &[f64], low: &[f64], p: usize, roc: usize) -> Result<Array1<f64>> {
    let len = same_len("high, low", &[high, low])?;
    period("timeperiod", p, 1)?;
    period("rocperiod", roc, 1)?;
    validate_input(len, p + roc)?;
    let range: Vec<f64> = high.iter().zip(low).map(|(h, l)| h - l).collect();
    let e = ema(&range, p)?;
    let mut out = init_output(len);
    for i in p - 1 + roc..len {
        let prev = e[i - roc];
        out[i] = if prev == 0.0 {
            0.0
        } else {
            100.0 * (e[i] - prev) / prev
        };
    }
    Ok(out)
}

pub fn efi(close: &[f64], volume: &[f64], p: usize) -> Result<Array1<f64>> {
    let len = same_len("close, volume", &[close, volume])?;
    period("timeperiod", p, 1)?;
    validate_input(len, p + (p > 0) as usize)?;
    let mut force = vec![0.0; len];
    for i in 1..len {
        force[i] = (close[i] - close[i - 1]) * volume[i];
    }
    let mut out = init_output(len);
    if p == 1 {
        for i in 1..len {
            out[i] = force[i];
        }
        return Ok(out);
    }
    let start = p;
    let mut value = force[1..=p].iter().sum::<f64>() / p as f64;
    out[start] = value;
    let alpha = 2.0 / (p as f64 + 1.0);
    for i in start + 1..len {
        value += (force[i] - value) * alpha;
        out[i] = value;
    }
    Ok(out)
}

pub fn eri(high: &[f64], low: &[f64], close: &[f64], p: usize) -> Result<EriResult> {
    let len = same_len("high, low, close", &[high, low, close])?;
    period("timeperiod", p, 1)?;
    validate_input(len, p)?;
    let e = ema(close, p)?;
    let mut bull = init_output(len);
    let mut bear = init_output(len);
    for i in p - 1..len {
        bull[i] = high[i] - e[i];
        bear[i] = low[i] - e[i];
    }
    Ok(EriResult {
        bullpower: bull,
        bearpower: bear,
    })
}

pub fn fosc(close: &[f64], p: usize) -> Result<Array1<f64>> {
    period("timeperiod", p, 2)?;
    validate_input(close.len(), p + 1)?;
    let mut out = init_output(close.len());
    let sx = (p * (p - 1) / 2) as f64;
    let sx2 = (p * (p - 1) * (2 * p - 1) / 6) as f64;
    let div = sx * sx - p as f64 * sx2;
    for i in p..close.len() {
        let mut sy = 0.0;
        let mut sxy = 0.0;
        for j in 0..p {
            let v = close[i - p + j];
            let x = (p - 1 - j) as f64;
            sy += v;
            sxy += x * v;
        }
        let m = (p as f64 * sxy - sx * sy) / div;
        let b = (sy - m * sx) / p as f64;
        out[i] = if close[i] == 0.0 {
            0.0
        } else {
            100.0 * (close[i] - (m * p as f64 + b)) / close[i]
        };
    }
    Ok(out)
}

pub fn fractal(high: &[f64], low: &[f64], left: usize, right: usize) -> Result<FractalResult> {
    let len = same_len("high, low", &[high, low])?;
    period("leftbars", left, 1)?;
    period("rightbars", right, 1)?;
    validate_input(len, left + right + 1)?;
    let mut sh = init_output(len);
    let mut sl = init_output(len);
    let start = left + right;
    for i in start..len {
        let pivot = i - right;
        let mut oh = f64::NEG_INFINITY;
        let mut ol = f64::INFINITY;
        for j in i - left - right..=i {
            if j != pivot {
                oh = oh.max(high[j]);
                ol = ol.min(low[j]);
            }
        }
        sh[i] = if high[pivot] > oh { 100.0 } else { 0.0 };
        sl[i] = if low[pivot] < ol { 100.0 } else { 0.0 };
    }
    Ok(FractalResult {
        swinghigh: sh,
        swinglow: sl,
    })
}

pub fn kc(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    p: usize,
    atr_p: usize,
    dev: f64,
) -> Result<KcResult> {
    let len = same_len("high, low, close", &[high, low, close])?;
    period("timeperiod", p, 2)?;
    period("atrperiod", atr_p, 1)?;
    validate_input(len, p.max(atr_p) + 1)?;
    let tp: Vec<f64> = high
        .iter()
        .zip(low)
        .zip(close)
        .map(|((h, l), c)| (h + l + c) / 3.0)
        .collect();
    let middle = ema(&tp, p)?;
    let mut true_ranges = vec![f64::NAN; len];
    for i in 1..len {
        true_ranges[i] = (high[i] - low[i])
            .max((high[i] - close[i - 1]).abs())
            .max((low[i] - close[i - 1]).abs());
    }
    let mut av = vec![f64::NAN; len];
    let first = (p - 1).max(atr_p);
    let mut seed = 0.0;
    for i in first + 1 - atr_p..=first {
        seed += true_ranges[i];
    }
    let mut current = seed / atr_p as f64;
    av[first] = current;
    for i in first + 1..len {
        current += (true_ranges[i] - current) / atr_p as f64;
        av[i] = current;
    }
    let mut u = init_output(len);
    let mut l = init_output(len);
    for i in first..len {
        u[i] = middle[i] + dev * av[i];
        l[i] = middle[i] - dev * av[i];
    }
    let mut m = middle;
    for i in 0..first {
        m[i] = f64::NAN;
    }
    Ok(KcResult {
        upperband: u,
        middleband: m,
        lowerband: l,
    })
}

pub fn kdj(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    fast: usize,
    slow: usize,
    _slow_type: usize,
    signal: usize,
    _signal_type: usize,
) -> Result<KdjResult> {
    let len = same_len("high, low, close", &[high, low, close])?;
    period("fastk_period", fast, 1)?;
    period("slowk_period", slow, 1)?;
    period("slowd_period", signal, 1)?;
    validate_input(len, fast + slow + signal - 2)?;
    let (hi, lo) = rolling_minmax(high, low, fast);
    let mut raw = vec![f64::NAN; len];
    for i in fast - 1..len {
        let den = hi[i] - lo[i];
        raw[i] = if den == 0.0 {
            0.0
        } else {
            (close[i] - lo[i]) / den * 100.0
        };
    }
    let rawv = raw[fast - 1..].to_vec();
    let ksmall = rma_profile(&rawv, slow)?;
    let kv = ksmall.to_vec();
    let dv = rma_profile(&kv[slow - 1..], signal)?;
    let mut k = init_output(len);
    let mut d = init_output(len);
    let mut j = init_output(len);
    let kbase = fast - 1;
    let dbase = kbase + slow - 1;
    for i in signal.saturating_sub(1)..dv.len() {
        let idx = dbase + signal.saturating_sub(1) + i - signal.saturating_sub(1);
        if idx < len {
            d[idx] = dv[i];
            k[idx] = kv[idx - kbase];
            j[idx] = 3.0 * k[idx] - 2.0 * d[idx];
        }
    }
    Ok(KdjResult { k, d, j })
}

pub fn marketfi(high: &[f64], low: &[f64], volume: &[f64]) -> Result<Array1<f64>> {
    let len = same_len("high, low, volume", &[high, low, volume])?;
    validate_input(len, 1)?;
    let mut out = init_output(len);
    for i in 0..len {
        out[i] = if volume[i] == 0.0 {
            0.0
        } else {
            (high[i] - low[i]) / volume[i]
        };
    }
    Ok(out)
}

pub fn massi(high: &[f64], low: &[f64], fast: usize, slow: usize) -> Result<Array1<f64>> {
    let len = same_len("high, low", &[high, low])?;
    period("fastperiod", fast, 1)?;
    period("slowperiod", slow, 1)?;
    validate_input(len, fast * 2 + slow - 1)?;
    let range: Vec<f64> = high.iter().zip(low).map(|(h, l)| h - l).collect();
    let e1 = ema(&range, fast)?;
    let e1v = e1.to_vec();
    let e2 = ema(&e1v[fast - 1..], fast)?;
    let mut ratio = init_output(len);
    for i in fast - 1 + fast - 1..len {
        ratio[i] = e1[i] / e2[i - (fast - 1)];
    }
    let ratio_slice = ratio.as_slice().unwrap();
    let mut out = init_output(len);
    let first = 2 * (fast - 1) + slow - 1;
    for i in first..len {
        out[i] = ratio_slice[i - slow + 1..=i].iter().sum();
    }
    Ok(out)
}

pub fn percentile(input: &[f64], p: usize, pct: f64) -> Result<Array1<f64>> {
    period("timeperiod", p, 1)?;
    validate_param("percentile", "between 0 and 100", || {
        (0.0..=100.0).contains(&pct)
    })?;
    validate_input(input.len(), p)?;
    let mut out = init_output(input.len());
    let rank = ((pct * p as f64 / 100.0).ceil() as usize).clamp(1, p);
    for i in p - 1..input.len() {
        let mut w = input[i + 1 - p..=i].to_vec();
        w.sort_by(|a, b| a.partial_cmp(b).unwrap());
        out[i] = w[rank - 1];
    }
    Ok(out)
}

pub fn pvo(volume: &[f64], fast: usize, slow: usize, _ma_type: usize) -> Result<Array1<f64>> {
    period("fastperiod", fast, 1)?;
    period("slowperiod", slow, 1)?;
    validate_input(volume.len(), fast.max(slow))?;
    let f = ema(volume, fast)?;
    let s = ema(volume, slow)?;
    let mut out = init_output(volume.len());
    for i in slow.max(fast) - 1..volume.len() {
        out[i] = if s[i] == 0.0 {
            0.0
        } else {
            (f[i] - s[i]) / s[i] * 100.0
        };
    }
    Ok(out)
}

pub fn qstick(open: &[f64], close: &[f64], p: usize) -> Result<Array1<f64>> {
    let len = same_len("open, close", &[open, close])?;
    period("timeperiod", p, 1)?;
    validate_input(len, p)?;
    let x: Vec<f64> = close.iter().zip(open).map(|(c, o)| c - o).collect();
    sma(&x, p)
}

pub fn rvi_profile(input: &[f64], p: usize, std_p: usize) -> Result<Array1<f64>> {
    period("timeperiod", p, 1)?;
    period("stddevperiod", std_p, 2)?;
    validate_input(input.len(), p + std_p - 1)?;
    let len = input.len();
    let mut sigma = vec![f64::NAN; len];
    for i in std_p - 1..len {
        let w = &input[i + 1 - std_p..=i];
        let mean = w.iter().sum::<f64>() / std_p as f64;
        sigma[i] = (w.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / std_p as f64).sqrt();
    }
    let mut up = vec![0.0; len];
    let mut dn = vec![0.0; len];
    for i in 1..len {
        if input[i] > input[i - 1] {
            up[i] = sigma[i];
        } else if input[i] < input[i - 1] {
            dn[i] = sigma[i];
        }
    }
    let u = rma_profile(&up[std_p - 1..], p)?;
    let d = rma_profile(&dn[std_p - 1..], p)?;
    let mut out = init_output(len);
    let base = std_p - 1;
    for i in 0..u.len() {
        let idx = base + i;
        if idx < len {
            let den = u[i] + d[i];
            out[idx] = if den == 0.0 { 0.0 } else { 100.0 * u[i] / den };
        }
    }
    Ok(out)
}

pub fn rvol(volume: &[f64], p: usize) -> Result<Array1<f64>> {
    period("timeperiod", p, 1)?;
    validate_input(volume.len(), p + 1)?;
    let mut out = init_output(volume.len());
    for i in p..volume.len() {
        let avg = volume[i - p..i].iter().sum::<f64>() / p as f64;
        out[i] = if avg == 0.0 { 0.0 } else { volume[i] / avg };
    }
    Ok(out)
}

pub fn smi(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    p: usize,
    fast: usize,
    slow: usize,
    signal: usize,
) -> Result<SmiResult> {
    let len = same_len("high, low, close", &[high, low, close])?;
    period("timeperiod", p, 2)?;
    period("fastperiod", fast, 2)?;
    period("slowperiod", slow, 2)?;
    period("signalperiod", signal, 2)?;
    validate_input(len, p + slow + fast + signal - 3)?;
    let (hi, lo) = rolling_minmax(high, low, p);
    let mut n = vec![f64::NAN; len];
    let mut den = vec![f64::NAN; len];
    for i in p - 1..len {
        n[i] = close[i] - (hi[i] + lo[i]) * 0.5;
        den[i] = hi[i] - lo[i];
    }
    let ns = ema(&n[p - 1..], slow)?;
    let ds = ema(&den[p - 1..], slow)?;
    let nsv = ns.to_vec();
    let dsv = ds.to_vec();
    let nf = ema(&nsv[slow - 1..], fast)?;
    let df = ema(&dsv[slow - 1..], fast)?;
    let mut raw = vec![f64::NAN; len];
    let base = p - 1 + slow - 1 + fast - 1;
    for i in fast - 1..nf.len() {
        let idx = base + i - (fast - 1);
        if idx >= len {
            break;
        }
        let denom = df[i] * 0.5;
        raw[idx] = if denom == 0.0 {
            0.0
        } else {
            100.0 * nf[i] / denom
        };
    }
    let sig = ema(&raw[base..], signal)?;
    let mut smi = init_output(len);
    let mut line = init_output(len);
    for i in signal - 1..sig.len() {
        let idx = base + i;
        if idx < len {
            line[idx] = sig[i];
            smi[idx] = raw[idx];
        }
    }
    Ok(SmiResult {
        smi,
        smisignal: line,
    })
}

pub fn vhf(input: &[f64], p: usize) -> Result<Array1<f64>> {
    period("timeperiod", p, 2)?;
    validate_input(input.len(), p + 1)?;
    let mut out = init_output(input.len());
    for i in p..input.len() {
        let w = &input[i - p + 1..=i];
        let hi = w.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let lo = w.iter().copied().fold(f64::INFINITY, f64::min);
        let changes = (i - p + 1..=i)
            .map(|j| (input[j] - input[j - 1]).abs())
            .sum::<f64>();
        out[i] = if changes == 0.0 {
            0.0
        } else {
            (hi - lo) / changes
        };
    }
    Ok(out)
}

pub fn wad(high: &[f64], low: &[f64], close: &[f64]) -> Result<Array1<f64>> {
    let len = same_len("high, low, close", &[high, low, close])?;
    validate_input(len, 1)?;
    let mut out = init_output(len);
    let mut sum = 0.0;
    for i in 1..len {
        let prev = close[i - 1];
        let tr_hi = high[i].max(prev);
        let tr_lo = low[i].min(prev);
        if close[i] > prev {
            sum += close[i] - tr_lo;
        } else if close[i] < prev {
            sum += close[i] - tr_hi;
        }
        out[i] = sum;
    }
    Ok(out)
}
