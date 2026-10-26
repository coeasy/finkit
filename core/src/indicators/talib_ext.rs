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

#[cfg(test)]
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

/// Windowed high/low extrema for the TA-Lib profile.
///
/// Delegates to the shared fused kernel rather than folding each window, which
/// is what used to cost `O(window)` per bar in KDJ and SMI. Both strategies the
/// kernel picks between are NaN-transparent, so a missing bar never poisons the
/// extrema and never blocks the bars behind it. The warm-up slots stay `NaN`
/// because the kernel only visits fully populated windows.
fn rolling_minmax(high: &[f64], low: &[f64], p: usize) -> (Vec<f64>, Vec<f64>) {
    let mut hi = vec![f64::NAN; high.len()];
    let mut lo = vec![f64::NAN; low.len()];
    crate::math::statistics::rolling_minmax_visit(high, low, p, |i, highest, lowest| {
        hi[i] = highest;
        lo[i] = lowest;
    });
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
    let window = slow.max(fast);
    let required = window
        .checked_add(signal - 1)
        .ok_or(TaError::InvalidParameter {
            name: "periods".to_string(),
            constraint: "combined lookback must fit in usize".to_string(),
        })?;
    validate_input(len, required)?;
    let lookback = required - 1;

    // Fuse AO and its signal SMA into one pass instead of materializing median
    // prices, two rolling sums, the AO series, and repeatedly summing its
    // signal windows.
    let osc_start = window - 1;
    let mut fast_sum = 0.0;
    let mut slow_sum = 0.0;
    let mut trailing_fast = osc_start - (fast - 1);
    let mut trailing_slow = osc_start - (slow - 1);
    for i in 0..osc_start {
        let median = (high[i] + low[i]) / 2.0;
        if i >= trailing_fast {
            fast_sum += median;
        }
        if i >= trailing_slow {
            slow_sum += median;
        }
    }

    let mut out = Vec::with_capacity(len);
    out.resize(lookback, f64::NAN);
    let out_spare = out.spare_capacity_mut();
    let mut local_osc_ring = [0.0; 32];
    let mut heap_osc_ring = if signal > local_osc_ring.len() {
        vec![0.0; signal]
    } else {
        Vec::new()
    };
    let osc_ring = if signal <= local_osc_ring.len() {
        &mut local_osc_ring[..signal]
    } else {
        heap_osc_ring.as_mut_slice()
    };
    let mut osc_ring_index = 0;
    let mut signal_sum = 0.0;
    for i in osc_start..len {
        let median = (high[i] + low[i]) / 2.0;
        fast_sum += median;
        slow_sum += median;
        let oscillator = fast_sum / fast as f64 - slow_sum / slow as f64;

        fast_sum -= (high[trailing_fast] + low[trailing_fast]) / 2.0;
        slow_sum -= (high[trailing_slow] + low[trailing_slow]) / 2.0;
        trailing_fast += 1;
        trailing_slow += 1;

        osc_ring[osc_ring_index] = oscillator;
        signal_sum += oscillator;
        if i >= lookback {
            out_spare[i - lookback].write(oscillator - signal_sum / signal as f64);
            osc_ring_index += 1;
            if osc_ring_index == signal {
                osc_ring_index = 0;
            }
            signal_sum -= osc_ring[osc_ring_index];
        } else {
            osc_ring_index += 1;
            if osc_ring_index == signal {
                osc_ring_index = 0;
            }
        }
    }
    // Warm-up and calculated output slots are all initialized at this point.
    unsafe { out.set_len(len) };
    Ok(Array1::from_vec(out))
}

pub fn adr(high: &[f64], low: &[f64], p: usize) -> Result<Array1<f64>> {
    let len = same_len("high, low", &[high, low])?;
    period("timeperiod", p, 1)?;
    validate_input(len, p)?;

    // Fuse range generation, rolling accumulation, and averaging. The former
    // materialized both a full range vector and rolling-sum output before
    // collecting a third result vector. Preserve the rolling_sum operation
    // order so NaN/Inf propagation and floating-point rounding stay identical.
    let mut out = vec![f64::NAN; len];
    let mut sum = 0.0;
    for i in 0..len {
        sum += high[i] - low[i];
        if i >= p {
            sum -= high[i - p] - low[i - p];
        }
        if i + 1 >= p {
            out[i] = sum / p as f64;
        }
    }
    Ok(Array1::from_vec(out))
}

pub fn cmou(close: &[f64], p: usize) -> Result<Array1<f64>> {
    period("timeperiod", p, 2)?;
    validate_input(close.len(), p + 1)?;
    // Only the warm-up prefix is undefined. Avoid initializing the full output
    // to NaN only to overwrite every element after the lookback.
    let mut out = Vec::with_capacity(close.len());
    out.resize(p, f64::NAN);
    let spare = out.spare_capacity_mut();
    let mut total_change = 0.0;
    let mut null_run = 0;
    for j in 1..=p {
        let change = close[j] - close[j - 1];
        if !change.is_finite() {
            return cmou_reference(close, p);
        }
        total_change += change.abs();
        if change == 0.0 {
            null_run += 1;
        } else {
            null_run = 0;
        }
    }
    let mut previous = close[p];
    let mut trailing = close[0];
    let mut trailing_index = 1;
    if null_run >= p {
        total_change = 0.0;
    }
    if !total_change.is_finite() {
        return cmou_reference(close, p);
    }
    for (i, out_slot) in (p..close.len()).zip(spare.iter_mut()) {
        out_slot.write(if total_change > 0.0 {
            100.0 * (close[i] - close[i - p]) / total_change
        } else {
            0.0
        });
        if i + 1 < close.len() {
            let trailing_price = close[trailing_index];
            let old = trailing_price - trailing;
            trailing = trailing_price;
            trailing_index += 1;
            total_change -= old.abs();

            let current = close[i + 1];
            let new = current - previous;
            previous = current;
            total_change += new.abs();
            if new == 0.0 {
                null_run += 1;
            } else {
                null_run = 0;
            }
            if null_run >= p {
                null_run = p;
                total_change = 0.0;
            }
            if !total_change.is_finite() {
                return cmou_reference(close, p);
            }
        }
    }
    // The warm-up prefix and every spare slot for the remaining output range
    // have been initialized before extending the Vec's visible length.
    unsafe { out.set_len(close.len()) };
    Ok(Array1::from_vec(out))
}

fn cmou_reference(close: &[f64], p: usize) -> Result<Array1<f64>> {
    let mut out = Vec::with_capacity(close.len());
    out.resize(p, f64::NAN);
    for i in p..close.len() {
        let mut up = 0.0;
        let mut down = 0.0;
        for j in i - p + 1..=i {
            let change = close[j] - close[j - 1];
            if change > 0.0 {
                up += change;
            } else {
                down -= change;
            }
        }
        let den = up + down;
        out.push(if den == 0.0 {
            0.0
        } else {
            100.0 * (up - down) / den
        });
    }
    Ok(Array1::from_vec(out))
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
    let (swinghigh, swinglow) = fractal_impl::<f64>(high, low, left, right)?;
    Ok(FractalResult {
        swinghigh,
        swinglow,
    })
}

/// TA-Lib-compatible integer signal output without an intermediate f64 copy.
pub fn fractal_i32(
    high: &[f64],
    low: &[f64],
    left: usize,
    right: usize,
) -> Result<(Array1<i32>, Array1<i32>)> {
    fractal_impl::<i32>(high, low, left, right)
}

trait FractalValue: Copy {
    fn warmup() -> Self;
    fn signal(is_pivot: bool) -> Self;
}

impl FractalValue for f64 {
    #[inline]
    fn warmup() -> Self {
        f64::NAN
    }

    #[inline]
    fn signal(is_pivot: bool) -> Self {
        if is_pivot {
            100.0
        } else {
            0.0
        }
    }
}

impl FractalValue for i32 {
    #[inline]
    fn warmup() -> Self {
        0
    }

    #[inline]
    fn signal(is_pivot: bool) -> Self {
        if is_pivot {
            100
        } else {
            0
        }
    }
}

fn fractal_impl<T: FractalValue>(
    high: &[f64],
    low: &[f64],
    left: usize,
    right: usize,
) -> Result<(Array1<T>, Array1<T>)> {
    let len = same_len("high, low", &[high, low])?;
    period("leftbars", left, 1)?;
    period("rightbars", right, 1)?;
    let window = left
        .checked_add(right)
        .and_then(|value| value.checked_add(1))
        .ok_or(TaError::InvalidParameter {
            name: "leftbars, rightbars".to_string(),
            constraint: "combined window must fit in usize".to_string(),
        })?;
    validate_input(len, window)?;
    let start = window - 1;
    let mut sh = Vec::with_capacity(len);
    sh.resize(start, T::warmup());
    let mut sl = Vec::with_capacity(len);
    sl.resize(start, T::warmup());
    let sh_spare = sh.spare_capacity_mut();
    let sl_spare = sl.spare_capacity_mut();
    if left == 2 && right == 2 {
        for i in start..len {
            let pivot = i - 2;
            let highest = high[i - 4].max(high[i - 3]).max(high[i - 1]).max(high[i]);
            let lowest = low[i - 4].min(low[i - 3]).min(low[i - 1]).min(low[i]);
            sh_spare[i - start].write(T::signal(high[pivot] > highest));
            sl_spare[i - start].write(T::signal(low[pivot] < lowest));
        }
    } else if window <= 17 {
        // For compact neighborhoods, a direct pair of scans beats the four
        // monotonic queues' bookkeeping while still avoiding a pivot branch.
        for i in start..len {
            let pivot = i - right;
            let mut highest = f64::NEG_INFINITY;
            let mut lowest = f64::INFINITY;
            for j in i - left - right..pivot {
                highest = highest.max(high[j]);
                lowest = lowest.min(low[j]);
            }
            for j in pivot + 1..=i {
                highest = highest.max(high[j]);
                lowest = lowest.min(low[j]);
            }
            sh_spare[i - start].write(T::signal(high[pivot] > highest));
            sl_spare[i - start].write(T::signal(low[pivot] < lowest));
        }
    } else {
        let mut left_high = std::collections::VecDeque::with_capacity(left);
        let mut left_low = std::collections::VecDeque::with_capacity(left);
        let mut right_high = std::collections::VecDeque::with_capacity(right);
        let mut right_low = std::collections::VecDeque::with_capacity(right);
        for i in 0..len {
            fractal_push_extreme::<true>(&mut right_high, high, i);
            fractal_push_extreme::<false>(&mut right_low, low, i);
            let right_start = i.saturating_add(1).saturating_sub(right);
            fractal_expire_before(&mut right_high, right_start);
            fractal_expire_before(&mut right_low, right_start);

            if i > right {
                let pivot = i - right;
                fractal_push_extreme::<true>(&mut left_high, high, pivot - 1);
                fractal_push_extreme::<false>(&mut left_low, low, pivot - 1);
                let left_start = pivot.saturating_sub(left);
                fractal_expire_before(&mut left_high, left_start);
                fractal_expire_before(&mut left_low, left_start);
            }

            if i >= start {
                let pivot = i - right;
                let highest = fractal_extreme::<true>(&left_high, high)
                    .max(fractal_extreme::<true>(&right_high, high));
                let lowest = fractal_extreme::<false>(&left_low, low)
                    .min(fractal_extreme::<false>(&right_low, low));
                sh_spare[i - start].write(T::signal(high[pivot] > highest));
                sl_spare[i - start].write(T::signal(low[pivot] < lowest));
            }
        }
    }
    // The warm-up prefix and every calculated output slot are initialized.
    unsafe {
        sh.set_len(len);
        sl.set_len(len);
    }
    Ok((Array1::from_vec(sh), Array1::from_vec(sl)))
}

#[inline]
fn fractal_push_extreme<const WANT_MAX: bool>(
    deque: &mut std::collections::VecDeque<usize>,
    values: &[f64],
    index: usize,
) {
    let value = values[index];
    if value.is_nan() {
        return;
    }
    while let Some(&back) = deque.back() {
        let dominated = if WANT_MAX {
            values[back] <= value
        } else {
            values[back] >= value
        };
        if dominated {
            deque.pop_back();
        } else {
            break;
        }
    }
    deque.push_back(index);
}

#[inline]
fn fractal_expire_before(deque: &mut std::collections::VecDeque<usize>, start: usize) {
    while deque.front().is_some_and(|&index| index < start) {
        deque.pop_front();
    }
}

#[inline]
fn fractal_extreme<const WANT_MAX: bool>(
    deque: &std::collections::VecDeque<usize>,
    values: &[f64],
) -> f64 {
    match deque.front() {
        Some(&index) => values[index],
        None if WANT_MAX => f64::NEG_INFINITY,
        None => f64::INFINITY,
    }
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
    let lookback_ema = fast - 1;
    let lookback_ema2 = lookback_ema * 2;
    let lookback_total = lookback_ema2 + slow - 1;
    validate_input(len, lookback_total + 1)?;

    let mut out = init_output(len);
    let mut ratio_ring = vec![0.0; slow];
    let mut ring_index = 0usize;
    let mut rolling_total = 0.0;
    let mut ema1 = 0.0;
    let mut ema2 = 0.0;
    let mut sum1 = 0.0;
    let mut sum2 = 0.0;
    let k = 2.0 / (fast as f64 + 1.0);

    for today in 0..len {
        let range = high[today] - low[today];
        if !range.is_finite() {
            return Err(TaError::InvalidParameter {
                name: "input".to_string(),
                constraint: format!("non-finite value at index {today}"),
            });
        }

        if today < fast {
            sum1 += range;
            if today == fast - 1 {
                ema1 = sum1 / fast as f64;
            }
        } else {
            ema1 = ((range - ema1) * k) + ema1;
        }

        if today >= lookback_ema {
            let second_index = today - lookback_ema;
            if !ema1.is_finite() {
                return Err(TaError::InvalidParameter {
                    name: "input".to_string(),
                    constraint: format!("non-finite value at index {second_index}"),
                });
            }
            if second_index < fast {
                sum2 += ema1;
                if second_index == fast - 1 {
                    ema2 = sum2 / fast as f64;
                }
            } else {
                ema2 = ((ema1 - ema2) * k) + ema2;
            }
        }

        if today >= lookback_ema2 {
            let ratio = if ema2 == 0.0 { 1.0 } else { ema1 / ema2 };
            rolling_total += ratio;
            ratio_ring[ring_index] = ratio;
            ring_index += 1;
            if ring_index == slow {
                ring_index = 0;
            }

            if today >= lookback_total {
                out[today] = rolling_total;
                // Match TA-Lib's TA_SUM order: add the new ratio, publish the
                // total, then remove the oldest entry from the next window.
                rolling_total -= ratio_ring[ring_index];
            }
        }
    }
    Ok(out)
}

pub fn percentile(input: &[f64], p: usize, pct: f64) -> Result<Array1<f64>> {
    period("timeperiod", p, 2)?;
    validate_param("percentile", "between 0 and 100", || {
        (0.0..=100.0).contains(&pct)
    })?;
    validate_input(input.len(), p)?;
    let mut out = init_output(input.len());
    let rank = ((pct * p as f64 / 100.0).ceil() as usize).clamp(1, p);
    if input.iter().all(|value| value.is_finite()) {
        let mut ordered = input[..p].to_vec();
        ordered.sort_by(|a, b| a.partial_cmp(b).expect("finite values are comparable"));
        let mut ring = input[..p].to_vec();
        let mut ring_index = 0;
        for i in p - 1..input.len() {
            out[i] = ordered[rank - 1];
            let next_index = i + 1;
            if next_index < input.len() {
                let leaving = ring[ring_index];
                // Equal values stay in arrival order, matching the reference
                // rolling window even for signed zero.
                let mut leaving_lo = 0;
                let mut leaving_hi = p;
                while leaving_lo < leaving_hi {
                    let mid = leaving_lo + (leaving_hi - leaving_lo) / 2;
                    if ordered[mid] < leaving {
                        leaving_lo = mid + 1;
                    } else {
                        leaving_hi = mid;
                    }
                }
                let leaving_index = leaving_lo;
                let entering = input[next_index];
                // Locate the stable insertion point with the old value omitted,
                // then perform just one contiguous shift for this update.
                let mut lo = 0;
                let mut hi = p - 1;
                while lo < hi {
                    let mid = lo + (hi - lo) / 2;
                    let ordered_index = if mid >= leaving_index { mid + 1 } else { mid };
                    if ordered[ordered_index] <= entering {
                        lo = mid + 1;
                    } else {
                        hi = mid;
                    }
                }
                let insertion_index = lo;
                if insertion_index < leaving_index {
                    ordered.copy_within(insertion_index..leaving_index, insertion_index + 1);
                    ordered[insertion_index] = entering;
                } else if insertion_index > leaving_index {
                    ordered.copy_within(leaving_index + 1..=insertion_index, leaving_index);
                    ordered[insertion_index] = entering;
                } else {
                    ordered[leaving_index] = entering;
                }
                ring[ring_index] = entering;
                ring_index += 1;
                if ring_index == p {
                    ring_index = 0;
                }
            }
        }
    } else {
        // Preserve deterministic handling of NaN and infinities. Market-data
        // finite inputs use the rolling fast path above.
        for i in p - 1..input.len() {
            let mut window = input[i + 1 - p..=i].to_vec();
            window.sort_by(f64::total_cmp);
            out[i] = window[rank - 1];
        }
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

    if let Some(output) = qstick_finite_fused(open, close, p) {
        return Ok(output);
    }

    let x: Vec<f64> = close.iter().zip(open).map(|(c, o)| c - o).collect();
    sma(&x, p)
}

fn qstick_finite_fused(open: &[f64], close: &[f64], period: usize) -> Option<Array1<f64>> {
    let len = open.len();
    let mut output = Vec::with_capacity(len);
    output.resize(period - 1, f64::NAN);
    let inv_period = 1.0 / period as f64;

    let mut sum = 0.0;
    for index in 0..period {
        let body = close[index] - open[index];
        if !body.is_finite() {
            return None;
        }
        sum += body;
    }
    if !sum.is_finite() {
        return None;
    }
    output.push(sum * inv_period);

    for index in period..len {
        let entering = close[index] - open[index];
        let leaving = close[index - period] - open[index - period];
        if !entering.is_finite() || !leaving.is_finite() {
            return None;
        }
        sum += entering - leaving;
        if !sum.is_finite() {
            return None;
        }
        output.push(sum * inv_period);
    }
    Some(Array1::from_vec(output))
}

#[cfg(test)]
mod qstick_fused_tests {
    use super::*;

    #[test]
    fn qstick_fused_matches_materialized_sma() {
        let open = (0..2048)
            .map(|index| 100.0 + (index as f64 * 0.017).sin() * 3.0)
            .collect::<Vec<_>>();
        let close = (0..2048)
            .map(|index| 100.0 + (index as f64 * 0.021).cos() * 2.0)
            .collect::<Vec<_>>();
        let bodies = close
            .iter()
            .zip(&open)
            .map(|(close, open)| close - open)
            .collect::<Vec<_>>();
        for period in [1, 2, 3, 10, 17, 64, 252] {
            let actual = qstick(&open, &close, period).unwrap();
            let expected = crate::math::moving_avg::sma(&bodies, period).unwrap();
            for (index, (actual, expected)) in actual.iter().zip(expected.iter()).enumerate() {
                if expected.is_nan() {
                    assert!(actual.is_nan(), "period={period}, index={index}");
                } else {
                    assert!(
                        (actual - expected).abs() <= 1e-12,
                        "period={period}, index={index}, actual={actual}, expected={expected}"
                    );
                }
            }
        }
    }

    #[test]
    fn qstick_non_finite_falls_back_to_existing_contract() {
        let open = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let mut close = [1.5, 2.5, 3.5, 4.5, 5.5, 6.5];
        close[3] = f64::NAN;
        assert!(qstick(&open, &close, 2).is_err());
    }
}

pub fn rvi_profile(input: &[f64], p: usize, std_p: usize) -> Result<Array1<f64>> {
    period("timeperiod", p, 1)?;
    period("stddevperiod", std_p, 2)?;
    let required = p.checked_add(std_p - 1).ok_or(TaError::InvalidParameter {
        name: "periods".to_string(),
        constraint: "combined lookback must fit in usize".to_string(),
    })?;
    validate_input(input.len(), required)?;

    let len = input.len();
    let base = std_p - 1;
    let first_output = base + p - 1;
    let mut out = Vec::with_capacity(len);
    out.resize(first_output, f64::NAN);
    let out_spare = out.spare_capacity_mut();
    let mut shift = input[0];
    let mut period_total = 0.0;
    let mut square_total = 0.0;
    for &value in &input[..base] {
        if !value.is_finite() {
            return rvi_profile_reference(input, p, std_p);
        }
        let centered = value - shift;
        period_total += centered;
        square_total += centered * centered;
    }
    let mut trailing = 0;
    let mut peak_square_total = square_total;
    let mut bars_since_reseed = std_p.saturating_mul(32);
    let mut up_total = 0.0;
    let mut down_total = 0.0;
    let mut today = base;

    for _ in 0..p {
        let Some(sigma) = rvi_next_sigma(
            input,
            today,
            std_p,
            &mut shift,
            &mut period_total,
            &mut square_total,
            &mut trailing,
            &mut peak_square_total,
            &mut bars_since_reseed,
        ) else {
            return rvi_profile_reference(input, p, std_p);
        };
        let delta = input[today] - input[today - 1];
        if delta > 0.0 {
            up_total += sigma;
        } else if delta < 0.0 {
            down_total += sigma;
        }
        today += 1;
    }

    let mut up_rma = up_total / p as f64;
    let mut down_rma = down_total / p as f64;
    let denominator = up_rma + down_rma;
    out_spare[0].write(if denominator == 0.0 {
        50.0
    } else {
        100.0 * (up_rma / denominator)
    });

    let beta = (p - 1) as f64 / p as f64;
    let alpha = 1.0 - beta;
    for i in first_output + 1..len {
        let Some(sigma) = rvi_next_sigma(
            input,
            today,
            std_p,
            &mut shift,
            &mut period_total,
            &mut square_total,
            &mut trailing,
            &mut peak_square_total,
            &mut bars_since_reseed,
        ) else {
            return rvi_profile_reference(input, p, std_p);
        };
        let delta = input[today] - input[today - 1];
        let (up, down) = if delta > 0.0 {
            (sigma, 0.0)
        } else if delta < 0.0 {
            (0.0, sigma)
        } else {
            (0.0, 0.0)
        };
        up_rma = alpha * up + beta * up_rma;
        down_rma = alpha * down + beta * down_rma;
        let denominator = up_rma + down_rma;
        out_spare[i - first_output].write(if denominator == 0.0 {
            50.0
        } else {
            100.0 * (up_rma / denominator)
        });
        today += 1;
    }

    // Warm-up and every post-lookback result have been initialized.
    unsafe { out.set_len(len) };
    Ok(Array1::from_vec(out))
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn rvi_next_sigma(
    input: &[f64],
    today: usize,
    period: usize,
    shift: &mut f64,
    period_total: &mut f64,
    square_total: &mut f64,
    trailing: &mut usize,
    peak_square_total: &mut f64,
    bars_since_reseed: &mut usize,
) -> Option<f64> {
    let current = *input.get(today)?;
    if !current.is_finite() {
        return None;
    }
    let centered = current - *shift;
    *period_total += centered;
    *square_total += centered * centered;
    *peak_square_total = (*peak_square_total).max(*square_total);

    let inverse_period = 1.0 / period as f64;
    let mean = *period_total * inverse_period;
    let mut variance = *square_total * inverse_period - mean * mean;
    let outgoing = *input.get(*trailing)? - *shift;
    *period_total -= outgoing;
    *square_total -= outgoing * outgoing;
    *trailing += 1;

    *bars_since_reseed = bars_since_reseed.saturating_sub(1);
    if variance < 1e-6 * (*peak_square_total * inverse_period) || *bars_since_reseed == 0 {
        *bars_since_reseed = period.saturating_mul(32);
        let window_start = today + 1 - period;
        let window = input.get(window_start..=today)?;
        let mut window_sum = 0.0;
        for &value in window {
            if !value.is_finite() {
                return None;
            }
            window_sum += value;
        }
        *shift = window_sum * inverse_period;
        *period_total = 0.0;
        *square_total = 0.0;
        for &value in window {
            let centered = value - *shift;
            *period_total += centered;
            *square_total += centered * centered;
        }
        let mean = *period_total * inverse_period;
        variance = *square_total * inverse_period - mean * mean;
        if variance < 1e-6 * (*square_total * inverse_period) {
            *shift = current;
            *period_total = 0.0;
            *square_total = 0.0;
            for &value in window {
                let centered = value - *shift;
                *period_total += centered;
                *square_total += centered * centered;
            }
            let mean = *period_total * inverse_period;
            variance = *square_total * inverse_period - mean * mean;
        }
        *peak_square_total = *square_total;
        if variance < 1e-12 * (*square_total * inverse_period) {
            variance = 0.0;
        }
        let outgoing = input[window_start] - *shift;
        *period_total -= outgoing;
        *square_total -= outgoing * outgoing;
        *trailing = window_start + 1;
    }

    if !variance.is_finite() || !period_total.is_finite() || !square_total.is_finite() {
        return None;
    }
    Some(variance.sqrt())
}

fn rvi_profile_reference(input: &[f64], p: usize, std_p: usize) -> Result<Array1<f64>> {
    let len = input.len();
    let mut sigma = vec![f64::NAN; len];
    for i in std_p - 1..len {
        let window = &input[i + 1 - std_p..=i];
        let mean = window.iter().sum::<f64>() / std_p as f64;
        sigma[i] =
            (window.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / std_p as f64).sqrt();
    }
    let mut up = vec![0.0; len];
    let mut down = vec![0.0; len];
    for i in 1..len {
        if input[i] > input[i - 1] {
            up[i] = sigma[i];
        } else if input[i] < input[i - 1] {
            down[i] = sigma[i];
        }
    }
    let up_rma = rma_profile(&up[std_p - 1..], p)?;
    let down_rma = rma_profile(&down[std_p - 1..], p)?;
    let mut output = init_output(len);
    let base = std_p - 1;
    for i in 0..up_rma.len() {
        let index = base + i;
        if index < len {
            let denominator = up_rma[i] + down_rma[i];
            output[index] = if denominator == 0.0 {
                0.0
            } else {
                100.0 * up_rma[i] / denominator
            };
        }
    }
    Ok(output)
}

pub fn rvol(volume: &[f64], p: usize) -> Result<Array1<f64>> {
    period("timeperiod", p, 1)?;
    validate_input(volume.len(), p + 1)?;

    let len = volume.len();
    let mut out = Vec::with_capacity(len);
    out.resize(p, f64::NAN);
    let out_spare = out.spare_capacity_mut();
    let mut sum = volume[..p].iter().sum::<f64>();
    if !sum.is_finite() {
        return rvol_reference(volume, p);
    }
    let period_scale = p as f64;
    for i in p..len {
        let value = if sum == 0.0 {
            0.0
        } else if sum.abs() < f64::MIN_POSITIVE * period_scale {
            // Preserve the division-underflow behavior of volume / (sum / p)
            // for subnormal window totals.
            let average = sum / period_scale;
            if average == 0.0 {
                0.0
            } else {
                volume[i] / average
            }
        } else {
            let scaled_volume = volume[i] * period_scale;
            if volume[i].is_finite() && !scaled_volume.is_finite() {
                return rvol_reference(volume, p);
            }
            scaled_volume / sum
        };
        out_spare[i - p].write(value);
        if i + 1 < len {
            // Advance from the prior-period window [i-p, i) to [i-p+1, i+1).
            sum += volume[i];
            sum -= volume[i - p];
            if !sum.is_finite() {
                return rvol_reference(volume, p);
            }
        }
    }
    // The warm-up prefix and each result slot have been written.
    unsafe { out.set_len(len) };
    Ok(Array1::from_vec(out))
}

fn rvol_reference(volume: &[f64], p: usize) -> Result<Array1<f64>> {
    let mut out = Vec::with_capacity(volume.len());
    out.resize(p, f64::NAN);
    for i in p..volume.len() {
        let average = volume[i - p..i].iter().sum::<f64>() / p as f64;
        out.push(if average == 0.0 {
            0.0
        } else {
            volume[i] / average
        });
    }
    Ok(Array1::from_vec(out))
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
    if input.iter().any(|value| !value.is_finite()) {
        // Retain the reference behavior for missing and infinite observations;
        // the fast rolling path below is specialized for the common finite
        // market-data case.
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
        return Ok(out);
    }

    let highest = crate::math::statistics::rolling_max(input, p)?;
    let lowest = crate::math::statistics::rolling_min(input, p)?;
    let mut changes = input[..=p]
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .sum::<f64>();
    for i in p..input.len() {
        if i > p {
            changes += (input[i] - input[i - 1]).abs() - (input[i - p] - input[i - p - 1]).abs();
        }
        out[i] = if changes == 0.0 {
            0.0
        } else {
            (highest[i] - lowest[i]) / changes
        };
    }
    Ok(out)
}

pub fn wad(high: &[f64], low: &[f64], close: &[f64]) -> Result<Array1<f64>> {
    let len = same_len("high, low, close", &[high, low, close])?;
    validate_input(len, 1)?;
    let mut out = Vec::with_capacity(len);
    out.push(f64::NAN);
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
        out.push(sum);
    }
    Ok(Array1::from_vec(out))
}

#[cfg(test)]
mod nan_safety_tests {
    use super::*;

    #[test]
    fn wad_push_output_matches_initialized_reference() {
        let mut high: Vec<f64> = (0..257)
            .map(|i| 100.0 + (i as f64 * 0.17).sin() * 3.0)
            .collect();
        let low: Vec<f64> = high.iter().map(|value| value - 0.5).collect();
        let mut close: Vec<f64> = high
            .iter()
            .enumerate()
            .map(|(i, value)| value + (i as f64 * 0.11).cos() * 0.3)
            .collect();
        high[91] = f64::NAN;
        close[137] = f64::NAN;

        let actual = wad(&high, &low, &close).unwrap();
        let mut expected = init_output(high.len());
        let mut sum = 0.0;
        for i in 1..high.len() {
            let prev = close[i - 1];
            let tr_hi = high[i].max(prev);
            let tr_lo = low[i].min(prev);
            if close[i] > prev {
                sum += close[i] - tr_lo;
            } else if close[i] < prev {
                sum += close[i] - tr_hi;
            }
            expected[i] = sum;
        }

        for (actual, expected) in actual.iter().zip(expected.iter()) {
            if expected.is_nan() {
                assert!(actual.is_nan());
            } else {
                assert_eq!(actual.to_bits(), expected.to_bits());
            }
        }
    }

    #[test]
    fn adr_fused_loop_matches_materialized_reference() {
        let high = (0..257)
            .map(|i| 100.0 + (i as f64 * 0.19).sin() * 4.0 + i as f64 * 0.03)
            .collect::<Vec<_>>();
        let low = high
            .iter()
            .enumerate()
            .map(|(i, value)| value - 0.2 - (i as f64 * 0.07).cos().abs())
            .collect::<Vec<_>>();

        for p in [1, 2, 14, 127, high.len()] {
            let actual = adr(&high, &low, p).unwrap();
            let range = high
                .iter()
                .zip(&low)
                .map(|(h, l)| h - l)
                .collect::<Vec<_>>();
            let expected = rolling_sum(&range, p)
                .into_iter()
                .map(|value| {
                    if value.is_nan() {
                        value
                    } else {
                        value / p as f64
                    }
                })
                .collect::<Vec<_>>();
            for (actual, expected) in actual.iter().zip(expected) {
                if expected.is_nan() {
                    assert!(actual.is_nan());
                } else {
                    assert_eq!(actual.to_bits(), expected.to_bits());
                }
            }
        }

        let mut high_with_nan = high.clone();
        high_with_nan[40] = f64::NAN;
        let actual = adr(&high_with_nan, &low, 14).unwrap();
        let ranges = high_with_nan
            .iter()
            .zip(&low)
            .map(|(h, l)| h - l)
            .collect::<Vec<_>>();
        for (actual, expected) in actual.iter().zip(rolling_sum(&ranges, 14)) {
            if expected.is_nan() {
                assert!(actual.is_nan());
            } else {
                assert_eq!(actual.to_bits(), (expected / 14.0).to_bits());
            }
        }
    }

    #[test]
    fn massi_incremental_window_sum_matches_recomputed_windows() {
        let high = (0..4096)
            .map(|i| 50.0 + (i as f64 * 0.013).sin() * 2.0 + i as f64 * 0.001)
            .collect::<Vec<_>>();
        let low = high
            .iter()
            .enumerate()
            .map(|(i, value)| value - 0.25 - (i as f64 * 0.071).cos().abs())
            .collect::<Vec<_>>();

        for (fast, slow) in [(1, 1), (1, 7), (2, 3), (5, 34), (9, 25), (31, 64)] {
            let actual = massi(&high, &low, fast, slow).unwrap();
            let range = high
                .iter()
                .zip(&low)
                .map(|(h, l)| h - l)
                .collect::<Vec<_>>();
            let first_ema = ema(&range, fast).unwrap();
            let first_ema_values = first_ema.to_vec();
            let second_ema = ema(&first_ema_values[fast - 1..], fast).unwrap();
            let first = 2 * (fast - 1) + slow - 1;
            for index in first..high.len() {
                let start = index + 1 - slow;
                let expected = (start..=index)
                    .map(|j| first_ema[j] / second_ema[j - (fast - 1)])
                    .sum::<f64>();
                assert!(
                    (actual[index] - expected).abs() <= 1e-10,
                    "fast={fast}, slow={slow}, i={index}, actual={}, expected={expected}",
                    actual[index]
                );
            }
        }

        let flat = vec![100.0; 128];
        for value in massi(&flat, &flat, 9, 25).unwrap().iter().skip(41) {
            assert_eq!(*value, 25.0);
        }
    }

    #[test]
    fn ac_fused_kernel_matches_materialized_composition() {
        let high = (0..2048)
            .map(|i| 100.0 + (i as f64 * 0.017).sin() + i as f64 * 0.002)
            .collect::<Vec<_>>();
        let low = high
            .iter()
            .enumerate()
            .map(|(i, value)| value - 0.5 - (i as f64 * 0.031).cos().abs())
            .collect::<Vec<_>>();
        let median = high
            .iter()
            .zip(&low)
            .map(|(h, l)| (h + l) / 2.0)
            .collect::<Vec<_>>();
        for (fast, slow, signal) in [(2, 5, 2), (5, 34, 5), (40, 10, 7)] {
            let actual = ac(&high, &low, fast, slow, signal).unwrap();
            let fast_sum = rolling_sum(&median, fast);
            let slow_sum = rolling_sum(&median, slow);
            let mut oscillator = vec![f64::NAN; high.len()];
            for i in 0..high.len() {
                oscillator[i] = fast_sum[i] / fast as f64 - slow_sum[i] / slow as f64;
            }
            let first = slow.max(fast) - 1 + signal - 1;
            for i in first..high.len() {
                let expected = oscillator[i]
                    - oscillator[i + 1 - signal..=i].iter().sum::<f64>() / signal as f64;
                assert!(
                    (actual[i] - expected).abs() <= 1e-10,
                    "fast={fast}, slow={slow}, signal={signal}, i={i}"
                );
            }
        }
    }

    #[test]
    fn fractal_prefix_initialization_matches_window_reference() {
        let high = (0..1024)
            .map(|i| 100.0 + (i as f64 * 0.037).sin() * 7.0 + (i % 13) as f64 * 0.01)
            .collect::<Vec<_>>();
        let low = high
            .iter()
            .enumerate()
            .map(|(i, value)| value - 0.2 - (i as f64 * 0.019).cos().abs())
            .collect::<Vec<_>>();
        for (left, right) in [(1, 1), (2, 2), (4, 3), (20, 20), (64, 32)] {
            let actual = fractal(&high, &low, left, right).unwrap();
            let (integer_high, integer_low) = fractal_i32(&high, &low, left, right).unwrap();
            for i in 0..high.len() {
                assert_eq!(
                    integer_high[i],
                    if actual.swinghigh[i].is_finite() {
                        actual.swinghigh[i] as i32
                    } else {
                        0
                    }
                );
                assert_eq!(
                    integer_low[i],
                    if actual.swinglow[i].is_finite() {
                        actual.swinglow[i] as i32
                    } else {
                        0
                    }
                );
            }
            let first = left + right;
            assert!(actual.swinghigh.as_slice().unwrap()[..first]
                .iter()
                .all(|value| value.is_nan()));
            assert!(actual.swinglow.as_slice().unwrap()[..first]
                .iter()
                .all(|value| value.is_nan()));
            for i in first..high.len() {
                let pivot = i - right;
                let mut highest = f64::NEG_INFINITY;
                let mut lowest = f64::INFINITY;
                for j in i - left - right..=i {
                    if j != pivot {
                        highest = highest.max(high[j]);
                        lowest = lowest.min(low[j]);
                    }
                }
                assert_eq!(
                    actual.swinghigh[i],
                    if high[pivot] > highest { 100.0 } else { 0.0 }
                );
                assert_eq!(
                    actual.swinglow[i],
                    if low[pivot] < lowest { 100.0 } else { 0.0 }
                );
            }
        }

        let mut high_with_nan = high.clone();
        let mut low_with_nan = low.clone();
        for index in [17, 64, 255, 511, 900] {
            high_with_nan[index] = f64::NAN;
            low_with_nan[index] = f64::NAN;
        }
        let (left, right) = (20, 20);
        let actual = fractal(&high_with_nan, &low_with_nan, left, right).unwrap();
        for i in left + right..high.len() {
            let pivot = i - right;
            let mut highest = f64::NEG_INFINITY;
            let mut lowest = f64::INFINITY;
            for j in i - left - right..=i {
                if j != pivot {
                    highest = highest.max(high_with_nan[j]);
                    lowest = lowest.min(low_with_nan[j]);
                }
            }
            assert_eq!(
                actual.swinghigh[i],
                if high_with_nan[pivot] > highest {
                    100.0
                } else {
                    0.0
                }
            );
            assert_eq!(
                actual.swinglow[i],
                if low_with_nan[pivot] < lowest {
                    100.0
                } else {
                    0.0
                }
            );
        }
    }

    #[test]
    fn cmou_rolling_path_matches_window_definition() {
        let input = (0..4096)
            .map(|i| (i as f64 * 0.013).sin() * 20.0 + (i as f64 * 0.071).cos())
            .collect::<Vec<_>>();
        for p in [2, 3, 14, 30, 127] {
            let actual = cmou(&input, p).unwrap();
            for i in p..input.len() {
                let mut up = 0.0;
                let mut down = 0.0;
                for j in i - p + 1..=i {
                    let change = input[j] - input[j - 1];
                    if change > 0.0 {
                        up += change;
                    } else {
                        down -= change;
                    }
                }
                let denominator = up + down;
                let expected = if denominator == 0.0 {
                    0.0
                } else {
                    100.0 * (up - down) / denominator
                };
                assert!((actual[i] - expected).abs() <= 1e-9, "p={p}, i={i}");
            }
        }
    }

    #[test]
    fn cmou_rejects_period_below_talib_minimum() {
        assert!(cmou(&[1.0, 2.0], 1).is_err());
    }

    #[test]
    fn cmou_non_finite_input_retains_window_recovery_behavior() {
        let input = [1.0, 2.0, f64::NAN, 4.0, 5.0, 6.0, 5.0, 4.0];
        let actual = cmou(&input, 2).unwrap();
        assert!(actual[2].is_nan());
        assert!(actual[3].is_nan());
        assert!(actual[4].is_nan());
        assert_eq!(actual[5], 100.0);
        assert_eq!(actual[6], 0.0);
        assert_eq!(actual[7], -100.0);
    }

    #[test]
    fn rvol_rolling_sum_matches_window_reference() {
        let input = (0..4096)
            .map(|i| 10_000.0 + (i as f64 * 0.17).sin() * 3_000.0 + i as f64)
            .collect::<Vec<_>>();
        for p in [1, 2, 20, 127] {
            let actual = rvol(&input, p).unwrap();
            for i in p..input.len() {
                let average = input[i - p..i].iter().sum::<f64>() / p as f64;
                let expected = if average == 0.0 {
                    0.0
                } else {
                    input[i] / average
                };
                assert!((actual[i] - expected).abs() <= 1e-10, "p={p}, i={i}");
            }
        }
    }

    #[test]
    fn rvol_non_finite_and_zero_windows_keep_reference_behavior() {
        let input = [0.0, 0.0, 2.0, f64::NAN, 4.0, 5.0, 6.0, 7.0];
        let actual = rvol(&input, 2).unwrap();
        assert_eq!(actual[2], 0.0);
        assert!(actual[3].is_nan());
        assert!(actual[4].is_nan());
        assert!(actual[5].is_nan());
        assert!((actual[6] - 6.0 / 4.5).abs() <= f64::EPSILON);
        assert!((actual[7] - 7.0 / 5.5).abs() <= f64::EPSILON);
    }

    #[test]
    fn rvi_rolling_variance_matches_materialized_reference() {
        let input = (0..4096)
            .map(|i| 100.0 + (i as f64 * 0.013).sin() * 4.0 + i as f64 * 0.0003)
            .collect::<Vec<_>>();
        for (period, std_period) in [(1, 2), (2, 2), (14, 10), (30, 20), (127, 63)] {
            let actual = rvi_profile(&input, period, std_period).unwrap();
            let expected = rvi_profile_reference(&input, period, std_period).unwrap();
            for (index, (&actual, &expected)) in actual.iter().zip(expected.iter()).enumerate() {
                if expected.is_nan() {
                    assert!(
                        actual.is_nan(),
                        "period={period}, std={std_period}, i={index}"
                    );
                } else {
                    let error = (actual - expected).abs();
                    assert!(
                        error <= 1e-8,
                        "period={period}, std={std_period}, i={index}, error={error}"
                    );
                }
            }
        }
    }

    #[test]
    fn rvi_non_finite_values_keep_reference_behavior() {
        let input = [10.0, 11.0, f64::NAN, 13.0, 12.0, 14.0, 15.0, 13.0];
        let actual = rvi_profile(&input, 2, 2).unwrap();
        let expected = rvi_profile_reference(&input, 2, 2).unwrap();
        for (&actual, &expected) in actual.iter().zip(expected.iter()) {
            assert!(actual.is_nan() && expected.is_nan() || actual == expected);
        }
    }

    #[test]
    fn percentile_does_not_panic_on_nan_input() {
        // Regression: the window sort used `partial_cmp(...).unwrap()`, which
        // panics when the input contains NaN. It now uses `total_cmp`.
        let input = [1.0, 2.0, f64::NAN, 4.0, 5.0, 6.0, 7.0];
        let out = percentile(&input, 3, 50.0).expect("percentile must not panic on NaN");
        assert_eq!(out.len(), input.len());
    }

    #[test]
    fn percentile_selection_matches_sorted_rank_for_all_ranks() {
        let input = (0..257)
            .map(|i| ((i * 37 % 101) as f64 - 50.0) / 7.0)
            .collect::<Vec<_>>();
        for p in [2, 7, 30, 64] {
            for pct in [0.0, 1.0, 25.0, 50.0, 99.0, 100.0] {
                let actual = percentile(&input, p, pct).unwrap();
                let rank = ((pct * p as f64 / 100.0).ceil() as usize).clamp(1, p);
                for i in p - 1..input.len() {
                    let mut sorted = input[i + 1 - p..=i].to_vec();
                    sorted.sort_by(f64::total_cmp);
                    assert_eq!(actual[i], sorted[rank - 1], "p={p}, pct={pct}, i={i}");
                }
            }
        }
    }

    #[test]
    fn vhf_linear_rolling_path_matches_window_definition() {
        let input = (0..512)
            .map(|i| (i as f64 * 0.17).sin() * 3.0 + i as f64 * 0.01)
            .collect::<Vec<_>>();
        for p in [2, 3, 14, 28, 127] {
            let actual = vhf(&input, p).unwrap();
            for i in p..input.len() {
                let window = &input[i + 1 - p..=i];
                let highest = window.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let lowest = window.iter().copied().fold(f64::INFINITY, f64::min);
                let changes = (i + 1 - p..=i)
                    .map(|j| (input[j] - input[j - 1]).abs())
                    .sum::<f64>();
                let expected = if changes == 0.0 {
                    0.0
                } else {
                    (highest - lowest) / changes
                };
                assert!((actual[i] - expected).abs() <= 1e-12);
            }
        }
    }

    #[test]
    fn vhf_non_finite_input_retains_reference_recovery_behavior() {
        let input = [1.0, 2.0, 3.0, f64::NAN, 5.0, 6.0, 7.0, 8.0];
        let actual = vhf(&input, 3).unwrap();
        assert!(actual[3].is_nan());
        assert!(actual[4].is_nan());
        assert!(actual[5].is_nan());
        assert!(actual[6].is_nan());
        assert!((actual[7] - 2.0 / 3.0).abs() < f64::EPSILON);
    }
}
