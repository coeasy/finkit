//! Extended momentum and oscillator indicators.
//!
//! Implements AO, Fisher Transform, TSI, Coppock Curve, KST, STC, CHOP,
//! Connors RSI, Stochastic RSI, and Relative Vigor Index (RVI).

use crate::error::{Result, TaError};
use crate::indicators::rsi;
use crate::math::moving_avg::sma;
use crate::math::statistics::{rolling_max, rolling_min};
use crate::utils::{init_output, smoothing_factor, true_range, validate_input};
use ndarray::Array1;
fn validate_hl(high: &[f64], low: &[f64]) -> Result<()> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    Ok(())
}

fn validate_hlc(high: &[f64], low: &[f64], close: &[f64]) -> Result<()> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    Ok(())
}

fn validate_period(name: &str, period: usize) -> Result<()> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: name.to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    Ok(())
}

fn validate_ohlc(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<()> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    Ok(())
}

/// Compute consecutive up/down streak from close prices.
///
/// Positive streak counts consecutive higher closes; negative counts consecutive lower closes.
fn compute_streak(close: &[f64]) -> Vec<f64> {
    let len = close.len();
    let mut streak = vec![0.0; len];
    for i in 1..len {
        if close[i] > close[i - 1] {
            streak[i] = if streak[i - 1] > 0.0 {
                streak[i - 1] + 1.0
            } else {
                1.0
            };
        } else if close[i] < close[i - 1] {
            streak[i] = if streak[i - 1] < 0.0 {
                streak[i - 1] - 1.0
            } else {
                -1.0
            };
        }
    }
    streak
}

/// Percentile rank (0–100) of `data[i]` within the trailing `period` values.
fn percentile_rank(data: &[f64], i: usize, period: usize) -> f64 {
    let start = i + 1 - period;
    let current = data[i];
    if current.is_nan() {
        return f64::NAN;
    }
    let count_below = data[start..i]
        .iter()
        .filter(|&&v| !v.is_nan() && v < current)
        .count();
    count_below as f64 / (period - 1) as f64 * 100.0
}

#[inline]
fn wavg4(a: f64, b: f64, c: f64, d: f64) -> f64 {
    (a + 2.0 * b + 2.0 * c + d) / 6.0
}

/// Awesome Oscillator (AO) — Bill Williams
///
/// AO = SMA(Median Price, fast) - SMA(Median Price, slow)
/// where Median Price = (High + Low) / 2.
pub fn ao(
    high: &[f64],
    low: &[f64],
    fast_period: usize,
    slow_period: usize,
) -> Result<Array1<f64>> {
    validate_hl(high, low)?;
    validate_period("fast_period", fast_period)?;
    validate_period("slow_period", slow_period)?;

    let max_period = fast_period.max(slow_period);
    validate_input(high.len(), max_period)?;

    let len = high.len();
    let median: Vec<f64> = high
        .iter()
        .zip(low.iter())
        .map(|(&h, &l)| (h + l) / 2.0)
        .collect();

    let fast_sma = sma(&median, fast_period)?;
    let slow_sma = sma(&median, slow_period)?;

    let mut output = init_output(len);
    let start = max_period - 1;
    for i in start..len {
        output[i] = fast_sma[i] - slow_sma[i];
    }

    Ok(output)
}

/// Fisher Transform result.
#[derive(Debug, Clone)]
pub struct FisherResult {
    /// Fisher line
    pub fisher: Array1<f64>,
    /// Signal line (previous Fisher value)
    pub signal: Array1<f64>,
}

const RING_STACK: usize = 64;
const FISHER_SMOOTH_NEW: f64 = 0.33;
const FISHER_SMOOTH_OLD: f64 = 0.67;
const FISHER_HALF: f64 = 0.5;

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn fisher_core(
    high: &[f64],
    low: &[f64],
    period: usize,
    fisher_out: &mut [f64],
    signal_out: &mut [f64],
    max_dq: &mut [usize],
    min_dq: &mut [usize],
    cap: usize,
) {
    let len = high.len();
    let (mut mh, mut mt) = (0usize, 0usize);
    let (mut nh, mut nt) = (0usize, 0usize);
    let mut value_prev = 0.0;
    let mut fisher_prev = 0.0;

    for i in 0..len {
        while mt > mh && high[max_dq[(mt - 1) % cap]] <= high[i] {
            mt -= 1;
        }
        max_dq[mt % cap] = i;
        mt += 1;
        if max_dq[mh % cap] + period <= i {
            mh += 1;
        }

        while nt > nh && low[min_dq[(nt - 1) % cap]] >= low[i] {
            nt -= 1;
        }
        min_dq[nt % cap] = i;
        nt += 1;
        if min_dq[nh % cap] + period <= i {
            nh += 1;
        }

        if i >= period - 1 {
            let hi = high[max_dq[mh % cap]];
            let lo = low[min_dq[nh % cap]];
            let mid = (high[i] + low[i]) * FISHER_HALF;
            let range = hi - lo;

            let normalized = if range > 1e-15 {
                (mid - lo) / range * 2.0 - 1.0
            } else {
                0.0
            };

            let mut value = normalized.mul_add(FISHER_SMOOTH_NEW, value_prev * FISHER_SMOOTH_OLD);
            value = value.clamp(-0.999, 0.999);
            value_prev = value;

            signal_out[i] = fisher_prev;
            let ratio = (1.0 + value) / (1.0 - value);
            let fisher_val = ratio.ln().mul_add(FISHER_HALF, fisher_prev * FISHER_HALF);
            fisher_prev = fisher_val;
            fisher_out[i] = fisher_val;
        }
    }
}

/// Ehlers Fisher Transform
///
/// Uses midpoint `(High + Low) / 2` as the price input.
#[inline]
pub fn fisher(high: &[f64], low: &[f64], period: usize) -> Result<FisherResult> {
    validate_hl(high, low)?;
    validate_period("period", period)?;
    validate_input(high.len(), period)?;

    let len = high.len();
    let mut fisher_out = Array1::from_elem(len, f64::NAN);
    let mut signal_out = Array1::from_elem(len, f64::NAN);
    let cap = period + 1;

    if cap <= RING_STACK {
        let mut max_dq = [0usize; RING_STACK];
        let mut min_dq = [0usize; RING_STACK];
        fisher_core(
            high,
            low,
            period,
            fisher_out.as_slice_mut().unwrap(),
            signal_out.as_slice_mut().unwrap(),
            &mut max_dq,
            &mut min_dq,
            cap,
        );
    } else {
        let mut max_dq = vec![0usize; cap];
        let mut min_dq = vec![0usize; cap];
        fisher_core(
            high,
            low,
            period,
            fisher_out.as_slice_mut().unwrap(),
            signal_out.as_slice_mut().unwrap(),
            &mut max_dq,
            &mut min_dq,
            cap,
        );
    }

    Ok(FisherResult {
        fisher: fisher_out,
        signal: signal_out,
    })
}

/// True Strength Index (TSI)
///
/// TSI = 100 * EMA(EMA(momentum, long), short) / EMA(EMA(|momentum|, long), short)
pub fn tsi(input: &[f64], long_period: usize, short_period: usize) -> Result<Array1<f64>> {
    validate_period("long_period", long_period)?;
    validate_period("short_period", short_period)?;
    validate_input(input.len(), long_period + short_period)?;

    let len = input.len();
    let mut output = init_output(len);

    let long_k = smoothing_factor(long_period);
    let long_ok = 1.0 - long_k;
    let short_k = smoothing_factor(short_period);
    let short_ok = 1.0 - short_k;

    // Incremental EMA state for momentum pipeline
    let mut mom_long_sum = 0.0_f64;
    let mut mom_long_count = 0usize;
    let mut mom_long_ema = f64::NAN;
    let mut mom_short_sum = 0.0_f64;
    let mut mom_short_count = 0usize;
    let mut mom_short_ema = f64::NAN;

    // Incremental EMA state for abs_momentum pipeline
    let mut abs_long_sum = 0.0_f64;
    let mut abs_long_count = 0usize;
    let mut abs_long_ema = f64::NAN;
    let mut abs_short_sum = 0.0_f64;
    let mut abs_short_count = 0usize;
    let mut abs_short_ema = f64::NAN;

    for i in 1..len {
        let mom = input[i] - input[i - 1];
        let abs_mom = mom.abs();

        // Feed momentum into long EMA
        if mom_long_count < long_period {
            mom_long_sum += mom;
            mom_long_count += 1;
            if mom_long_count == long_period {
                mom_long_ema = mom_long_sum / long_period as f64;
                // Feed first value into short EMA
                mom_short_sum += mom_long_ema;
                mom_short_count += 1;
                if mom_short_count == short_period {
                    mom_short_ema = mom_short_sum / short_period as f64;
                }
            }
        } else {
            mom_long_ema = mom * long_k + mom_long_ema * long_ok;
            // Feed into short EMA
            if mom_short_count < short_period {
                mom_short_sum += mom_long_ema;
                mom_short_count += 1;
                if mom_short_count == short_period {
                    mom_short_ema = mom_short_sum / short_period as f64;
                }
            } else {
                mom_short_ema = mom_long_ema * short_k + mom_short_ema * short_ok;
            }
        }

        // Feed abs_momentum into long EMA
        if abs_long_count < long_period {
            abs_long_sum += abs_mom;
            abs_long_count += 1;
            if abs_long_count == long_period {
                abs_long_ema = abs_long_sum / long_period as f64;
                // Feed first value into short EMA
                abs_short_sum += abs_long_ema;
                abs_short_count += 1;
                if abs_short_count == short_period {
                    abs_short_ema = abs_short_sum / short_period as f64;
                }
            }
        } else {
            abs_long_ema = abs_mom * long_k + abs_long_ema * long_ok;
            // Feed into short EMA
            if abs_short_count < short_period {
                abs_short_sum += abs_long_ema;
                abs_short_count += 1;
                if abs_short_count == short_period {
                    abs_short_ema = abs_short_sum / short_period as f64;
                }
            } else {
                abs_short_ema = abs_long_ema * short_k + abs_short_ema * short_ok;
            }
        }

        // Compute TSI when both pipelines have valid values
        if !mom_short_ema.is_nan() && !abs_short_ema.is_nan() && abs_short_ema.abs() > 1e-15 {
            output[i] = 100.0 * mom_short_ema / abs_short_ema;
        }
    }

    Ok(output)
}

/// Coppock Curve
///
/// Coppock = WMA(ROC(long) + ROC(short), wma_period)
pub fn coppock(
    input: &[f64],
    wma_period: usize,
    long_roc: usize,
    short_roc: usize,
) -> Result<Array1<f64>> {
    validate_period("wma_period", wma_period)?;
    validate_period("long_roc", long_roc)?;
    validate_period("short_roc", short_roc)?;

    let roc_start = long_roc.max(short_roc);
    validate_input(input.len(), roc_start + wma_period)?;

    let len = input.len();
    let mut combined = init_output(len);
    for i in roc_start..len {
        let long_val = if input[i - long_roc].abs() > 1e-15 {
            (input[i] - input[i - long_roc]) / input[i - long_roc] * 100.0
        } else {
            f64::NAN
        };
        let short_val = if input[i - short_roc].abs() > 1e-15 {
            (input[i] - input[i - short_roc]) / input[i - short_roc] * 100.0
        } else {
            f64::NAN
        };
        if !long_val.is_nan() && !short_val.is_nan() {
            combined[i] = long_val + short_val;
        }
    }

    let mut output = init_output(len);
    let inv_weight_sum = 1.0 / (wma_period * (wma_period + 1) / 2) as f64;
    let p = wma_period as f64;
    let wma_first = roc_start + wma_period - 1;

    let combined_slice = combined.as_slice().unwrap();
    let win_start = wma_first + 1 - wma_period;
    let mut window_sum: f64 = combined_slice[win_start..=wma_first].iter().sum();
    let mut wsum: f64 = combined_slice[win_start..=wma_first]
        .iter()
        .enumerate()
        .map(|(j, &v)| (j + 1) as f64 * v)
        .sum();
    output[wma_first] = wsum * inv_weight_sum;
    let mut dirty = window_sum.is_nan();

    for i in (wma_first + 1)..len {
        let old = combined_slice[i - wma_period];
        let new = combined_slice[i];
        if dirty || old.is_nan() || new.is_nan() {
            let start = i + 1 - wma_period;
            let win = &combined_slice[start..=i];
            window_sum = win.iter().sum();
            wsum = win
                .iter()
                .enumerate()
                .map(|(j, &v)| (j + 1) as f64 * v)
                .sum();
            dirty = window_sum.is_nan();
        } else {
            wsum += p * new - window_sum;
            window_sum += new - old;
        }
        output[i] = wsum * inv_weight_sum;
    }

    Ok(output)
}

/// Know Sure Thing (KST) result.
#[derive(Debug, Clone)]
pub struct KstResult {
    /// KST line
    pub kst: Array1<f64>,
    /// Signal line
    pub signal: Array1<f64>,
}

#[inline]
fn roc_at(input: &[f64], i: usize, period: usize) -> f64 {
    if input[i - period].abs() > 1e-15 {
        (input[i] - input[i - period]) / input[i - period] * 100.0
    } else {
        f64::NAN
    }
}

/// Know Sure Thing (KST)
///
/// KST = SMA(ROC(roc1), sma1)*1 + SMA(ROC(roc2), sma2)*2
///     + SMA(ROC(roc3), sma3)*3 + SMA(ROC(roc4), sma4)*4
#[allow(clippy::too_many_arguments)]
pub fn kst(
    input: &[f64],
    roc1: usize,
    roc2: usize,
    roc3: usize,
    roc4: usize,
    sma1: usize,
    sma2: usize,
    sma3: usize,
    sma4: usize,
    signal_period: usize,
) -> Result<KstResult> {
    let rocs = [roc1, roc2, roc3, roc4];
    let smas = [sma1, sma2, sma3, sma4];
    let weights = [1.0, 2.0, 3.0, 4.0];

    for (i, &r) in rocs.iter().enumerate() {
        validate_period(&format!("roc{}", i + 1), r)?;
        validate_period(&format!("sma{}", i + 1), smas[i])?;
    }
    validate_period("signal_period", signal_period)?;

    let kst_start = rocs
        .iter()
        .zip(smas.iter())
        .map(|(&r, &s)| r + s - 1)
        .max()
        .unwrap_or(0);
    validate_input(input.len(), kst_start + signal_period)?;

    let len = input.len();
    let mut kst_out = Array1::from_elem(len, f64::NAN);
    let mut signal_out = Array1::from_elem(len, f64::NAN);

    // Per-component incremental SMA state using ring buffers
    let mut roc_rings: [Vec<f64>; 4] = std::array::from_fn(|j| vec![f64::NAN; smas[j]]);
    let mut ring_pos = [0usize; 4];
    let mut ring_count = [0usize; 4];
    let mut ring_sum = [0.0f64; 4];
    let mut ring_nan_count = [0usize; 4];
    let mut component_val = [f64::NAN; 4];

    // Signal SMA ring buffer
    let mut sig_ring = vec![f64::NAN; signal_period];
    let mut sig_pos = 0usize;
    let mut sig_count = 0usize;
    let mut sig_sum = 0.0f64;
    let mut sig_nan_count = 0usize;

    for i in 0..len {
        // Compute ROC and feed into incremental SMA for each component
        for j in 0..4 {
            if i < rocs[j] {
                continue;
            }
            let roc_val = roc_at(input, i, rocs[j]);
            let p = ring_pos[j];

            if ring_count[j] >= smas[j] {
                // Evict oldest value
                let old = roc_rings[j][p];
                if old.is_nan() {
                    ring_nan_count[j] -= 1;
                } else {
                    ring_sum[j] -= old;
                }
            } else {
                ring_count[j] += 1;
            }

            // Add new value
            if roc_val.is_nan() {
                ring_nan_count[j] += 1;
            } else {
                ring_sum[j] += roc_val;
            }
            roc_rings[j][p] = roc_val;
            ring_pos[j] = (p + 1) % smas[j];

            // Compute SMA if window is full and no NaN
            if ring_count[j] >= smas[j] && ring_nan_count[j] == 0 {
                component_val[j] = ring_sum[j] / smas[j] as f64;
            } else {
                component_val[j] = f64::NAN;
            }
        }

        // Compute KST
        let mut all_valid = true;
        let mut kst_val = 0.0;
        for j in 0..4 {
            if component_val[j].is_nan() {
                all_valid = false;
                break;
            }
            kst_val += component_val[j] * weights[j];
        }
        if all_valid {
            kst_out[i] = kst_val;
        }

        // Feed KST into signal SMA ring buffer
        let sp = sig_pos;
        if sig_count >= signal_period {
            let old = sig_ring[sp];
            if old.is_nan() {
                sig_nan_count -= 1;
            } else {
                sig_sum -= old;
            }
        } else {
            sig_count += 1;
        }

        if kst_out[i].is_nan() {
            sig_nan_count += 1;
        } else {
            sig_sum += kst_out[i];
        }
        sig_ring[sp] = kst_out[i];
        sig_pos = (sp + 1) % signal_period;

        if sig_count >= signal_period && sig_nan_count == 0 {
            signal_out[i] = sig_sum / signal_period as f64;
        }
    }

    Ok(KstResult {
        kst: kst_out,
        signal: signal_out,
    })
}

const STC_SMOOTH_HALF: f64 = 0.5;
const STC_STOCH_MID: f64 = 50.0;
const STC_STOCH_SCALE: f64 = 100.0;

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn stc_core(
    input: &[f64],
    output: &mut [f64],
    fast_period: usize,
    slow_period: usize,
    cycle: usize,
    cap: usize,
    macd_ring: &mut [f64],
    s1_ring: &mut [f64],
    macd_write: &mut [usize],
    s1_write: &mut [usize],
    max1: &mut [usize],
    min1: &mut [usize],
    max2: &mut [usize],
    min2: &mut [usize],
    k_fast: f64,
    k_slow: f64,
    ok_fast: f64,
    ok_slow: f64,
) {
    let len = input.len();
    let (mut mh1, mut mt1) = (0usize, 0usize);
    let (mut nh1, mut nt1) = (0usize, 0usize);
    let (mut mh2, mut mt2) = (0usize, 0usize);
    let (mut nh2, mut nt2) = (0usize, 0usize);

    let mut sum_f = 0.0;
    let mut sum_s = 0.0;
    let mut ema_f = f64::NAN;
    let mut ema_s = f64::NAN;
    let mut smooth1_prev = f64::NAN;
    let mut smooth2_prev = f64::NAN;
    let mut macd_count = 0usize;
    let mut s1_count = 0usize;

    for i in 0..len {
        let v = input[i];
        if i < fast_period {
            sum_f += v;
            if i == fast_period - 1 {
                ema_f = sum_f / fast_period as f64;
            }
        } else {
            ema_f = v.mul_add(k_fast, ema_f * ok_fast);
        }
        if i < slow_period {
            sum_s += v;
            if i == slow_period - 1 {
                ema_s = sum_s / slow_period as f64;
            }
        } else {
            ema_s = v.mul_add(k_slow, ema_s * ok_slow);
        }

        if ema_f.is_nan() || ema_s.is_nan() {
            continue;
        }

        let macd_val = ema_f - ema_s;
        let mi = macd_count % cap;
        macd_ring[mi] = macd_val;
        macd_write[mi] = macd_count;

        while mt1 > mh1 && macd_ring[max1[(mt1 - 1) % cap]] <= macd_val {
            mt1 -= 1;
        }
        max1[mt1 % cap] = mi;
        mt1 += 1;
        if macd_write[max1[mh1 % cap]] + cycle <= macd_count {
            mh1 += 1;
        }

        while nt1 > nh1 && macd_ring[min1[(nt1 - 1) % cap]] >= macd_val {
            nt1 -= 1;
        }
        min1[nt1 % cap] = mi;
        nt1 += 1;
        if macd_write[min1[nh1 % cap]] + cycle <= macd_count {
            nh1 += 1;
        }

        macd_count += 1;

        if macd_count >= cycle {
            let hi = macd_ring[max1[mh1 % cap]];
            let lo = macd_ring[min1[nh1 % cap]];
            let range = hi - lo;
            let k1 = if range > 1e-15 {
                (macd_val - lo) / range * STC_STOCH_SCALE
            } else {
                STC_STOCH_MID
            };
            smooth1_prev = if smooth1_prev.is_nan() {
                k1
            } else {
                k1.mul_add(STC_SMOOTH_HALF, smooth1_prev * STC_SMOOTH_HALF)
            };

            let si = s1_count % cap;
            s1_ring[si] = smooth1_prev;
            s1_write[si] = s1_count;

            while mt2 > mh2 && s1_ring[max2[(mt2 - 1) % cap]] <= smooth1_prev {
                mt2 -= 1;
            }
            max2[mt2 % cap] = si;
            mt2 += 1;
            if s1_write[max2[mh2 % cap]] + cycle <= s1_count {
                mh2 += 1;
            }

            while nt2 > nh2 && s1_ring[min2[(nt2 - 1) % cap]] >= smooth1_prev {
                nt2 -= 1;
            }
            min2[nt2 % cap] = si;
            nt2 += 1;
            if s1_write[min2[nh2 % cap]] + cycle <= s1_count {
                nh2 += 1;
            }

            s1_count += 1;

            if s1_count >= cycle {
                let hi2 = s1_ring[max2[mh2 % cap]];
                let lo2 = s1_ring[min2[nh2 % cap]];
                let range2 = hi2 - lo2;
                let k2 = if range2 > 1e-15 {
                    (smooth1_prev - lo2) / range2 * STC_STOCH_SCALE
                } else {
                    STC_STOCH_MID
                };
                smooth2_prev = if smooth2_prev.is_nan() {
                    k2
                } else {
                    k2.mul_add(STC_SMOOTH_HALF, smooth2_prev * STC_SMOOTH_HALF)
                };
                output[i] = smooth2_prev;
            }
        }
    }
}

/// Schaff Trend Cycle (STC)
///
/// Applies MACD, double stochastic smoothing with factor 0.5.
/// Single-pass fused pipeline: EMA→MACD→stoch1→smooth1→stoch2→smooth2.
pub fn stc(
    input: &[f64],
    fast_period: usize,
    slow_period: usize,
    cycle: usize,
) -> Result<Array1<f64>> {
    validate_period("fast_period", fast_period)?;
    validate_period("slow_period", slow_period)?;
    validate_period("cycle", cycle)?;
    validate_input(input.len(), slow_period + 2 * cycle)?;

    let len = input.len();
    let mut output = Array1::from_elem(len, f64::NAN);

    let k_fast = 2.0 / (fast_period as f64 + 1.0);
    let k_slow = 2.0 / (slow_period as f64 + 1.0);
    let ok_fast = 1.0 - k_fast;
    let ok_slow = 1.0 - k_slow;
    let cap = cycle + 1;

    if cap <= RING_STACK {
        let mut macd_ring = [f64::NAN; RING_STACK];
        let mut s1_ring = [f64::NAN; RING_STACK];
        let mut macd_write = [0usize; RING_STACK];
        let mut s1_write = [0usize; RING_STACK];
        let mut max1 = [0usize; RING_STACK];
        let mut min1 = [0usize; RING_STACK];
        let mut max2 = [0usize; RING_STACK];
        let mut min2 = [0usize; RING_STACK];
        stc_core(
            input,
            output.as_slice_mut().unwrap(),
            fast_period,
            slow_period,
            cycle,
            cap,
            &mut macd_ring,
            &mut s1_ring,
            &mut macd_write,
            &mut s1_write,
            &mut max1,
            &mut min1,
            &mut max2,
            &mut min2,
            k_fast,
            k_slow,
            ok_fast,
            ok_slow,
        );
    } else {
        let mut macd_ring = vec![f64::NAN; cap];
        let mut s1_ring = vec![f64::NAN; cap];
        let mut macd_write = vec![0usize; cap];
        let mut s1_write = vec![0usize; cap];
        let mut max1 = vec![0usize; cap];
        let mut min1 = vec![0usize; cap];
        let mut max2 = vec![0usize; cap];
        let mut min2 = vec![0usize; cap];
        stc_core(
            input,
            output.as_slice_mut().unwrap(),
            fast_period,
            slow_period,
            cycle,
            cap,
            &mut macd_ring,
            &mut s1_ring,
            &mut macd_write,
            &mut s1_write,
            &mut max1,
            &mut min1,
            &mut max2,
            &mut min2,
            k_fast,
            k_slow,
            ok_fast,
            ok_slow,
        );
    }

    Ok(output)
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn chop_core(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    output: &mut [f64],
    inv_log_period: f64,
    max_dq: &mut [usize],
    min_dq: &mut [usize],
    cap: usize,
    tr_ring: &mut [f64],
) {
    let len = high.len();
    let (mut mh, mut mt) = (0usize, 0usize);
    let (mut nh, mut nt) = (0usize, 0usize);
    let mut tr_sum = 0.0;
    let mut ring_pos = 0usize;

    for i in 0..len {
        let tr_i = if i == 0 {
            high[0] - low[0]
        } else {
            true_range(high[i], low[i], close[i - 1])
        };

        if i < period {
            tr_ring[i] = tr_i;
            tr_sum += tr_i;
        } else {
            tr_sum += tr_i - tr_ring[ring_pos];
            tr_ring[ring_pos] = tr_i;
            ring_pos += 1;
            if ring_pos == period {
                ring_pos = 0;
            }
        }

        while mt > mh && high[max_dq[(mt - 1) % cap]] <= high[i] {
            mt -= 1;
        }
        max_dq[mt % cap] = i;
        mt += 1;
        if max_dq[mh % cap] + period <= i {
            mh += 1;
        }

        while nt > nh && low[min_dq[(nt - 1) % cap]] >= low[i] {
            nt -= 1;
        }
        min_dq[nt % cap] = i;
        nt += 1;
        if min_dq[nh % cap] + period <= i {
            nh += 1;
        }

        if i >= period - 1 {
            let range = high[max_dq[mh % cap]] - low[min_dq[nh % cap]];
            let ratio = tr_sum / range;
            let chop_val = ratio.log10() * inv_log_period;
            if range > 1e-15 && tr_sum > 0.0 {
                output[i] = chop_val;
            }
        }
    }
}

/// Choppiness Index (CHOP)
///
/// CHOP = 100 * LOG10(SUM(TR, period) / (Highest High - Lowest Low)) / LOG10(period)
#[inline]
pub fn chop(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_hlc(high, low, close)?;
    validate_period("period", period)?;
    validate_input(high.len(), period)?;

    let len = high.len();
    let inv_log_period = 100.0 / (period as f64).log10();
    let mut output = Array1::from_elem(len, f64::NAN);
    let cap = period + 1;

    if cap <= RING_STACK && period <= RING_STACK {
        let mut max_dq = [0usize; RING_STACK];
        let mut min_dq = [0usize; RING_STACK];
        let mut tr_ring = [0.0; RING_STACK];
        chop_core(
            high,
            low,
            close,
            period,
            output.as_slice_mut().unwrap(),
            inv_log_period,
            &mut max_dq,
            &mut min_dq,
            cap,
            &mut tr_ring,
        );
    } else {
        let mut max_dq = vec![0usize; cap];
        let mut min_dq = vec![0usize; cap];
        let mut tr_ring = vec![0.0; period];
        chop_core(
            high,
            low,
            close,
            period,
            output.as_slice_mut().unwrap(),
            inv_log_period,
            &mut max_dq,
            &mut min_dq,
            cap,
            &mut tr_ring,
        );
    }

    Ok(output)
}

/// Connors RSI
///
/// Composite oscillator averaging three components:
/// 1. Standard RSI on close prices
/// 2. RSI applied to the up/down streak sequence
/// 3. Percentile rank of 1-day ROC over the lookback window
///
/// `ConnorsRSI = (RSI + StreakRSI + PctRank) / 3`
///
/// # Arguments
/// * `close` - Close prices
/// * `rsi_period` - RSI lookback (default: 3)
/// * `streak_period` - Streak RSI lookback (default: 2)
/// * `rank_period` - Percentile rank lookback (default: 100)
pub fn connors_rsi(
    close: &[f64],
    rsi_period: usize,
    streak_period: usize,
    rank_period: usize,
) -> Result<Array1<f64>> {
    validate_period("rsi_period", rsi_period)?;
    validate_period("streak_period", streak_period)?;
    validate_period("rank_period", rank_period)?;
    if rank_period < 2 {
        return Err(TaError::InvalidParameter {
            name: "rank_period".to_string(),
            constraint: "at least 2".to_string(),
        });
    }

    let min_len = rsi_period.max(streak_period + 1).max(rank_period + 1);
    validate_input(close.len(), min_len)?;

    let len = close.len();
    let price_rsi = rsi(close, rsi_period)?;
    let streak = compute_streak(close);
    let streak_rsi = rsi(&streak, streak_period)?;

    let mut roc = vec![f64::NAN; len];
    for i in 1..len {
        if close[i - 1].abs() > 1e-15 {
            roc[i] = (close[i] - close[i - 1]) / close[i - 1] * 100.0;
        }
    }

    let mut output = init_output(len);
    let start = rsi_period.max(streak_period + 1).max(rank_period);

    for i in start..len {
        let r = price_rsi[i];
        let sr = streak_rsi[i];
        let pr = percentile_rank(&roc, i, rank_period);
        if !r.is_nan() && !sr.is_nan() && !pr.is_nan() {
            output[i] = (r + sr + pr) / 3.0;
        }
    }

    Ok(output)
}

/// Stochastic RSI result.
#[derive(Debug, Clone)]
pub struct StochRsiResult {
    /// Smoothed Stochastic RSI %K line
    pub k: Array1<f64>,
    /// Stochastic RSI %D line (SMA of %K)
    pub d: Array1<f64>,
}

/// Stochastic RSI
///
/// Applies the Stochastic formula to RSI values, then smooths %K and %D.
///
/// `StochRSI_K = (RSI - min(RSI, stoch_period)) / (max(RSI, stoch_period) - min(RSI, stoch_period))`
///
/// # Arguments
/// * `close` - Close prices
/// * `rsi_period` - RSI lookback (default: 14)
/// * `stoch_period` - Stochastic lookback on RSI (default: 14)
/// * `k_period` - %K smoothing period (default: 3)
/// * `d_period` - %D smoothing period (default: 3)
#[deprecated(since = "0.2.0", note = "Use `stochrsi` (TA-Lib naming convention)")]
pub fn stoch_rsi(
    close: &[f64],
    rsi_period: usize,
    stoch_period: usize,
    k_period: usize,
    d_period: usize,
) -> Result<StochRsiResult> {
    validate_period("rsi_period", rsi_period)?;
    validate_period("stoch_period", stoch_period)?;
    validate_period("k_period", k_period)?;
    validate_period("d_period", d_period)?;
    validate_input(close.len(), rsi_period + stoch_period + k_period + d_period)?;

    let len = close.len();
    let rsi_vals = rsi(close, rsi_period)?;
    let rsi_slice = rsi_vals.as_slice().unwrap();

    let rsi_max = rolling_max(rsi_slice, stoch_period)?;
    let rsi_min = rolling_min(rsi_slice, stoch_period)?;

    let stoch_start = rsi_period + stoch_period - 1;

    // Inline incremental SMA for %K smoothing (NaN→0.0 directly, no intermediate Vec)
    let inv_k = 1.0 / k_period as f64;
    let mut k_sum = 0.0_f64;
    let mut k_buf = vec![0.0_f64; k_period]; // ring buffer for raw_k values
    let mut k_pos: usize = 0;
    let mut smoothed_k = vec![f64::NAN; len];

    // Inline incremental SMA for %D smoothing
    let inv_d = 1.0 / d_period as f64;
    let mut d_sum = 0.0_f64;
    let mut d_buf = vec![0.0_f64; d_period]; // ring buffer for smoothed_k values
    let mut d_pos: usize = 0;
    let mut smoothed_d = vec![f64::NAN; len];

    for i in 0..len {
        // Compute raw_k inline: NaN→0.0
        let raw_k = if i >= stoch_start
            && !rsi_slice[i].is_nan()
            && !rsi_max[i].is_nan()
            && !rsi_min[i].is_nan()
        {
            let range = rsi_max[i] - rsi_min[i];
            if range.abs() > 1e-15 {
                (rsi_slice[i] - rsi_min[i]) / range
            } else {
                0.5
            }
        } else {
            0.0
        };

        // Incremental SMA for %K
        k_sum += raw_k - k_buf[k_pos];
        k_buf[k_pos] = raw_k;
        k_pos += 1;
        if k_pos == k_period {
            k_pos = 0;
        }
        if i >= k_period - 1 {
            smoothed_k[i] = k_sum * inv_k;
        }

        // Incremental SMA for %D (feeds from smoothed_k)
        let sk_val = if smoothed_k[i].is_nan() {
            0.0
        } else {
            smoothed_k[i]
        };
        d_sum += sk_val - d_buf[d_pos];
        d_buf[d_pos] = sk_val;
        d_pos += 1;
        if d_pos == d_period {
            d_pos = 0;
        }
        if i >= d_period - 1 {
            smoothed_d[i] = d_sum * inv_d;
        }
    }

    // Build output arrays: NaN for warm-up, valid values after
    let mut out_k = init_output(len);
    let mut out_d = init_output(len);
    let k_start = stoch_start + k_period - 1;
    let d_start = k_start + d_period - 1;
    for i in k_start..len {
        if !smoothed_k[i].is_nan() {
            out_k[i] = smoothed_k[i];
        }
    }
    for i in d_start..len {
        if !smoothed_d[i].is_nan() {
            out_d[i] = smoothed_d[i];
        }
    }

    Ok(StochRsiResult { k: out_k, d: out_d })
}

/// Relative Vigor Index (RVI) result.
#[derive(Debug, Clone)]
pub struct RviResult {
    /// RVI line
    pub rvi: Array1<f64>,
    /// Signal line (symmetric 4-bar weighted average of RVI)
    pub signal: Array1<f64>,
}

/// Relative Vigor Index (RVI)
///
/// Compares the tendency of price to close above/below its open against the
/// total price range, using symmetric 4-bar weighted averages.
///
/// `numerator = SMA4(close - open)`, `denominator = SMA4(high - low)`,
/// `RVI = SMA(numerator / denominator, period)`, `Signal = SMA4(RVI)`.
///
/// # Arguments
/// * `open` - Open prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - SMA period for smoothing the vigor ratio (default: 10)
pub fn rvi(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
) -> Result<RviResult> {
    validate_ohlc(open, high, low, close)?;
    validate_period("period", period)?;
    validate_input(open.len(), period + 3)?;

    let len = open.len();
    let wavg_start = 3;
    let mut ratio = vec![f64::NAN; len];

    for i in wavg_start..len {
        let num = wavg4(
            close[i] - open[i],
            close[i - 1] - open[i - 1],
            close[i - 2] - open[i - 2],
            close[i - 3] - open[i - 3],
        );
        let denom = wavg4(
            high[i] - low[i],
            high[i - 1] - low[i - 1],
            high[i - 2] - low[i - 2],
            high[i - 3] - low[i - 3],
        );
        if denom.abs() > 1e-15 {
            ratio[i] = num / denom;
        }
    }

    let rvi_start = wavg_start + period - 1;
    let mut rvi_line = Array1::from_elem(len, f64::NAN);
    let inv_period = 1.0 / period as f64;
    // Incremental SMA with NaN tracking: O(1) per bar instead of O(period)
    let mut sum = 0.0_f64;
    let mut nan_count: usize = 0;
    for i in wavg_start..=rvi_start {
        if ratio[i].is_nan() {
            nan_count += 1;
        } else {
            sum += ratio[i];
        }
    }
    if nan_count == 0 {
        rvi_line[rvi_start] = sum * inv_period;
    }
    for i in (rvi_start + 1)..len {
        let old = ratio[i - period];
        let new = ratio[i];
        if old.is_nan() {
            nan_count -= 1;
        } else {
            sum -= old;
        }
        if new.is_nan() {
            nan_count += 1;
        } else {
            sum += new;
        }
        if nan_count == 0 {
            rvi_line[i] = sum * inv_period;
        }
    }

    let mut signal = Array1::from_elem(len, f64::NAN);
    let signal_start = rvi_start + 3;
    let rvi_slice = rvi_line.as_slice().unwrap();
    for i in signal_start..len {
        let r0 = rvi_slice[i - 3];
        let r1 = rvi_slice[i - 2];
        let r2 = rvi_slice[i - 1];
        let r3 = rvi_slice[i];
        if !r0.is_nan() && !r1.is_nan() && !r2.is_nan() && !r3.is_nan() {
            signal[i] = wavg4(r0, r1, r2, r3);
        }
    }

    Ok(RviResult {
        rvi: rvi_line,
        signal,
    })
}

// ============================================================================
// Chande Kroll Stop
// ============================================================================

/// Chande Kroll Stop result.
pub struct ChandeKrollStopResult {
    /// Stop for long positions (support line).
    pub stop_long: Array1<f64>,
    /// Stop for short positions (resistance line).
    pub stop_short: Array1<f64>,
}

/// Chande Kroll Stop
///
/// A volatility-based trailing stop that adapts to market conditions using ATR.
/// Produces two lines: stop_long (below price, support) and stop_short
/// (above price, resistance).
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `atr_period` - ATR period (typically 10)
/// * `atr_mult` - ATR multiplier for initial stop (typically 1)
/// * `stop_period` - Period for highest/lowest stop (typically 9)
pub fn chande_kroll_stop(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    atr_period: usize,
    atr_mult: usize,
    stop_period: usize,
) -> Result<ChandeKrollStopResult> {
    validate_hlc(high, low, close)?;
    let min_len = atr_period + stop_period;
    validate_input(high.len(), min_len)?;

    let len = high.len();
    let atr = crate::indicators::atr(high, low, close, atr_period)?;

    let mut first_high_stop = init_output(len);
    let mut first_low_stop = init_output(len);

    for i in 0..len {
        if !atr[i].is_nan() {
            first_high_stop[i] = high[i] - atr_mult as f64 * atr[i];
            first_low_stop[i] = low[i] + atr_mult as f64 * atr[i];
        }
    }

    let mut stop_long = init_output(len);
    let mut stop_short = init_output(len);

    for i in (stop_period - 1)..len {
        let mut max_val = f64::NEG_INFINITY;
        let mut min_val = f64::INFINITY;
        for j in (i + 1 - stop_period)..=i {
            if !first_high_stop[j].is_nan() && first_high_stop[j] > max_val {
                max_val = first_high_stop[j];
            }
            if !first_low_stop[j].is_nan() && first_low_stop[j] < min_val {
                min_val = first_low_stop[j];
            }
        }
        if max_val > f64::NEG_INFINITY {
            stop_long[i] = max_val;
        }
        if min_val < f64::INFINITY {
            stop_short[i] = min_val;
        }
    }

    Ok(ChandeKrollStopResult {
        stop_long,
        stop_short,
    })
}

// ============================================================================
// TTM Squeeze
// ============================================================================

/// TTM Squeeze Momentum result.
pub struct TtmSqueezeResult {
    /// Momentum histogram values.
    pub momentum: Array1<f64>,
    /// Squeeze state: 1.0 when Bollinger Bands are inside Keltner Channels, 0.0 otherwise.
    pub squeeze_on: Array1<f64>,
}

/// TTM Squeeze Momentum
///
/// Detects low-volatility "squeeze" periods (Bollinger inside Keltner) and
/// measures momentum direction via a linear regression of the midline difference.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `bb_period` - Bollinger Bands period (typically 20)
/// * `bb_mult` - Bollinger Bands std dev multiplier (typically 2.0)
/// * `kc_period` - Keltner Channel period (typically 20)
/// * `kc_mult` - Keltner Channel ATR multiplier (typically 1.5)
pub fn ttm_squeeze(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    bb_period: usize,
    bb_mult: f64,
    kc_period: usize,
    kc_mult: f64,
) -> Result<TtmSqueezeResult> {
    validate_hlc(high, low, close)?;
    let min_len = bb_period.max(kc_period);
    validate_input(high.len(), min_len)?;

    let len = high.len();

    let bb = crate::indicators::bbands(close, bb_period, bb_mult, bb_mult)?;
    let atr = crate::indicators::atr(high, low, close, kc_period)?;
    let kc_mid = sma(close, kc_period)?;

    let mut squeeze_on = init_output(len);
    let mut momentum = init_output(len);

    for i in 0..len {
        if !bb.upper[i].is_nan() && !atr[i].is_nan() && !kc_mid[i].is_nan() {
            let kc_upper = kc_mid[i] + kc_mult * atr[i];
            let kc_lower = kc_mid[i] - kc_mult * atr[i];
            squeeze_on[i] = if bb.lower[i] > kc_lower && bb.upper[i] < kc_upper {
                1.0
            } else {
                0.0
            };

            let bb_kc_mid = (kc_mid[i] + (bb.upper[i] + bb.lower[i]) / 2.0) / 2.0;
            momentum[i] = close[i] - bb_kc_mid;
        }
    }

    Ok(TtmSqueezeResult {
        momentum,
        squeeze_on,
    })
}

// ============================================================================
// Williams Fractal
// ============================================================================

/// Williams Fractal result.
pub struct WilliamsFractalResult {
    /// Up fractal values (NaN if not a fractal point).
    pub fractal_up: Array1<f64>,
    /// Down fractal values (NaN if not a fractal point).
    pub fractal_down: Array1<f64>,
}

/// Williams Fractal
///
/// Identifies swing high/low points (fractals). A fractal up occurs when a bar's
/// high is the highest among `n` bars on each side. A fractal down occurs when a
/// bar's low is the lowest among `n` bars on each side.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `n` - Number of bars on each side (typically 2)
pub fn williams_fractal(high: &[f64], low: &[f64], n: usize) -> Result<WilliamsFractalResult> {
    validate_hl(high, low)?;
    let min_len = 2 * n + 1;
    validate_input(high.len(), min_len)?;

    let len = high.len();
    let mut fractal_up = init_output(len);
    let mut fractal_down = init_output(len);

    for i in n..(len - n) {
        let mut is_up = true;
        let mut is_down = true;

        for j in 1..=n {
            if high[i - j] >= high[i] || high[i + j] >= high[i] {
                is_up = false;
            }
            if low[i - j] <= low[i] || low[i + j] <= low[i] {
                is_down = false;
            }
        }

        if is_up {
            fractal_up[i] = high[i];
        }
        if is_down {
            fractal_down[i] = low[i];
        }
    }

    Ok(WilliamsFractalResult {
        fractal_up,
        fractal_down,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_ao_basic() {
        let high: Vec<f64> = (0..40).map(|i| 10.0 + i as f64 * 0.5).collect();
        let low: Vec<f64> = high.iter().map(|h| h - 1.0).collect();
        let result = ao(&high, &low, 5, 34).unwrap();

        assert!(result[32].is_nan());
        assert!(!result[33].is_nan());

        let median: Vec<f64> = high
            .iter()
            .zip(low.iter())
            .map(|(&h, &l)| (h + l) / 2.0)
            .collect();
        let fast = sma(&median, 5).unwrap();
        let slow = sma(&median, 34).unwrap();
        assert_relative_eq!(result[33], fast[33] - slow[33], epsilon = 1e-10);
    }

    #[test]
    fn test_fisher_basic() {
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0];
        let low = vec![8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let result = fisher(&high, &low, 3).unwrap();

        assert!(result.fisher[1].is_nan());
        assert!(!result.fisher[2].is_nan());
        assert!(result.fisher[2].abs() <= 10.0);
        assert!(result.signal[3].is_nan() || result.signal[3].abs() <= 10.0);
    }

    #[test]
    fn test_tsi_basic() {
        let input: Vec<f64> = (0..50).map(|i| 100.0 + i as f64).collect();
        let result = tsi(&input, 25, 13).unwrap();

        let offset = 1 + (25 - 1) + (13 - 1);
        let last = result.len() - 1;
        assert!(!result[last].is_nan());
        assert!(result[last] > 0.0);
        let first_valid = result.iter().position(|v| !v.is_nan());
        assert!(first_valid.is_some());
        assert!(first_valid.unwrap() >= offset);
    }

    #[test]
    fn test_coppock_basic() {
        let input: Vec<f64> = (0..30).map(|i| 100.0 + i as f64 * 2.0).collect();
        let result = coppock(&input, 10, 14, 11).unwrap();

        let first_valid = 14 + 10 - 1;
        assert!(result[first_valid - 1].is_nan());
        assert!(!result[first_valid].is_nan());
    }

    #[test]
    fn test_kst_basic() {
        let input: Vec<f64> = (0..60).map(|i| 100.0 + i as f64).collect();
        let result = kst(&input, 10, 15, 20, 30, 10, 10, 10, 15, 9).unwrap();

        let kst_start = 30 + 15 - 1; // index 44
        assert!(result.kst[kst_start - 1].is_nan());
        assert!(!result.kst[kst_start].is_nan());

        let signal_start = kst_start + 9 - 1; // index 52
        assert!(result.signal[signal_start - 1].is_nan());
        assert!(!result.signal[signal_start].is_nan());
    }

    #[test]
    fn test_stc_basic() {
        let input: Vec<f64> = (0..80)
            .map(|i| 100.0 + (i as f64 * 0.3).sin() * 5.0)
            .collect();
        let result = stc(&input, 23, 50, 10).unwrap();

        assert!(result.iter().any(|v| !v.is_nan()));
        for v in result.iter().filter(|v| !v.is_nan()) {
            assert!(*v >= 0.0 && *v <= 100.0);
        }
    }

    #[test]
    fn test_chop_basic() {
        let high: Vec<f64> = (0..20).map(|i| 10.0 + i as f64).collect();
        let low: Vec<f64> = high.iter().map(|h| h - 1.0).collect();
        let close: Vec<f64> = high
            .iter()
            .zip(low.iter())
            .map(|(h, l)| (h + l) / 2.0)
            .collect();
        let result = chop(&high, &low, &close, 14).unwrap();

        assert!(result[12].is_nan());
        assert!(!result[13].is_nan());
        assert!(result[13] >= 0.0 && result[13] <= 100.0);
    }

    #[test]
    fn test_chop_trending_lower_than_choppy() {
        let len = 30;
        let high: Vec<f64> = (0..len).map(|i| 10.0 + i as f64 * 0.1).collect();
        let low: Vec<f64> = high.iter().map(|h| h - 0.5).collect();
        let close: Vec<f64> = high
            .iter()
            .zip(low.iter())
            .map(|(h, l)| (h + l) / 2.0)
            .collect();

        let trending = chop(&high, &low, &close, 14).unwrap();

        let mut choppy_high = vec![10.0; len];
        let mut choppy_low = vec![9.0; len];
        let mut choppy_close = vec![9.5; len];
        for i in 0..len {
            if i % 2 == 0 {
                choppy_high[i] = 12.0;
                choppy_low[i] = 8.0;
                choppy_close[i] = 11.0;
            } else {
                choppy_high[i] = 11.0;
                choppy_low[i] = 9.0;
                choppy_close[i] = 9.5;
            }
        }
        let choppy = chop(&choppy_high, &choppy_low, &choppy_close, 14).unwrap();

        let t_idx = len - 1;
        assert!(choppy[t_idx] > trending[t_idx]);
    }

    #[test]
    fn test_empty_and_invalid_input() {
        assert!(ao(&[], &[], 5, 34).is_err());
        assert!(fisher(&[1.0], &[1.0], 0).is_err());
        assert!(tsi(&[], 25, 13).is_err());
        assert!(coppock(&[], 10, 14, 11).is_err());
        assert!(kst(&[], 10, 15, 20, 30, 10, 10, 10, 15, 9).is_err());
        assert!(stc(&[], 23, 50, 10).is_err());
        assert!(chop(&[], &[], &[], 14).is_err());

        let high = vec![1.0, 2.0];
        let low = vec![1.0, 2.0];
        assert!(ao(&high, &low, 5, 34).is_err());
        assert!(fisher(&high, &low, 5).is_err());
    }

    #[test]
    fn test_mismatched_lengths() {
        let high = vec![1.0, 2.0, 3.0];
        let low = vec![1.0, 2.0];
        assert!(ao(&high, &low, 5, 34).is_err());
        assert!(fisher(&high, &low, 3).is_err());

        let close = vec![1.0, 2.0];
        assert!(chop(&high, &low, &close, 3).is_err());
    }

    #[test]
    fn test_connors_rsi_basic() {
        let close: Vec<f64> = (0..120)
            .map(|i| 100.0 + (i as f64 * 0.2).sin() * 5.0)
            .collect();
        let result = connors_rsi(&close, 3, 2, 100).unwrap();

        assert_eq!(result.len(), close.len());
        assert!(result[99].is_nan());
        assert!(!result[100].is_nan());
        assert!(result[100] >= 0.0 && result[100] <= 100.0);
    }

    #[test]
    fn test_connors_rsi_uptrend_high() {
        let close: Vec<f64> = (0..120).map(|i| 100.0 + i as f64).collect();
        let result = connors_rsi(&close, 3, 2, 100).unwrap();
        let last = result.len() - 1;
        assert!(!result[last].is_nan());
        assert!(result[last] > 50.0);
    }

    #[test]
    fn test_stoch_rsi_basic() {
        let close: Vec<f64> = (0..60)
            .map(|i| 100.0 + (i as f64 * 0.15).sin() * 10.0)
            .collect();
        let result = stoch_rsi(&close, 14, 14, 3, 3).unwrap();

        assert_eq!(result.k.len(), close.len());
        assert_eq!(result.d.len(), close.len());

        let k_start = 14 + 14 - 1 + 3 - 1;
        assert!(result.k[k_start - 1].is_nan());
        assert!(!result.k[k_start].is_nan());

        let d_start = k_start + 3 - 1;
        assert!(result.d[d_start - 1].is_nan());
        assert!(!result.d[d_start].is_nan());
    }

    #[test]
    fn test_stoch_rsi_range() {
        let close: Vec<f64> = (0..80)
            .map(|i| 100.0 + (i as f64 * 0.1).sin() * 8.0)
            .collect();
        let result = stoch_rsi(&close, 14, 14, 3, 3).unwrap();

        for v in result.k.iter().filter(|v| !v.is_nan()) {
            assert!(*v >= 0.0 && *v <= 1.0);
        }
        for v in result.d.iter().filter(|v| !v.is_nan()) {
            assert!(*v >= 0.0 && *v <= 1.0);
        }
    }

    #[test]
    fn test_rvi_basic() {
        let len = 30;
        let open: Vec<f64> = (0..len).map(|i| 100.0 + i as f64 * 0.1).collect();
        let high: Vec<f64> = open.iter().map(|o| o + 2.0).collect();
        let low: Vec<f64> = open.iter().map(|o| o - 1.0).collect();
        let close: Vec<f64> = open.iter().map(|o| o + 1.5).collect();

        let result = rvi(&open, &high, &low, &close, 10).unwrap();
        let first_rvi = 3 + 10 - 1;
        let first_signal = first_rvi + 3;

        assert_eq!(result.rvi.len(), len);
        assert!(result.rvi[first_rvi - 1].is_nan());
        assert!(!result.rvi[first_rvi].is_nan());
        assert!(result.signal[first_signal - 1].is_nan());
        assert!(!result.signal[first_signal].is_nan());
    }

    #[test]
    fn test_rvi_bullish_close_above_open() {
        let len = 25;
        let open = vec![100.0; len];
        let high: Vec<f64> = (0..len).map(|i| 102.0 + i as f64 * 0.05).collect();
        let low: Vec<f64> = (0..len).map(|i| 98.0 + i as f64 * 0.05).collect();
        let close: Vec<f64> = (0..len).map(|i| 101.5 + i as f64 * 0.05).collect();

        let result = rvi(&open, &high, &low, &close, 10).unwrap();
        let last = result.rvi.len() - 1;
        assert!(!result.rvi[last].is_nan());
        assert!(result.rvi[last] > 0.0);
    }

    #[test]
    fn test_connors_stoch_rvi_invalid() {
        assert!(connors_rsi(&[], 3, 2, 100).is_err());
        assert!(stoch_rsi(&[], 14, 14, 3, 3).is_err());
        assert!(rvi(&[1.0], &[1.0], &[1.0], &[1.0], 10).is_err());
        assert!(connors_rsi(&[1.0, 2.0], 3, 2, 100).is_err());
    }

    // ============ Chande Kroll Stop tests ============
    #[test]
    fn test_chande_kroll_stop_basic() {
        let high: Vec<f64> = (0..50)
            .map(|i| 110.0 + (i as f64 * 0.3).sin() * 5.0)
            .collect();
        let low: Vec<f64> = (0..50)
            .map(|i| 90.0 + (i as f64 * 0.3).sin() * 5.0)
            .collect();
        let close: Vec<f64> = (0..50)
            .map(|i| 100.0 + (i as f64 * 0.3).sin() * 5.0)
            .collect();
        let result = chande_kroll_stop(&high, &low, &close, 10, 1, 9).unwrap();
        assert_eq!(result.stop_long.len(), 50);
        assert_eq!(result.stop_short.len(), 50);
    }

    #[test]
    fn test_chande_kroll_stop_invalid() {
        assert!(chande_kroll_stop(&[], &[], &[], 10, 1, 9).is_err());
    }

    // ============ TTM Squeeze tests ============
    #[test]
    fn test_ttm_squeeze_basic() {
        let high: Vec<f64> = (0..60)
            .map(|i| 110.0 + (i as f64 * 0.2).sin() * 5.0)
            .collect();
        let low: Vec<f64> = (0..60)
            .map(|i| 90.0 + (i as f64 * 0.2).sin() * 5.0)
            .collect();
        let close: Vec<f64> = (0..60)
            .map(|i| 100.0 + (i as f64 * 0.2).sin() * 5.0)
            .collect();
        let result = ttm_squeeze(&high, &low, &close, 20, 2.0, 20, 1.5).unwrap();
        assert_eq!(result.momentum.len(), 60);
        assert_eq!(result.squeeze_on.len(), 60);
    }

    // ============ Williams Fractal tests ============
    #[test]
    fn test_williams_fractal_basic() {
        let high: Vec<f64> = vec![1.0, 2.0, 5.0, 3.0, 1.0, 1.0, 2.0, 1.0, 4.0, 2.0, 1.0];
        let low: Vec<f64> = vec![0.5, 1.5, 4.0, 2.0, 0.5, 0.5, 1.5, 0.5, 3.0, 1.5, 0.5];
        let result = williams_fractal(&high, &low, 2).unwrap();
        assert_eq!(result.fractal_up.len(), 11);
        assert_eq!(result.fractal_down.len(), 11);
        // Index 2 should be an up fractal (5.0 is highest of 5 bars centered at index 2)
        assert!(!result.fractal_up[2].is_nan(), "bar 2 should be up fractal");
    }

    #[test]
    fn test_williams_fractal_invalid() {
        assert!(williams_fractal(&[1.0, 2.0], &[0.5, 1.0], 2).is_err());
    }
}

/// Vortex Indicator result containing VI+ and VI- lines.
#[derive(Debug, Clone)]
pub struct VortexResult {
    /// Vortex Positive (VI+): upward trend strength
    pub vi_plus: Array1<f64>,
    /// Vortex Negative (VI-): downward trend strength
    pub vi_minus: Array1<f64>,
}

/// Vortex Indicator (VI)
///
/// Measures positive and negative trend movement by comparing current high/low
/// with previous low/high, normalized by True Range over a rolling period.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - Rolling sum period (typically 14)
///
/// # Returns
/// `VortexResult` with `vi_plus` and `vi_minus` arrays
///
/// # Examples
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0,
///                 46.5, 47.0, 46.5, 47.0, 47.5];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5,
///                44.0, 44.5, 44.0, 44.5, 45.0];
/// let close = vec![44.0, 44.5, 45.0, 44.0, 45.0, 44.5, 44.0, 43.5, 44.0, 44.5,
///                  45.0, 45.5, 45.0, 45.5, 46.0];
/// let result = indicators::vortex(&high, &low, &close, 7).unwrap();
/// assert_eq!(result.vi_plus.len(), 15);
/// ```
pub fn vortex(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<VortexResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period + 1)?;

    let len = high.len();
    let mut vi_plus = vec![f64::NAN; len];
    let mut vi_minus = vec![f64::NAN; len];

    let mut vm_plus_sum: f64 = 0.0;
    let mut vm_minus_sum: f64 = 0.0;
    let mut tr_sum: f64 = 0.0;

    let mut vm_plus_ring = vec![0.0_f64; period];
    let mut vm_minus_ring = vec![0.0_f64; period];
    let mut tr_ring = vec![0.0_f64; period];
    let mut ring_idx: usize = 0;

    for i in 1..len {
        let vm_p = (high[i] - low[i - 1]).abs();
        let vm_m = (low[i] - high[i - 1]).abs();
        let hl = high[i] - low[i];
        let hc = (high[i] - close[i - 1]).abs();
        let lc = (low[i] - close[i - 1]).abs();
        let tr = hl.max(hc).max(lc);

        vm_plus_sum += vm_p - vm_plus_ring[ring_idx];
        vm_minus_sum += vm_m - vm_minus_ring[ring_idx];
        tr_sum += tr - tr_ring[ring_idx];
        vm_plus_ring[ring_idx] = vm_p;
        vm_minus_ring[ring_idx] = vm_m;
        tr_ring[ring_idx] = tr;
        ring_idx += 1;
        if ring_idx == period {
            ring_idx = 0;
        }

        if i >= period {
            if tr_sum > 1e-15 {
                vi_plus[i] = vm_plus_sum / tr_sum;
                vi_minus[i] = vm_minus_sum / tr_sum;
            } else {
                vi_plus[i] = 0.0;
                vi_minus[i] = 0.0;
            }
        }
    }

    Ok(VortexResult {
        vi_plus: Array1::from_vec(vi_plus),
        vi_minus: Array1::from_vec(vi_minus),
    })
}

#[cfg(test)]
mod vortex_tests {
    use super::*;

    #[test]
    fn test_vortex_basic() {
        let high = vec![
            45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0, 46.5, 47.0, 46.5, 47.0,
            47.5, 47.0, 46.5, 47.0, 47.5, 48.0,
        ];
        let low = vec![
            43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5, 44.0, 44.5, 44.0, 44.5,
            45.0, 44.5, 44.0, 44.5, 45.0, 45.5,
        ];
        let close = vec![
            44.0, 44.5, 45.0, 44.0, 45.0, 44.5, 44.0, 43.5, 44.0, 44.5, 45.0, 45.5, 45.0, 45.5,
            46.0, 45.5, 45.0, 45.5, 46.0, 46.5,
        ];
        let result = vortex(&high, &low, &close, 14).unwrap();
        assert_eq!(result.vi_plus.len(), 20);
        assert_eq!(result.vi_minus.len(), 20);
        for i in 0..14 {
            assert!(result.vi_plus[i].is_nan());
            assert!(result.vi_minus[i].is_nan());
        }
        for i in 14..20 {
            assert!(result.vi_plus[i].is_finite());
            assert!(result.vi_minus[i].is_finite());
            assert!(result.vi_plus[i] > 0.0);
            assert!(result.vi_minus[i] > 0.0);
        }
    }

    #[test]
    fn test_vortex_invalid_input() {
        let high = vec![1.0, 2.0];
        let low = vec![0.5, 1.0];
        let close = vec![0.75, 1.5];
        assert!(vortex(&high, &low, &close, 14).is_err());
    }

    #[test]
    fn test_vortex_mismatched_lengths() {
        let high = vec![1.0, 2.0, 3.0];
        let low = vec![0.5, 1.0];
        let close = vec![0.75, 1.5, 2.5];
        assert!(vortex(&high, &low, &close, 2).is_err());
    }
}

// ============================================================================
// Inertia Indicator
// ============================================================================

/// Inertia Indicator (惯性指标)
///
/// Applies Time Series Forecast (linear regression) smoothing to the
/// Relative Volatility Index (RVI) ratio, producing a momentum oscillator
/// that measures trend inertia.
///
/// # Algorithm
/// 1. Compute the RVI ratio: wavg4(close-open) / wavg4(high-low) for each bar
/// 2. Compute a rolling SMA of the ratio over `rvi_period` bars
/// 3. Apply TSF (linear regression forecast) over `tsf_period` bars to the smoothed RVI
///
/// # Parameters
/// * `open` - Open prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `rvi_period` - Period for the RVI smoothing (default: 10)
/// * `tsf_period` - Period for the TSF linear regression (default: 14)
///
/// # Returns
/// Inertia values array. First `3 + rvi_period - 1 + tsf_period - 1` values are NaN.
pub fn inertia(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    rvi_period: usize,
    tsf_period: usize,
) -> Result<Array1<f64>> {
    validate_ohlc(open, high, low, close)?;
    if rvi_period < 1 {
        return Err(TaError::InvalidParameter {
            name: "rvi_period".to_string(),
            constraint: "at least 1".to_string(),
        });
    }
    if tsf_period < 2 {
        return Err(TaError::InvalidParameter {
            name: "tsf_period".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    let min_len = 3 + rvi_period + tsf_period;
    validate_input(open.len(), min_len)?;

    let len = open.len();
    let mut output = vec![f64::NAN; len];

    // Step 1: Compute RVI ratio with 4-bar weighted average (starting at index 3)
    let wavg_start = 3;
    let mut ratio = vec![f64::NAN; len];
    for i in wavg_start..len {
        let num = wavg4(
            close[i] - open[i],
            close[i - 1] - open[i - 1],
            close[i - 2] - open[i - 2],
            close[i - 3] - open[i - 3],
        );
        let denom = wavg4(
            high[i] - low[i],
            high[i - 1] - low[i - 1],
            high[i - 2] - low[i - 2],
            high[i - 3] - low[i - 3],
        );
        ratio[i] = if denom.abs() > 1e-15 {
            num / denom
        } else {
            0.0
        };
    }

    // Step 2: Compute rolling SMA of ratio over rvi_period
    let rvi_start = wavg_start + rvi_period - 1;
    let inv_rvi = 1.0 / rvi_period as f64;
    let mut rvi_vals = vec![f64::NAN; len];

    let mut sum = 0.0;
    for val in ratio.iter().skip(wavg_start).take(rvi_period) {
        sum += val;
    }
    rvi_vals[rvi_start] = sum * inv_rvi;
    for i in (rvi_start + 1)..len {
        sum += ratio[i] - ratio[i - rvi_period];
        rvi_vals[i] = sum * inv_rvi;
    }

    // Step 3: Apply TSF (linear regression forecast) over tsf_period
    // TSF[i] = intercept + slope * tsf_period (one-step forecast)
    // Incremental linreg: O(1) per bar instead of O(period)
    let tsf_start = rvi_start + tsf_period - 1;
    let n = tsf_period as f64;
    // Precompute constants using closed-form formulas
    let np1 = (tsf_period - 1) as f64;
    let sx = np1 * (np1 + 1.0) / 2.0;
    let sx2 = np1 * (np1 + 1.0) * (2.0 * np1 + 1.0) / 6.0;
    let denom = n * sx2 - sx * sx;

    if denom.abs() < 1e-15 {
        return Ok(Array1::from_vec(output));
    }

    // Initialize accumulators from the first window
    let win_start = tsf_start + 1 - tsf_period;
    let mut sy: f64 = rvi_vals[win_start..=tsf_start].iter().sum();
    let mut sxy: f64 = rvi_vals[win_start..=tsf_start]
        .iter()
        .enumerate()
        .map(|(j, &y)| j as f64 * y)
        .sum();
    let slope = (n * sxy - sx * sy) / denom;
    let intercept = (sy - slope * sx) / n;
    output[tsf_start] = intercept + slope * n;

    // Slide window incrementally
    for i in (tsf_start + 1)..len {
        let old_y = rvi_vals[i - tsf_period];
        let new_y = rvi_vals[i];
        // sum_xy_new = sum_xy_old - sum_y_old + old_y + (period-1) * new_y
        sxy = sxy - sy + old_y + np1 * new_y;
        sy = sy - old_y + new_y;
        let slope = (n * sxy - sx * sy) / denom;
        let intercept = (sy - slope * sx) / n;
        output[i] = intercept + slope * n;
    }

    Ok(Array1::from_vec(output))
}

// ============================================================================
// Squeeze Momentum (John Carter version)
// ============================================================================

/// Squeeze Momentum result (John Carter version).
#[derive(Debug, Clone)]
pub struct SqueezeMomentumResult {
    /// Momentum histogram: linear regression slope of (close - midline) over bb_period.
    pub momentum: Array1<f64>,
    /// Squeeze state: true (1.0) when BB is inside KC, false (0.0) otherwise.
    pub squeeze_on: Array1<f64>,
    /// Squeeze firing: true (1.0) when squeeze just released (was on, now off).
    pub squeeze_off: Array1<f64>,
}

/// Squeeze Momentum Indicator (John Carter version)
///
/// Different from TTM Squeeze: momentum is the linear regression value of the
/// distance between close and the average of BB/KC midlines, computed over
/// the Bollinger Bands period.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `bb_period` - Bollinger Bands period (typically 20)
/// * `bb_mult` - Bollinger Bands std dev multiplier (typically 2.0)
/// * `kc_period` - Keltner Channel period (typically 20)
/// * `kc_mult` - Keltner Channel ATR multiplier (typically 1.5)
pub fn squeeze_momentum(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    bb_period: usize,
    bb_mult: f64,
    kc_period: usize,
    kc_mult: f64,
) -> Result<SqueezeMomentumResult> {
    validate_hlc(high, low, close)?;
    let min_len = bb_period.max(kc_period);
    if bb_period < 2 {
        return Err(TaError::InvalidParameter {
            name: "bb_period".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    validate_input(high.len(), min_len)?;

    let len = high.len();

    let bb = crate::indicators::bbands(close, bb_period, bb_mult, bb_mult)?;
    let atr = crate::indicators::atr(high, low, close, kc_period)?;
    let kc_mid = sma(close, kc_period)?;

    let mut squeeze_on_raw = vec![0.0_f64; len];
    let mut delta = vec![f64::NAN; len];

    for i in 0..len {
        if !bb.upper[i].is_nan() && !atr[i].is_nan() && !kc_mid[i].is_nan() {
            let kc_upper = kc_mid[i] + kc_mult * atr[i];
            let kc_lower = kc_mid[i] - kc_mult * atr[i];
            squeeze_on_raw[i] = if bb.lower[i] > kc_lower && bb.upper[i] < kc_upper {
                1.0
            } else {
                0.0
            };

            // Delta = close - average of (BB midline, KC midline)
            let bb_mid = (bb.upper[i] + bb.lower[i]) / 2.0;
            delta[i] = close[i] - (bb_mid + kc_mid[i]) / 2.0;
        }
    }

    // Compute linear regression (TSF) on delta over bb_period
    // Incremental linreg with NaN tracking: O(1) per bar instead of O(period)
    let mut momentum = vec![f64::NAN; len];
    let n = bb_period as f64;
    let np1 = (bb_period - 1) as f64;
    let sx = np1 * (np1 + 1.0) / 2.0;
    let sx2 = np1 * (np1 + 1.0) * (2.0 * np1 + 1.0) / 6.0;
    let denom = n * sx2 - sx * sx;

    if denom.abs() > 1e-15 {
        let first_valid = min_len - 1;
        let linreg_start = first_valid + bb_period - 1;

        let mut sy = 0.0_f64;
        let mut sxy = 0.0_f64;
        let mut nan_count: usize = 0;
        let mut accum_valid = false;

        for i in linreg_start..len {
            let win_start = i + 1 - bb_period;
            let old_y = delta[i - bb_period];
            let new_y = delta[i];

            if i == linreg_start {
                // Initialize accumulators from first window
                for j in 0..bb_period {
                    let y = delta[win_start + j];
                    if y.is_nan() {
                        nan_count += 1;
                    } else {
                        sy += y;
                        sxy += j as f64 * y;
                    }
                }
            } else {
                // Slide window: update nan_count
                if old_y.is_nan() {
                    nan_count -= 1;
                }
                if new_y.is_nan() {
                    nan_count += 1;
                }

                if nan_count == 0 && accum_valid {
                    // Clean incremental update using the x-shift formula:
                    // sxy_new = sxy_old - sy_old + old_y + (period-1) * new_y
                    sxy = sxy - sy + old_y + np1 * new_y;
                    sy = sy - old_y + new_y;
                } else if nan_count == 0 {
                    // Transition from NaN→clean: recompute from scratch
                    sy = 0.0;
                    sxy = 0.0;
                    for j in 0..bb_period {
                        let y = delta[win_start + j];
                        sy += y;
                        sxy += j as f64 * y;
                    }
                    accum_valid = true;
                } else {
                    accum_valid = false;
                    continue;
                }
            }

            if nan_count == 0 {
                let slope = (n * sxy - sx * sy) / denom;
                let intercept = (sy - slope * sx) / n;
                momentum[i] = intercept + slope * n;
            }
        }
    }

    // Compute squeeze_off: was squeeze_on, now squeeze is off
    let mut squeeze_off = vec![0.0_f64; len];
    for i in 1..len {
        if squeeze_on_raw[i - 1] == 1.0 && squeeze_on_raw[i] == 0.0 {
            squeeze_off[i] = 1.0;
        }
    }

    Ok(SqueezeMomentumResult {
        momentum: Array1::from_vec(momentum),
        squeeze_on: Array1::from_vec(squeeze_on_raw),
        squeeze_off: Array1::from_vec(squeeze_off),
    })
}

#[cfg(test)]
mod inertia_tests {
    use super::*;

    #[test]
    fn test_inertia_basic() {
        let n = 50;
        let open: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64) * 0.5).collect();
        let close: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64) * 0.5 + 0.3).collect();
        let high: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64) * 0.5 + 1.0).collect();
        let low: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64) * 0.5 - 0.5).collect();

        let result = inertia(&open, &high, &low, &close, 10, 14).unwrap();
        assert_eq!(result.len(), n);

        let first_valid = 3 + 10 - 1 + 14 - 1;
        for i in 0..first_valid {
            assert!(result[i].is_nan(), "expected NaN at index {i}");
        }
        for i in first_valid..n {
            assert!(result[i].is_finite(), "expected finite at index {i}");
        }
    }

    #[test]
    fn test_inertia_invalid_params() {
        let o = vec![1.0; 30];
        let h = vec![2.0; 30];
        let l = vec![0.5; 30];
        let c = vec![1.5; 30];
        assert!(inertia(&o, &h, &l, &c, 0, 14).is_err());
        assert!(inertia(&o, &h, &l, &c, 10, 1).is_err());
    }

    #[test]
    fn test_inertia_insufficient_data() {
        let o = vec![1.0; 5];
        let h = vec![2.0; 5];
        let l = vec![0.5; 5];
        let c = vec![1.5; 5];
        assert!(inertia(&o, &h, &l, &c, 10, 14).is_err());
    }
}

#[cfg(test)]
mod squeeze_momentum_tests {
    use super::*;

    #[test]
    fn test_squeeze_momentum_basic() {
        let n = 100;
        let close: Vec<f64> = (0..n)
            .map(|i| 100.0 + (i as f64 * 0.1).sin() * 5.0)
            .collect();
        let high: Vec<f64> = close.iter().map(|c| c + 1.5).collect();
        let low: Vec<f64> = close.iter().map(|c| c - 1.5).collect();

        let result = squeeze_momentum(&high, &low, &close, 20, 2.0, 20, 1.5).unwrap();
        assert_eq!(result.momentum.len(), n);
        assert_eq!(result.squeeze_on.len(), n);
        assert_eq!(result.squeeze_off.len(), n);

        let mut has_finite = false;
        for i in 0..n {
            if result.momentum[i].is_finite() {
                has_finite = true;
            }
            assert!(
                result.squeeze_on[i] == 0.0 || result.squeeze_on[i] == 1.0,
                "squeeze_on at {i} must be 0 or 1"
            );
        }
        assert!(has_finite);
    }

    #[test]
    fn test_squeeze_momentum_invalid() {
        let h = vec![2.0; 10];
        let l = vec![1.0; 10];
        let c = vec![1.5; 10];
        assert!(squeeze_momentum(&h, &l, &c, 20, 2.0, 20, 1.5).is_err());
    }

    #[test]
    fn test_squeeze_momentum_small_period() {
        let h = vec![2.0; 5];
        let l = vec![1.0; 5];
        let c = vec![1.5; 5];
        assert!(squeeze_momentum(&h, &l, &c, 1, 2.0, 1, 1.5).is_err());
    }
}

// ============================================================================
// QStick Indicator
// ============================================================================

/// QStick Indicator (趋势确认指标)
///
/// QStick = MA(Close - Open). Measures the average direction of candlestick
/// bodies to confirm trend direction.
///
/// # Arguments
/// * `open` - Open prices
/// * `close` - Close prices
/// * `period` - MA smoothing period
/// * `ma_type` - Moving average type (SMA, EMA, etc.)
///
/// # Returns
/// QStick values array. Initial warm-up values are NaN.
pub fn qstick(
    open: &[f64],
    close: &[f64],
    period: usize,
    ma_type: crate::indicators::MaType,
) -> Result<Array1<f64>> {
    if open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), period)?;

    let diff: Vec<f64> = close.iter().zip(open.iter()).map(|(c, o)| c - o).collect();
    crate::indicators::ma(&diff, period, ma_type)
}

#[cfg(test)]
mod qstick_tests {
    use super::*;
    use crate::indicators::MaType;

    #[test]
    fn test_qstick_basic_sma() {
        let open = vec![
            100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 107.0, 108.0, 109.0,
        ];
        let close = vec![
            101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 107.0, 108.0, 109.0, 110.0,
        ];
        let result = qstick(&open, &close, 5, MaType::Sma).unwrap();
        assert_eq!(result.len(), 10);
        for i in 4..10 {
            assert!(
                (result[i] - 1.0).abs() < 1e-10,
                "Expected 1.0 at {i}, got {}",
                result[i]
            );
        }
    }

    #[test]
    fn test_qstick_ema() {
        let open = vec![100.0; 20];
        let close: Vec<f64> = (0..20).map(|i| 100.0 + i as f64 * 0.5).collect();
        let result = qstick(&open, &close, 10, MaType::Ema).unwrap();
        assert_eq!(result.len(), 20);
        assert!(result[9].is_finite());
    }

    #[test]
    fn test_qstick_invalid() {
        let open = vec![1.0; 3];
        let close = vec![1.5; 5];
        assert!(qstick(&open, &close, 5, MaType::Sma).is_err());
    }

    #[test]
    fn test_qstick_insufficient_data() {
        let open = vec![1.0; 3];
        let close = vec![1.5; 3];
        assert!(qstick(&open, &close, 5, MaType::Sma).is_err());
    }
}

// ============================================================================
// Chande Forecast Oscillator (CFO)
// ============================================================================

/// Chande Forecast Oscillator (CFO)
///
/// CFO = ((Close - TSF(Close, period)) / Close) * 100
///
/// Measures the percentage deviation of price from its time series forecast.
/// Positive values indicate price is above forecast (bullish), negative below (bearish).
///
/// # Arguments
/// * `input` - Price series (typically close prices)
/// * `period` - TSF lookback period
///
/// # Returns
/// CFO values as percentages. First `period - 1` values are NaN.
pub fn chande_forecast_oscillator(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period < 2 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let len = input.len();
    let mut output = vec![f64::NAN; len];

    let n = period as f64;
    let np1 = (period - 1) as f64;
    let sx = np1 * (np1 + 1.0) / 2.0;
    let sx2 = np1 * (np1 + 1.0) * (2.0 * np1 + 1.0) / 6.0;
    let denom = n * sx2 - sx * sx;

    if denom.abs() < 1e-15 {
        return Ok(Array1::from_vec(output));
    }

    // Initialize accumulators from the first window
    let first_idx = period - 1;
    let mut sy: f64 = input[0..period].iter().sum();
    let mut sxy: f64 = input[0..period]
        .iter()
        .enumerate()
        .map(|(j, &y)| j as f64 * y)
        .sum();
    let slope = (n * sxy - sx * sy) / denom;
    let intercept = (sy - slope * sx) / n;
    let tsf_val = intercept + slope * n;
    output[first_idx] = if input[first_idx].abs() > 1e-15 {
        ((input[first_idx] - tsf_val) / input[first_idx]) * 100.0
    } else {
        0.0
    };

    // Slide window incrementally
    for i in period..len {
        let old_y = input[i - period];
        let new_y = input[i];
        sxy = sxy - sy + old_y + np1 * new_y;
        sy = sy - old_y + new_y;
        let slope = (n * sxy - sx * sy) / denom;
        let intercept = (sy - slope * sx) / n;
        let tsf_val = intercept + slope * n;
        output[i] = if input[i].abs() > 1e-15 {
            ((input[i] - tsf_val) / input[i]) * 100.0
        } else {
            0.0
        };
    }

    Ok(Array1::from_vec(output))
}

#[cfg(test)]
mod cfo_tests {
    use super::*;

    #[test]
    fn test_cfo_basic() {
        let data: Vec<f64> = (0..30)
            .map(|i| 100.0 + (i as f64 * 0.3).sin() * 5.0)
            .collect();
        let result = chande_forecast_oscillator(&data, 14).unwrap();
        assert_eq!(result.len(), 30);
        for i in 0..13 {
            assert!(result[i].is_nan());
        }
        for i in 13..30 {
            assert!(result[i].is_finite(), "NaN at index {i}");
        }
    }

    #[test]
    fn test_cfo_trending() {
        let data: Vec<f64> = (0..20).map(|i| 100.0 + i as f64).collect();
        let result = chande_forecast_oscillator(&data, 5).unwrap();
        for i in 4..20 {
            assert!(result[i].is_finite());
        }
    }

    #[test]
    fn test_cfo_invalid() {
        assert!(chande_forecast_oscillator(&[1.0, 2.0], 1).is_err());
        assert!(chande_forecast_oscillator(&[1.0], 5).is_err());
    }
}
