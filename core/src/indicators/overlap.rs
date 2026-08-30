use crate::error::{Result, TaError};
use crate::math::moving_avg;
use crate::math::statistics::{rolling_max, rolling_min};
use crate::utils::{init_output, validate_input};
use ndarray::Array1;

pub use crate::math::moving_avg::{dema, ema, ema_into, kama, mavp, sma, sma_into, tema, trima, wma, wma_into};

/// Moving average type selector for the generic `ma()` function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaType {
    Sma,
    Ema,
    Wma,
    Dema,
    Tema,
    Kama,
    T3,
    Trima,
    Hma,
    Alma,
    Vidya,
    Mama,
    Frama,
}

/// Generic Moving Average (MA)
///
/// Dispatches to the requested moving average implementation.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
/// * `ma_type` - Type of moving average
///
/// # Examples
///
/// ```
/// use finkit::indicators::{self, MaType};
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::ma(&close, 5, MaType::Sma).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn ma(input: &[f64], period: usize, ma_type: MaType) -> Result<Array1<f64>> {
    match ma_type {
        MaType::Sma => moving_avg::sma(input, period),
        MaType::Ema => moving_avg::ema(input, period),
        MaType::Wma => moving_avg::wma(input, period),
        MaType::Dema => moving_avg::dema(input, period),
        MaType::Tema => moving_avg::tema(input, period),
        MaType::Kama => moving_avg::kama(input, period, 2, 30),
        MaType::T3 => t3(input, period, 0.7),
        MaType::Trima => moving_avg::trima(input, period),
        MaType::Hma => hma(input, period),
        MaType::Alma => alma(input, period, 0.85, 6.0),
        MaType::Vidya => vidya(input, 9, period.max(9)),
        MaType::Mama => mama(input, 0.5, 0.05).map(|r| r.mama),
        MaType::Frama => frama(input, period.max(4).next_multiple_of(2)),
    }
}

/// Bollinger Bands Result
#[derive(Debug, Clone)]
pub struct BbandsResult {
    /// Upper band
    pub upper: Array1<f64>,
    /// Middle band (SMA)
    pub middle: Array1<f64>,
    /// Lower band
    pub lower: Array1<f64>,
}

/// Bollinger Bands (BBANDS)
///
/// Upper = SMA + (std_dev * nb_dev_up)
/// Middle = SMA
/// Lower = SMA - (std_dev * nb_dev_dn)
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
/// * `nb_dev_up` - Number of standard deviations for upper band
/// * `nb_dev_dn` - Number of standard deviations for lower band
///
/// # Returns
/// BbandsResult containing upper, middle, and lower bands
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::bbands(&close, 5, 2.0, 2.0).unwrap();
/// assert_eq!(result.middle.len(), 10);
/// ```
pub fn bbands(
    input: &[f64],
    period: usize,
    nb_dev_up: f64,
    nb_dev_dn: f64,
) -> Result<BbandsResult> {
    if period < 2 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    if let Some(idx) = input.iter().position(|v| !v.is_finite()) {
        #[cfg(feature = "metrics")]
        crate::metrics::input_rejected("bbands", "non_finite");
        return Err(TaError::InvalidParameter {
            name: "input".to_string(),
            constraint: format!("non-finite value at index {idx}"),
        });
    }
    validate_input(input.len(), period)?;

    let len = input.len();
    let mut upper = init_output(len);
    let mut middle = init_output(len);
    let mut lower = init_output(len);
    let inv_p = 1.0 / period as f64;
    let period_f = period as f64;

    // Welford online algorithm: O(1) per step for mean + population variance (TA-Lib compatible).
    let mut mean = 0.0;
    let mut m2 = 0.0;
    for (j, &x) in input.iter().enumerate().take(period) {
        let n = (j + 1) as f64;
        let delta = x - mean;
        mean += delta / n;
        m2 += delta * (x - mean);
    }

    let std = (m2 * inv_p).max(0.0).sqrt();
    middle[period - 1] = mean;
    upper[period - 1] = mean + std * nb_dev_up;
    lower[period - 1] = mean - std * nb_dev_dn;

    // Pointer-based loop to eliminate bounds checking
    let input_ptr = input.as_ptr();
    let upper_ptr = upper.as_mut_ptr();
    let middle_ptr = middle.as_mut_ptr();
    let lower_ptr = lower.as_mut_ptr();
    
    for i in period..len {
        let old = unsafe { *input_ptr.add(i - period) };
        let new = unsafe { *input_ptr.add(i) };
        let old_mean = mean;
        mean += (new - old) / period_f;
        m2 += (new - mean) * (new - old_mean) - (old - mean) * (old - old_mean);
        let std = (m2 * inv_p).sqrt();
        unsafe {
            *middle_ptr.add(i) = mean;
            *upper_ptr.add(i) = mean + std * nb_dev_up;
            *lower_ptr.add(i) = mean - std * nb_dev_dn;
        }
    }

    Ok(BbandsResult {
        upper,
        middle,
        lower,
    })
}

/// Midpoint (MIDPOINT)
///
/// MIDPOINT = (highest_high + lowest_low) / 2
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
///
/// # Returns
/// Array of midpoint values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::midpoint(&close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn midpoint(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let max = rolling_max(input, period)?;
    let min = rolling_min(input, period)?;

    let len = input.len();
    let mut output = init_output(len);

    for i in 0..len {
        if !max[i].is_nan() && !min[i].is_nan() {
            output[i] = (max[i] + min[i]) / 2.0;
        }
    }

    Ok(output)
}

/// Midprice (MIDPRICE)
///
/// MIDPRICE = (highest_high + lowest_low) / 2
/// Calculated using high and low prices
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `period` - Lookback period
///
/// # Returns
/// Array of midprice values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let result = indicators::midprice(&high, &low, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn midprice(high: &[f64], low: &[f64], period: usize) -> Result<Array1<f64>> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period)?;

    let max = rolling_max(high, period)?;
    let min = rolling_min(low, period)?;

    let len = high.len();
    let mut output = init_output(len);

    for i in 0..len {
        if !max[i].is_nan() && !min[i].is_nan() {
            output[i] = (max[i] + min[i]) / 2.0;
        }
    }

    Ok(output)
}

/// Parabolic SAR (SAR) Result
#[derive(Debug, Clone)]
pub struct SarResult {
    /// SAR values
    pub sar: Array1<f64>,
    /// Acceleration factor values
    pub af: Array1<f64>,
}

/// Parabolic Stop and Reverse (SAR)
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `acceleration` - Acceleration factor step (default: 0.02)
/// * `maximum` - Maximum acceleration factor (default: 0.2)
///
/// # Returns
/// SarResult containing SAR and AF values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let result = indicators::sar(&high, &low, 0.02, 0.2).unwrap();
/// assert_eq!(result.sar.len(), 10);
/// ```
pub fn sar(high: &[f64], low: &[f64], acceleration: f64, maximum: f64) -> Result<SarResult> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if high.len() < 2 {
        return Err(TaError::InsufficientData {
            length: high.len(),
            required: 2,
        });
    }
    if acceleration <= 0.0 || maximum <= 0.0 || acceleration >= maximum {
        return Err(TaError::InvalidParameter {
            name: "acceleration/maximum".to_string(),
            constraint: "0 < acceleration < maximum".to_string(),
        });
    }

    let len = high.len();
    let mut sar_values = init_output(len);
    let mut af_values = init_output(len);

    // Assume initial trend is up (first bar)
    let mut is_long = true;
    let mut ep = high[0]; // Extreme point
    let mut af = acceleration;
    let mut prev_sar = low[0];

    sar_values[0] = prev_sar;
    af_values[0] = af;

    for i in 1..len {
        let mut current_sar = prev_sar + af * (ep - prev_sar);

        // SAR limits
        if is_long {
            if i >= 2 {
                current_sar = current_sar.min(low[i - 1]);
                if i >= 3 {
                    current_sar = current_sar.min(low[i - 2]);
                }
            }
        } else {
            if i >= 2 {
                current_sar = current_sar.max(high[i - 1]);
                if i >= 3 {
                    current_sar = current_sar.max(high[i - 2]);
                }
            }
        }

        // Check for SAR crossover
        let mut switched = false;
        if is_long {
            if low[i] < current_sar {
                is_long = false;
                current_sar = ep;
                ep = low[i];
                af = acceleration;
                switched = true;
            }
        } else {
            if high[i] > current_sar {
                is_long = true;
                current_sar = ep;
                ep = high[i];
                af = acceleration;
                switched = true;
            }
        }

        // Update EP and AF
        if !switched {
            if is_long && high[i] > ep {
                ep = high[i];
                af = (af + acceleration).min(maximum);
            } else if !is_long && low[i] < ep {
                ep = low[i];
                af = (af + acceleration).min(maximum);
            }
        }

        sar_values[i] = current_sar;
        af_values[i] = af;
        prev_sar = current_sar;
    }

    Ok(SarResult {
        sar: sar_values,
        af: af_values,
    })
}

/// Parabolic SAR Extended (SAREXT)
///
/// Extended version of SAR with full control over acceleration factor parameters.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `start_value` - Initial SAR value (0.0 = auto-detect)
/// * `offset_on_reverse` - Offset applied on trend reversal (default: 0.0)
/// * `af_init_long` - Initial AF for long trends (default: 0.02)
/// * `af_long` - AF step for long trends (default: 0.02)
/// * `af_max_long` - Maximum AF for long trends (default: 0.2)
/// * `af_init_short` - Initial AF for short trends (default: 0.02)
/// * `af_short` - AF step for short trends (default: 0.02)
/// * `af_max_short` - Maximum AF for short trends (default: 0.2)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let result = indicators::sarext(&high, &low, 0.0, 0.0, 0.02, 0.02, 0.2, 0.02, 0.02, 0.2).unwrap();
/// assert_eq!(result.sar.len(), 10);
/// ```
#[allow(clippy::too_many_arguments)]
pub fn sarext(
    high: &[f64],
    low: &[f64],
    start_value: f64,
    offset_on_reverse: f64,
    af_init_long: f64,
    af_long: f64,
    af_max_long: f64,
    af_init_short: f64,
    af_short: f64,
    af_max_short: f64,
) -> Result<SarResult> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if high.len() < 2 {
        return Err(TaError::InsufficientData {
            length: high.len(),
            required: 2,
        });
    }

    let len = high.len();
    let mut sar_values = init_output(len);
    let mut af_values = init_output(len);

    let mut is_long = start_value >= 0.0;
    let initial_af = if is_long { af_init_long } else { af_init_short };
    let mut af = initial_af;
    let mut ep = if is_long { high[0] } else { low[0] };
    let mut prev_sar = if is_long {
        if start_value > 0.0 {
            start_value
        } else {
            low[0]
        }
    } else {
        if start_value < 0.0 {
            -start_value
        } else {
            high[0]
        }
    };

    sar_values[0] = if is_long { prev_sar } else { -prev_sar };
    af_values[0] = af;

    for i in 1..len {
        let mut current_sar = prev_sar + af * (ep - prev_sar);

        if is_long {
            if i >= 2 {
                current_sar = current_sar.min(low[i - 1]);
                if i >= 3 {
                    current_sar = current_sar.min(low[i - 2]);
                }
            }
        } else {
            if i >= 2 {
                current_sar = current_sar.max(high[i - 1]);
                if i >= 3 {
                    current_sar = current_sar.max(high[i - 2]);
                }
            }
        }

        let mut switched = false;
        if is_long {
            if low[i] < current_sar {
                is_long = false;
                current_sar = ep + offset_on_reverse;
                ep = low[i];
                af = af_init_short;
                switched = true;
            }
        } else {
            if high[i] > current_sar {
                is_long = true;
                current_sar = ep - offset_on_reverse;
                ep = high[i];
                af = af_init_long;
                switched = true;
            }
        }

        if !switched {
            if is_long && high[i] > ep {
                ep = high[i];
                af = (af + af_long).min(af_max_long);
            } else if !is_long && low[i] < ep {
                ep = low[i];
                af = (af + af_short).min(af_max_short);
            }
        }

        sar_values[i] = if is_long {
            current_sar
        } else {
            -current_sar
        };
        af_values[i] = af;
        prev_sar = current_sar.abs();
    }

    Ok(SarResult {
        sar: sar_values,
        af: af_values,
    })
}

/// MAMA (MESA Adaptive Moving Average) Result
#[derive(Debug, Clone)]
pub struct MamaResult {
    /// MAMA values
    pub mama: Array1<f64>,
    /// FAMA (Following Adaptive Moving Average) values
    pub fama: Array1<f64>,
}

/// MESA Adaptive Moving Average (MAMA)
///
/// Uses Hilbert Transform to create an adaptive moving average that
/// adapts to price fluctuations without the phase lag of traditional MAs.
///
/// # Algorithm
/// 1. Smooth the input data
/// 2. Apply Hilbert Transform to extract phase information
/// 3. Calculate instantaneous period from phase changes
/// 4. Use period to compute adaptive alpha
/// 5. Apply exponential smoothing with adaptive alpha
///
/// # Arguments
/// * `input` - Input data series (at least 32 points)
/// * `fast_limit` - Fast limit for alpha (default: 0.5)
/// * `slow_limit` - Slow limit for alpha (default: 0.05)
///
/// # Returns
/// MamaResult containing MAMA and FAMA arrays
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close: Vec<f64> = (0..40).map(|i| 44.0 + (i as f64 * 0.2).sin()).collect();
/// let result = indicators::mama(&close, 0.5, 0.05).unwrap();
/// assert_eq!(result.mama.len(), 40);
/// ```
pub fn mama(input: &[f64], fast_limit: f64, slow_limit: f64) -> Result<MamaResult> {
    validate_input(input.len(), 32)?;

    if fast_limit <= slow_limit {
        return Err(TaError::InvalidParameter {
            name: "fast_limit".to_string(),
            constraint: "greater than slow_limit".to_string(),
        });
    }

    if fast_limit > 1.0 || fast_limit <= 0.0 {
        return Err(TaError::InvalidParameter {
            name: "fast_limit".to_string(),
            constraint: "between 0 and 1".to_string(),
        });
    }

    if slow_limit <= 0.0 || slow_limit >= 1.0 {
        return Err(TaError::InvalidParameter {
            name: "slow_limit".to_string(),
            constraint: "between 0 and 1".to_string(),
        });
    }

    let len = input.len();
    let mut mama_values = init_output(len);
    let mut fama_values = init_output(len);

    // Faithful TA-Lib MAMA implementation using IIR recursive Hilbert Transform.
    // Lookback = 32 (12 WMA + 6 detrender + 6 Q1 + 3 jI + 3 jQ + 1 Re/Im + 1 deltaPhase).
    let rad2deg = 180.0 / (4.0 * (1.0f64).atan());
    let a_coeff = 0.0962;
    let b_coeff = 0.5769;

    // IIR filter state
    let mut prev_q2 = 0.0;
    let mut prev_i2 = 0.0;
    let mut re = 0.0;
    let mut im = 0.0;
    let mut period = 0.0;

    // Delay lines for I1 (detrender delayed by 2 bars for each parity)
    let mut i1_for_even_prev3 = 0.0;
    let mut i1_for_odd_prev3 = 0.0;
    let mut i1_for_even_prev2 = 0.0;
    let mut i1_for_odd_prev2 = 0.0;

    // 3-element circular buffers for IIR highpass filters
    let mut detrender_even = [0.0; 3];
    let mut detrender_odd = [0.0; 3];
    let mut q1_even = [0.0; 3];
    let mut q1_odd = [0.0; 3];
    let mut ji_even = [0.0; 3];
    let mut ji_odd = [0.0; 3];
    let mut jq_even = [0.0; 3];
    let mut jq_odd = [0.0; 3];

    // Previous values for IIR highpass feedback
    let mut prev_detrender_even = 0.0;
    let mut prev_detrender_odd = 0.0;
    let mut prev_detrender_input_even = 0.0;
    let mut prev_detrender_input_odd = 0.0;
    let mut prev_q1_even = 0.0;
    let mut prev_q1_odd = 0.0;
    let mut prev_q1_input_even = 0.0;
    let mut prev_q1_input_odd = 0.0;
    let mut prev_ji_even = 0.0;
    let mut prev_ji_odd = 0.0;
    let mut prev_ji_input_even = 0.0;
    let mut prev_ji_input_odd = 0.0;
    let mut prev_jq_even = 0.0;
    let mut prev_jq_odd = 0.0;
    let mut prev_jq_input_even = 0.0;
    let mut prev_jq_input_odd = 0.0;

    let mut hilbert_idx = 0;
    let mut prev_phase = 0.0;
    let mut mama_val = 0.0;
    let mut fama_val = 0.0;

    // WMA smoother state (10-period weighted moving average)
    let mut trailing_wma_idx = 0;
    let mut period_wma_sub;
    let mut period_wma_sum;
    let mut trailing_wma_value = 0.0;

    // Initialize WMA with first 3 bars
    period_wma_sub = input[0];
    period_wma_sum = input[0];
    period_wma_sub += input[1];
    period_wma_sum += input[1] * 2.0;
    period_wma_sub += input[2];
    period_wma_sum += input[2] * 3.0;

    // Process from bar 3 (after WMA init) through bar 9 to warm up
    for i in 3..10 {
        let today_value = input[i];
        period_wma_sub += today_value;
        period_wma_sub -= trailing_wma_value;
        period_wma_sum += today_value * 4.0;
        trailing_wma_value = input[trailing_wma_idx];
        trailing_wma_idx += 1;
        let _smoothed_value = period_wma_sum * 0.1;
        period_wma_sum -= period_wma_sub;
    }

    // Main processing loop from bar 10 onward (lookback = 32, output starts at 32)
    for i in 10..len {
        let adjusted_prev_period = 0.075 * period + 0.54;
        let today_value = input[i];

        // Update WMA smoother
        period_wma_sub += today_value;
        period_wma_sub -= trailing_wma_value;
        period_wma_sum += today_value * 4.0;
        trailing_wma_value = input[trailing_wma_idx];
        trailing_wma_idx += 1;
        let smoothed_value = period_wma_sum * 0.1;
        period_wma_sum -= period_wma_sub;

        let phase_degrees;

        if i % 2 == 0 {
            // ---- Even bar processing ----
            // Detrender IIR highpass
            let mut detrender_val = -detrender_even[hilbert_idx];
            detrender_even[hilbert_idx] = a_coeff * smoothed_value;
            detrender_val += a_coeff * smoothed_value;
            detrender_val -= prev_detrender_even;
            prev_detrender_even = b_coeff * prev_detrender_input_even;
            detrender_val += prev_detrender_even;
            prev_detrender_input_even = smoothed_value;
            detrender_val *= adjusted_prev_period;

            // Q1 IIR highpass
            let mut q1_val = -q1_even[hilbert_idx];
            q1_even[hilbert_idx] = a_coeff * detrender_val;
            q1_val += a_coeff * detrender_val;
            q1_val -= prev_q1_even;
            prev_q1_even = b_coeff * prev_q1_input_even;
            q1_val += prev_q1_even;
            prev_q1_input_even = detrender_val;
            q1_val *= adjusted_prev_period;

            // jI IIR highpass
            let mut ji_val = -ji_even[hilbert_idx];
            ji_even[hilbert_idx] = a_coeff * i1_for_even_prev3;
            ji_val += a_coeff * i1_for_even_prev3;
            ji_val -= prev_ji_even;
            prev_ji_even = b_coeff * prev_ji_input_even;
            ji_val += prev_ji_even;
            prev_ji_input_even = i1_for_even_prev3;
            ji_val *= adjusted_prev_period;

            // jQ IIR highpass
            let mut jq_val = -jq_even[hilbert_idx];
            jq_even[hilbert_idx] = a_coeff * q1_val;
            jq_val += a_coeff * q1_val;
            jq_val -= prev_jq_even;
            prev_jq_even = b_coeff * prev_jq_input_even;
            jq_val += prev_jq_even;
            prev_jq_input_even = q1_val;
            jq_val *= adjusted_prev_period;

            // Advance circular buffer index
            hilbert_idx = if hilbert_idx == 2 { 0 } else { hilbert_idx + 1 };

            // IIR recursive filtering for Q2 and I2
            let q2 = 0.2 * (q1_val + ji_val) + 0.8 * prev_q2;
            let i2 = 0.2 * (i1_for_even_prev3 - jq_val) + 0.8 * prev_i2;

            // Update I1 delay lines for next odd bar
            i1_for_odd_prev3 = i1_for_odd_prev2;
            i1_for_odd_prev2 = detrender_val;

            // Phase: atan(Q1 / I1ForEvenPrev3) in degrees
            phase_degrees = if i1_for_even_prev3.abs() > 1e-15 {
                (q1_val / i1_for_even_prev3).atan() * rad2deg
            } else {
                0.0
            };

            // Re/Im use OLD prevQ2/prevI2 (before update), matching TA-Lib
            re = 0.2 * (i2 * prev_i2 + q2 * prev_q2) + 0.8 * re;
            im = 0.2 * (i2 * prev_q2 - q2 * prev_i2) + 0.8 * im;
            prev_q2 = q2;
            prev_i2 = i2;
        } else {
            // ---- Odd bar processing ----
            // Detrender IIR highpass
            let mut detrender_val = -detrender_odd[hilbert_idx];
            detrender_odd[hilbert_idx] = a_coeff * smoothed_value;
            detrender_val += a_coeff * smoothed_value;
            detrender_val -= prev_detrender_odd;
            prev_detrender_odd = b_coeff * prev_detrender_input_odd;
            detrender_val += prev_detrender_odd;
            prev_detrender_input_odd = smoothed_value;
            detrender_val *= adjusted_prev_period;

            // Q1 IIR highpass
            let mut q1_val = -q1_odd[hilbert_idx];
            q1_odd[hilbert_idx] = a_coeff * detrender_val;
            q1_val += a_coeff * detrender_val;
            q1_val -= prev_q1_odd;
            prev_q1_odd = b_coeff * prev_q1_input_odd;
            q1_val += prev_q1_odd;
            prev_q1_input_odd = detrender_val;
            q1_val *= adjusted_prev_period;

            // jI IIR highpass
            let mut ji_val = -ji_odd[hilbert_idx];
            ji_odd[hilbert_idx] = a_coeff * i1_for_odd_prev3;
            ji_val += a_coeff * i1_for_odd_prev3;
            ji_val -= prev_ji_odd;
            prev_ji_odd = b_coeff * prev_ji_input_odd;
            ji_val += prev_ji_odd;
            prev_ji_input_odd = i1_for_odd_prev3;
            ji_val *= adjusted_prev_period;

            // jQ IIR highpass
            let mut jq_val = -jq_odd[hilbert_idx];
            jq_odd[hilbert_idx] = a_coeff * q1_val;
            jq_val += a_coeff * q1_val;
            jq_val -= prev_jq_odd;
            prev_jq_odd = b_coeff * prev_jq_input_odd;
            jq_val += prev_jq_odd;
            prev_jq_input_odd = q1_val;
            jq_val *= adjusted_prev_period;

            // IIR recursive filtering for Q2 and I2
            let q2 = 0.2 * (q1_val + ji_val) + 0.8 * prev_q2;
            let i2 = 0.2 * (i1_for_odd_prev3 - jq_val) + 0.8 * prev_i2;

            // Update I1 delay lines for next even bar
            i1_for_even_prev3 = i1_for_even_prev2;
            i1_for_even_prev2 = detrender_val;

            // Phase: atan(Q1 / I1ForOddPrev3) in degrees
            phase_degrees = if i1_for_odd_prev3.abs() > 1e-15 {
                (q1_val / i1_for_odd_prev3).atan() * rad2deg
            } else {
                0.0
            };

            // Re/Im use OLD prevQ2/prevI2 (before update), matching TA-Lib
            re = 0.2 * (i2 * prev_i2 + q2 * prev_q2) + 0.8 * re;
            im = 0.2 * (i2 * prev_q2 - q2 * prev_i2) + 0.8 * im;
            prev_q2 = q2;
            prev_i2 = i2;
        }

        // Delta Phase
        let mut delta_phase = prev_phase - phase_degrees;
        prev_phase = phase_degrees;

        // Clamp delta phase to >= 1.0
        if delta_phase < 1.0 {
            delta_phase = 1.0;
        }

        // Compute alpha
        let alpha = if delta_phase > 1.0 {
            let temp = fast_limit / delta_phase;
            if temp < slow_limit {
                slow_limit
            } else {
                temp
            }
        } else {
            fast_limit
        };

        // Update MAMA and FAMA
        mama_val = alpha * today_value + (1.0 - alpha) * mama_val;
        let half_alpha = alpha * 0.5;
        fama_val = half_alpha * mama_val + (1.0 - half_alpha) * fama_val;

        // Store output (valid from bar 32 onward)
        if i >= 32 {
            mama_values[i] = mama_val;
            fama_values[i] = fama_val;
        }

        // Adjust period for next bar (same as HT_DCPERIOD)
        let temp_period = period;
        if im.abs() > 1e-10 && re.abs() > 1e-10 {
            period = 360.0 / (im / re).atan();
        }

        let temp15 = 1.5 * temp_period;
        if period > temp15 {
            period = temp15;
        }
        let temp067 = 0.67 * temp_period;
        if period < temp067 {
            period = temp067;
        }
        if period < 6.0 {
            period = 6.0;
        } else if period > 50.0 {
            period = 50.0;
        }
        period = 0.2 * period + 0.8 * temp_period;
    }

    Ok(MamaResult {
        mama: mama_values,
        fama: fama_values,
    })
}

/// MAMA zero-copy variant: writes (mama, fama) into pre-allocated slices.
///
/// Reuses caller-provided buffers to avoid the two `Array1` allocations that
/// the array-returning version requires. Identical numerical output.
pub fn mama_into(
    input: &[f64],
    fast_limit: f64,
    slow_limit: f64,
    mama_out: &mut [f64],
    fama_out: &mut [f64],
) -> Result<()> {
    validate_input(input.len(), 32)?;

    if fast_limit <= slow_limit {
        return Err(TaError::InvalidParameter {
            name: "fast_limit".to_string(),
            constraint: "greater than slow_limit".to_string(),
        });
    }
    if fast_limit > 1.0 || fast_limit <= 0.0 {
        return Err(TaError::InvalidParameter {
            name: "fast_limit".to_string(),
            constraint: "between 0 and 1".to_string(),
        });
    }
    if slow_limit <= 0.0 || slow_limit >= 1.0 {
        return Err(TaError::InvalidParameter {
            name: "slow_limit".to_string(),
            constraint: "between 0 and 1".to_string(),
        });
    }
    let len = input.len();
    if mama_out.len() != len || fama_out.len() != len {
        return Err(TaError::InvalidParameter {
            name: "mama_out/fama_out".to_string(),
            constraint: "must each have the same length as input".to_string(),
        });
    }

    // Initialise outputs to NaN.
    for v in mama_out.iter_mut() {
        *v = f64::NAN;
    }
    for v in fama_out.iter_mut() {
        *v = f64::NAN;
    }

    // Faithful TA-Lib MAMA implementation using IIR recursive Hilbert Transform.
    let rad2deg = 180.0 / (4.0 * (1.0f64).atan());
    let a_coeff = 0.0962;
    let b_coeff = 0.5769;

    // IIR filter state
    let mut prev_q2 = 0.0;
    let mut prev_i2 = 0.0;
    let mut re = 0.0;
    let mut im = 0.0;
    let mut period = 0.0;

    // Delay lines for I1 (detrender delayed by 2 bars for each parity)
    let mut i1_for_even_prev3 = 0.0;
    let mut i1_for_odd_prev3 = 0.0;
    let mut i1_for_even_prev2 = 0.0;
    let mut i1_for_odd_prev2 = 0.0;

    // 3-element circular buffers for IIR highpass filters
    let mut detrender_even = [0.0; 3];
    let mut detrender_odd = [0.0; 3];
    let mut q1_even = [0.0; 3];
    let mut q1_odd = [0.0; 3];
    let mut ji_even = [0.0; 3];
    let mut ji_odd = [0.0; 3];
    let mut jq_even = [0.0; 3];
    let mut jq_odd = [0.0; 3];

    // Previous values for IIR highpass feedback
    let mut prev_detrender_even = 0.0;
    let mut prev_detrender_odd = 0.0;
    let mut prev_detrender_input_even = 0.0;
    let mut prev_detrender_input_odd = 0.0;
    let mut prev_q1_even = 0.0;
    let mut prev_q1_odd = 0.0;
    let mut prev_q1_input_even = 0.0;
    let mut prev_q1_input_odd = 0.0;
    let mut prev_ji_even = 0.0;
    let mut prev_ji_odd = 0.0;
    let mut prev_ji_input_even = 0.0;
    let mut prev_ji_input_odd = 0.0;
    let mut prev_jq_even = 0.0;
    let mut prev_jq_odd = 0.0;
    let mut prev_jq_input_even = 0.0;
    let mut prev_jq_input_odd = 0.0;

    let mut hilbert_idx = 0;
    let mut prev_phase = 0.0;
    let mut mama_val = 0.0;
    let mut fama_val = 0.0;

    // WMA smoother state (10-period weighted moving average)
    let mut trailing_wma_idx = 0;
    let mut period_wma_sub;
    let mut period_wma_sum;
    let mut trailing_wma_value = 0.0;

    // Initialize WMA with first 3 bars
    period_wma_sub = input[0];
    period_wma_sum = input[0];
    period_wma_sub += input[1];
    period_wma_sum += input[1] * 2.0;
    period_wma_sub += input[2];
    period_wma_sum += input[2] * 3.0;

    // Process from bar 3 (after WMA init) through bar 9 to warm up
    for i in 3..10 {
        let today_value = input[i];
        period_wma_sub += today_value;
        period_wma_sub -= trailing_wma_value;
        period_wma_sum += today_value * 4.0;
        trailing_wma_value = input[trailing_wma_idx];
        trailing_wma_idx += 1;
        let _smoothed_value = period_wma_sum * 0.1;
        period_wma_sum -= period_wma_sub;
    }

    // Main processing loop from bar 10 onward (lookback = 32, output starts at 32)
    for i in 10..len {
        let adjusted_prev_period = 0.075 * period + 0.54;
        let today_value = input[i];

        // Update WMA smoother
        period_wma_sub += today_value;
        period_wma_sub -= trailing_wma_value;
        period_wma_sum += today_value * 4.0;
        trailing_wma_value = input[trailing_wma_idx];
        trailing_wma_idx += 1;
        let smoothed_value = period_wma_sum * 0.1;
        period_wma_sum -= period_wma_sub;

        let phase_degrees;

        if i % 2 == 0 {
            // ---- Even bar processing ----
            // Detrender IIR highpass
            let mut detrender_val = -detrender_even[hilbert_idx];
            detrender_even[hilbert_idx] = a_coeff * smoothed_value;
            detrender_val += a_coeff * smoothed_value;
            detrender_val -= prev_detrender_even;
            prev_detrender_even = b_coeff * prev_detrender_input_even;
            detrender_val += prev_detrender_even;
            prev_detrender_input_even = smoothed_value;
            detrender_val *= adjusted_prev_period;

            // Q1 IIR highpass
            let mut q1_val = -q1_even[hilbert_idx];
            q1_even[hilbert_idx] = a_coeff * detrender_val;
            q1_val += a_coeff * detrender_val;
            q1_val -= prev_q1_even;
            prev_q1_even = b_coeff * prev_q1_input_even;
            q1_val += prev_q1_even;
            prev_q1_input_even = detrender_val;
            q1_val *= adjusted_prev_period;

            // jI IIR highpass
            let mut ji_val = -ji_even[hilbert_idx];
            ji_even[hilbert_idx] = a_coeff * i1_for_even_prev3;
            ji_val += a_coeff * i1_for_even_prev3;
            ji_val -= prev_ji_even;
            prev_ji_even = b_coeff * prev_ji_input_even;
            ji_val += prev_ji_even;
            prev_ji_input_even = i1_for_even_prev3;
            ji_val *= adjusted_prev_period;

            // jQ IIR highpass
            let mut jq_val = -jq_even[hilbert_idx];
            jq_even[hilbert_idx] = a_coeff * q1_val;
            jq_val += a_coeff * q1_val;
            jq_val -= prev_jq_even;
            prev_jq_even = b_coeff * prev_jq_input_even;
            jq_val += prev_jq_even;
            prev_jq_input_even = q1_val;
            jq_val *= adjusted_prev_period;

            // Advance circular buffer index
            hilbert_idx = if hilbert_idx == 2 { 0 } else { hilbert_idx + 1 };

            // IIR recursive filtering for Q2 and I2
            let q2 = 0.2 * (q1_val + ji_val) + 0.8 * prev_q2;
            let i2 = 0.2 * (i1_for_even_prev3 - jq_val) + 0.8 * prev_i2;

            // Update I1 delay lines for next odd bar
            i1_for_odd_prev3 = i1_for_odd_prev2;
            i1_for_odd_prev2 = detrender_val;

            // Phase: atan(Q1 / I1ForEvenPrev3) in degrees
            phase_degrees = if i1_for_even_prev3.abs() > 1e-15 {
                (q1_val / i1_for_even_prev3).atan() * rad2deg
            } else {
                0.0
            };

            // Re/Im use OLD prevQ2/prevI2 (before update), matching TA-Lib
            re = 0.2 * (i2 * prev_i2 + q2 * prev_q2) + 0.8 * re;
            im = 0.2 * (i2 * prev_q2 - q2 * prev_i2) + 0.8 * im;
            prev_q2 = q2;
            prev_i2 = i2;
        } else {
            // ---- Odd bar processing ----
            // Detrender IIR highpass
            let mut detrender_val = -detrender_odd[hilbert_idx];
            detrender_odd[hilbert_idx] = a_coeff * smoothed_value;
            detrender_val += a_coeff * smoothed_value;
            detrender_val -= prev_detrender_odd;
            prev_detrender_odd = b_coeff * prev_detrender_input_odd;
            detrender_val += prev_detrender_odd;
            prev_detrender_input_odd = smoothed_value;
            detrender_val *= adjusted_prev_period;

            // Q1 IIR highpass
            let mut q1_val = -q1_odd[hilbert_idx];
            q1_odd[hilbert_idx] = a_coeff * detrender_val;
            q1_val += a_coeff * detrender_val;
            q1_val -= prev_q1_odd;
            prev_q1_odd = b_coeff * prev_q1_input_odd;
            q1_val += prev_q1_odd;
            prev_q1_input_odd = detrender_val;
            q1_val *= adjusted_prev_period;

            // jI IIR highpass
            let mut ji_val = -ji_odd[hilbert_idx];
            ji_odd[hilbert_idx] = a_coeff * i1_for_odd_prev3;
            ji_val += a_coeff * i1_for_odd_prev3;
            ji_val -= prev_ji_odd;
            prev_ji_odd = b_coeff * prev_ji_input_odd;
            ji_val += prev_ji_odd;
            prev_ji_input_odd = i1_for_odd_prev3;
            ji_val *= adjusted_prev_period;

            // jQ IIR highpass
            let mut jq_val = -jq_odd[hilbert_idx];
            jq_odd[hilbert_idx] = a_coeff * q1_val;
            jq_val += a_coeff * q1_val;
            jq_val -= prev_jq_odd;
            prev_jq_odd = b_coeff * prev_jq_input_odd;
            jq_val += prev_jq_odd;
            prev_jq_input_odd = q1_val;
            jq_val *= adjusted_prev_period;

            // IIR recursive filtering for Q2 and I2
            let q2 = 0.2 * (q1_val + ji_val) + 0.8 * prev_q2;
            let i2 = 0.2 * (i1_for_odd_prev3 - jq_val) + 0.8 * prev_i2;

            // Update I1 delay lines for next even bar
            i1_for_even_prev3 = i1_for_even_prev2;
            i1_for_even_prev2 = detrender_val;

            // Phase: atan(Q1 / I1ForOddPrev3) in degrees
            phase_degrees = if i1_for_odd_prev3.abs() > 1e-15 {
                (q1_val / i1_for_odd_prev3).atan() * rad2deg
            } else {
                0.0
            };

            // Re/Im use OLD prevQ2/prevI2 (before update), matching TA-Lib
            re = 0.2 * (i2 * prev_i2 + q2 * prev_q2) + 0.8 * re;
            im = 0.2 * (i2 * prev_q2 - q2 * prev_i2) + 0.8 * im;
            prev_q2 = q2;
            prev_i2 = i2;
        }

        // Delta Phase
        let mut delta_phase = prev_phase - phase_degrees;
        prev_phase = phase_degrees;

        // Clamp delta phase to >= 1.0
        if delta_phase < 1.0 {
            delta_phase = 1.0;
        }

        // Compute alpha
        let alpha = if delta_phase > 1.0 {
            let temp = fast_limit / delta_phase;
            if temp < slow_limit {
                slow_limit
            } else {
                temp
            }
        } else {
            fast_limit
        };

        // Update MAMA and FAMA
        mama_val = alpha * today_value + (1.0 - alpha) * mama_val;
        let half_alpha = alpha * 0.5;
        fama_val = half_alpha * mama_val + (1.0 - half_alpha) * fama_val;

        // Store output (valid from bar 32 onward)
        if i >= 32 {
            mama_out[i] = mama_val;
            fama_out[i] = fama_val;
        }

        // Adjust period for next bar (same as HT_DCPERIOD)
        let temp_period = period;
        if im.abs() > 1e-10 && re.abs() > 1e-10 {
            period = 360.0 / (im / re).atan();
        }

        let temp15 = 1.5 * temp_period;
        if period > temp15 {
            period = temp15;
        }
        let temp067 = 0.67 * temp_period;
        if period < temp067 {
            period = temp067;
        }
        if period < 6.0 {
            period = 6.0;
        } else if period > 50.0 {
            period = 50.0;
        }
        period = 0.2 * period + 0.8 * temp_period;
    }

    Ok(())
}

/// T3 Moving Average (T3)
///
/// A moving average that uses exponential smoothing with a volume factor
/// to reduce lag and improve signal quality.
///
/// # Algorithm
/// T3 uses 6 EMAs and combines them with coefficients derived from the volume factor:
/// T3 = c1 * EMA6 + c2 * EMA5 + c3 * EMA4 + c4 * EMA3
///
/// where:
/// - c1 = -v³
/// - c2 = 3v² + v³
/// - c3 = -6v² - 3v³
/// - c4 = 1 + 3v + v³ + 3v²
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period (must be >= 1)
/// * `vfactor` - Volume factor (0 to 1, default: 0.7)
///   - Lower values: smoother but more lag
///   - Higher values: more responsive but less smooth
///
/// # Returns
/// Array of T3 values (first values are NaN until enough data for 6 EMAs)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close: Vec<f64> = (1..=20).map(|x| x as f64).collect();
/// let result = indicators::t3(&close, 5, 0.7).unwrap();
/// assert_eq!(result.len(), 20);
/// ```
pub fn t3(input: &[f64], period: usize, vfactor: f64) -> Result<Array1<f64>> {
    if !(0.0..=1.0).contains(&vfactor) {
        return Err(TaError::InvalidParameter {
            name: "vfactor".to_string(),
            constraint: "between 0 and 1".to_string(),
        });
    }

    validate_input(input.len(), period)?;

    // T3 coefficients matching TA-Lib ta_T3.c
    let c1 = -vfactor * vfactor * vfactor;
    let c2 = 3.0 * vfactor * vfactor + vfactor * vfactor * vfactor;
    let c3 = -6.0 * vfactor * vfactor - 3.0 * vfactor * vfactor * vfactor;
    let c4 = 1.0 + 3.0 * vfactor + vfactor * vfactor * vfactor + 3.0 * vfactor * vfactor;

    let len = input.len();
    let mut output = init_output(len);

    // Zero-allocation T3: 6 cascaded EMA layers with SMA seeding (matching
    // TA-Lib's EMA warm-up). Each layer is updated in-place per bar, avoiding
    // the 6 intermediate `Array1` allocations of a naive 6x `ema()` approach.
    let k = crate::utils::smoothing_factor(period);
    let one_minus_k = 1.0 - k;
    let inv_period = 1.0 / period as f64;

    let mut counts = [0usize; 6];
    let mut sums = [0.0f64; 6];
    let mut prevs = [0.0f64; 6];

    for i in 0..len {
        let mut val = input[i];
        for layer in 0..6 {
            counts[layer] += 1;
            if counts[layer] < period {
                sums[layer] += val;
                val = 0.0;
            } else if counts[layer] == period {
                sums[layer] += val;
                prevs[layer] = sums[layer] * inv_period;
                val = prevs[layer];
            } else {
                prevs[layer] = val * k + prevs[layer] * one_minus_k;
                val = prevs[layer];
            }
        }

        if counts[0] >= period
            && counts[1] >= period
            && counts[2] >= period
            && counts[3] >= period
            && counts[4] >= period
            && counts[5] >= period
        {
            output[i] = c1 * prevs[5] + c2 * prevs[4] + c3 * prevs[3] + c4 * prevs[2];
        }
    }

    Ok(output)
}

/// Hull Moving Average (HMA)
///
/// HMA = WMA(2 * WMA(n/2) - WMA(n), sqrt(n))
///
/// Reduces lag compared to traditional moving averages by combining weighted
/// averages at multiple time scales.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period (must be at least 2)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = indicators::hma(&close, 4).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn hma(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period < 2 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "at least 2 for HMA".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let half_period = period / 2;
    let sqrt_period = (period as f64).sqrt().round() as usize;
    let wma_half = moving_avg::wma(input, half_period)?;
    let wma_full = moving_avg::wma(input, period)?;

    let len = input.len();
    let diff_start = period - 1;
    let mut output = init_output(len);
    if diff_start >= len {
        return Ok(output);
    }

    let diff: Vec<f64> = (diff_start..len)
        .map(|i| {
            if wma_half[i].is_nan() || wma_full[i].is_nan() {
                f64::NAN
            } else {
                2.0 * wma_half[i] - wma_full[i]
            }
        })
        .collect();

    if diff.len() < sqrt_period {
        return Ok(output);
    }

    let hma_inner = moving_avg::wma(&diff, sqrt_period)?;
    for (j, &v) in hma_inner.iter().enumerate() {
        if j >= sqrt_period - 1 {
            output[diff_start + j] = v;
        }
    }

    Ok(output)
}

/// Arnaud Legoux Moving Average (ALMA)
///
/// Applies a Gaussian-weighted moving average with configurable offset and width.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
/// * `offset_factor` - Gaussian peak offset (default: 0.85), maps to `offset = offset_factor * (period - 1)`
/// * `sigma` - Gaussian width control (default: 6.0, must be > 0)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = indicators::alma(&close, 3, 0.85, 6.0).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn alma(
    input: &[f64],
    period: usize,
    offset_factor: f64,
    sigma: f64,
) -> Result<Array1<f64>> {
    moving_avg::alma(input, period, sigma, offset_factor)
}

/// Variable Index Dynamic Average (VIDYA)
///
/// Adaptive EMA whose smoothing constant scales with the absolute Chande Momentum
/// Oscillator (CMO) ratio.
///
/// VIDYA[t] = alpha * |CMO_ratio| * price[t] + (1 - alpha * |CMO_ratio|) * VIDYA[t-1]
/// where alpha = 2 / (long_period + 1).
///
/// # Arguments
/// * `input` - Input data series
/// * `short_period` - CMO lookback period
/// * `long_period` - EMA smoothing period
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = indicators::vidya(&close, 3, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn vidya(input: &[f64], short_period: usize, long_period: usize) -> Result<Array1<f64>> {
    if short_period == 0 || long_period == 0 {
        return Err(TaError::InvalidParameter {
            name: "short_period/long_period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    moving_avg::vidya(input, long_period, short_period)
}

/// Fractal Adaptive Moving Average (FRAMA)
///
/// Adjusts EMA smoothing via fractal dimension: responsive in trends, smooth in ranges.
///
/// alpha = exp(-4.6 * (D - 1)) where D is estimated from price range fractal geometry.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback window (must be even and at least 4)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close: Vec<f64> = (1..=20).map(|x| x as f64).collect();
/// let result = indicators::frama(&close, 4).unwrap();
/// assert_eq!(result.len(), 20);
/// ```
pub fn frama(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period < 4 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "at least 4 for FRAMA".to_string(),
        });
    }
    if !period.is_multiple_of(2) {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "even number for FRAMA".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let len = input.len();
    let half = period / 2;
    let log2 = std::f64::consts::LN_2;
    let mut output = init_output(len);

    output[period - 1] = input[period - 1];

    let mut max1 = input[1..1 + half]
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let mut min1 = input[1..1 + half]
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let mut max2 = input[1 + half..=period]
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let mut min2 = input[1 + half..=period]
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let mut max3 = input[1..=period]
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let mut min3 = input[1..=period]
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);

    for i in period..len {
        let n1 = (max1 - min1) / half as f64;
        let n2 = (max2 - min2) / half as f64;
        let n3 = (max3 - min3) / period as f64;

        let alpha = if n1 > 0.0 && n2 > 0.0 && n3 > 0.0 {
            let d = ((n1 + n2).ln() - n3.ln()) / log2;
            (-4.6 * (d - 1.0)).exp().clamp(0.01, 1.0)
        } else {
            0.01
        };

        let prev = output[i - 1];
        if !prev.is_nan() && !input[i].is_nan() {
            output[i] = alpha * input[i] + (1.0 - alpha) * prev;
        }

        if i + 1 >= len {
            break;
        }

        let evicted_first = input[i + 1 - period];
        let added_first = input[i + 1 - half];
        let evicted_second = input[i + 1 - half];
        let added_second = input[i + 1];
        let evicted_window = input[i + 1 - period];
        let added_window = input[i + 1];

        if added_first > max1 {
            max1 = added_first;
        } else if evicted_first == max1 {
            max1 = input[i + 2 - period..i + 2 - half]
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
        }
        if added_first < min1 {
            min1 = added_first;
        } else if evicted_first == min1 {
            min1 = input[i + 2 - period..i + 2 - half]
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min);
        }

        if added_second > max2 {
            max2 = added_second;
        } else if evicted_second == max2 {
            max2 = input[i + 2 - half..=i + 1]
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
        }
        if added_second < min2 {
            min2 = added_second;
        } else if evicted_second == min2 {
            min2 = input[i + 2 - half..=i + 1]
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min);
        }

        if added_window > max3 {
            max3 = added_window;
        } else if evicted_window == max3 {
            max3 = input[i + 2 - period..=i + 1]
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
        }
        if added_window < min3 {
            min3 = added_window;
        } else if evicted_window == min3 {
            min3 = input[i + 2 - period..=i + 1]
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min);
        }
    }

    Ok(output)
}

/// Bollinger Bands zero-copy variant: writes (middle, upper, lower) into pre-allocated slices.
pub fn bbands_into(
    input: &[f64],
    period: usize,
    nb_dev_up: f64,
    nb_dev_dn: f64,
    middle: &mut [f64],
    upper: &mut [f64],
    lower: &mut [f64],
) -> Result<()> {
    if period < 2 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    let len = input.len();
    if middle.len() != len || upper.len() != len || lower.len() != len {
        return Err(TaError::InvalidParameter {
            name: "output slices".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    if let Some(idx) = input.iter().position(|v| !v.is_finite()) {
        #[cfg(feature = "metrics")]
        crate::metrics::input_rejected("bbands_into", "non_finite");
        return Err(TaError::InvalidParameter {
            name: "input".to_string(),
            constraint: format!("non-finite value at index {idx}"),
        });
    }
    validate_input(len, period)?;

    let inv_p = 1.0 / period as f64;
    let period_f = period as f64;

    // Welford online algorithm: O(1) per step for mean + population variance (TA-Lib compatible).
    let mut mean = 0.0;
    let mut m2 = 0.0;
    for (j, &x) in input.iter().enumerate().take(period) {
        let n = (j + 1) as f64;
        let delta = x - mean;
        mean += delta / n;
        m2 += delta * (x - mean);
    }

    let std = (m2 * inv_p).max(0.0).sqrt();
    middle[period - 1] = mean;
    upper[period - 1] = mean + std * nb_dev_up;
    lower[period - 1] = mean - std * nb_dev_dn;

    // Pointer-based loop to eliminate bounds checking
    let input_ptr = input.as_ptr();
    let upper_ptr = upper.as_mut_ptr();
    let middle_ptr = middle.as_mut_ptr();
    let lower_ptr = lower.as_mut_ptr();

    for i in period..len {
        let old = unsafe { *input_ptr.add(i - period) };
        let new = unsafe { *input_ptr.add(i) };
        let old_mean = mean;
        mean += (new - old) / period_f;
        m2 += (new - mean) * (new - old_mean) - (old - mean) * (old - old_mean);
        let std = (m2 * inv_p).sqrt();
        unsafe {
            *middle_ptr.add(i) = mean;
            *upper_ptr.add(i) = mean + std * nb_dev_up;
            *lower_ptr.add(i) = mean - std * nb_dev_dn;
        }
    }

    Ok(())
}

/// DEMA zero-copy variant: writes result into pre-allocated slice.
pub fn dema_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    let result = dema(input, period)?;
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    output.copy_from_slice(result.as_slice().unwrap());
    Ok(())
}

/// TEMA zero-copy variant: writes result into pre-allocated slice.
pub fn tema_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    let result = tema(input, period)?;
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    output.copy_from_slice(result.as_slice().unwrap());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_bbands() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = bbands(&input, 5, 2.0, 2.0).unwrap();

        assert!(result.middle[0].is_nan());
        assert!(result.middle[4] > 0.0);
        assert!(result.upper[4] > result.middle[4]);
        assert!(result.lower[4] < result.middle[4]);
    }

    #[test]
    fn test_midpoint() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = midpoint(&input, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_midprice() {
        let high = vec![10.0, 12.0, 14.0, 16.0, 18.0];
        let low = vec![8.0, 10.0, 12.0, 14.0, 16.0];
        let result = midprice(&high, &low, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        // (max(10,12,14) + min(8,10,12)) / 2 = (14 + 8) / 2 = 11.0
        assert_relative_eq!(result[2], 11.0, epsilon = 1e-10);
    }

    #[test]
    fn test_sar() {
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let result = sar(&high, &low, 0.02, 0.2).unwrap();

        assert!(!result.sar[0].is_nan());
        assert!(!result.sar[4].is_nan());
    }

    #[test]
    fn test_mama_into_matches_mama() {
        let input: Vec<f64> = (1..=50).map(|x| 50.0 + (x as f64 * 0.1).sin() * 5.0).collect();
        let expected = mama(&input, 0.5, 0.05).unwrap();
        let mut mama_out = vec![0.0; input.len()];
        let mut fama_out = vec![0.0; input.len()];
        mama_into(&input, 0.5, 0.05, &mut mama_out, &mut fama_out).unwrap();
        for i in 0..input.len() {
            let em = expected.mama[i];
            let am = mama_out[i];
            if em.is_nan() {
                assert!(am.is_nan());
            } else {
                assert!((em - am).abs() < 1e-9, "mama mismatch at {i}: {em} vs {am}");
            }
            let ef = expected.fama[i];
            let af = fama_out[i];
            if ef.is_nan() {
                assert!(af.is_nan());
            } else {
                assert!((ef - af).abs() < 1e-9, "fama mismatch at {i}: {ef} vs {af}");
            }
        }
    }

    #[test]
    fn test_mama() {
        let input: Vec<f64> = (0..50).map(|i| (i as f64 * 0.2).sin()).collect();
        let result = mama(&input, 0.5, 0.05).unwrap();
        assert_eq!(result.mama.len(), 50);
        assert_eq!(result.fama.len(), 50);
        assert!(result.mama.iter().any(|&x| !x.is_nan()));
        assert!(result.fama.iter().any(|&x| !x.is_nan()));
    }

    #[test]
    fn test_mama_insufficient_data() {
        let input: Vec<f64> = (0..5).map(|i| i as f64).collect();
        assert!(mama(&input, 0.5, 0.05).is_err());
    }

    #[test]
    fn test_mama_invalid_limits() {
        let input: Vec<f64> = (0..50).map(|i| (i as f64 * 0.2).sin()).collect();
        assert!(mama(&input, 0.05, 0.5).is_err());
        assert!(mama(&input, 1.5, 0.05).is_err());
        assert!(mama(&input, 0.5, -0.05).is_err());
    }

    #[test]
    fn test_t3() {
        let input = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
            17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0, 30.0,
        ];
        let result = t3(&input, 5, 0.7).unwrap();
        assert!(result[0].is_nan());
        assert!(result.iter().any(|&x| !x.is_nan()));
    }

    #[test]
    fn test_t3_invalid_vfactor() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!(t3(&input, 3, -0.1).is_err());
        assert!(t3(&input, 3, 1.1).is_err());
    }

    #[test]
    fn test_t3_vfactor_zero() {
        let input = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
            17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0, 30.0,
        ];
        let result = t3(&input, 5, 0.0).unwrap();
        assert!(result.iter().any(|&x| !x.is_nan()));
    }

    #[test]
    fn test_t3_vfactor_one() {
        let input = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
            17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0, 30.0,
        ];
        let result = t3(&input, 5, 1.0).unwrap();
        assert!(result.iter().any(|&x| !x.is_nan()));
    }

    #[test]
    fn test_sma_export() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&input, 3).unwrap();
        assert!(result[2] > 0.0);
    }

    #[test]
    fn test_sma_into_export() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let expected = sma(&input, 3).unwrap();
        let mut output = vec![0.0; input.len()];
        sma_into(&input, 3, &mut output).unwrap();
        for (a, b) in expected.iter().zip(output.iter()) {
            if a.is_nan() {
                assert!(b.is_nan());
            } else {
                assert_relative_eq!(*a, *b, epsilon = 1e-15);
            }
        }
    }

    #[test]
    fn test_ema_into_export() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let expected = ema(&input, 3).unwrap();
        let mut output = vec![0.0; input.len()];
        ema_into(&input, 3, &mut output).unwrap();
        for (a, b) in expected.iter().zip(output.iter()) {
            if a.is_nan() {
                assert!(b.is_nan());
            } else {
                assert_relative_eq!(*a, *b, epsilon = 1e-15);
            }
        }
    }

    #[test]
    fn test_ma_dispatches() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let sma_direct = sma(&input, 3).unwrap();
        let sma_via_ma = ma(&input, 3, MaType::Sma).unwrap();
        for i in 0..input.len() {
            if sma_direct[i].is_nan() {
                assert!(sma_via_ma[i].is_nan());
            } else {
                assert_relative_eq!(sma_direct[i], sma_via_ma[i], epsilon = 1e-10);
            }
        }
    }

    #[test]
    fn test_ma_all_types() {
        let input: Vec<f64> = (1..=50).map(|x| x as f64).collect();
        for ma_type in [
            MaType::Sma,
            MaType::Ema,
            MaType::Wma,
            MaType::Dema,
            MaType::Tema,
            MaType::Kama,
            MaType::T3,
            MaType::Trima,
            MaType::Hma,
            MaType::Alma,
            MaType::Vidya,
            MaType::Mama,
            MaType::Frama,
        ] {
            let result = ma(&input, 5, ma_type);
            assert!(result.is_ok(), "ma({:?}) failed", ma_type);
        }
    }

    #[test]
    fn test_trima() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = trima(&input, 5).unwrap();
        assert!(result[0].is_nan());
        assert!(result[3].is_nan());
        assert!(!result[4].is_nan());
        assert!(result[4] > 0.0);
    }

    #[test]
    fn test_trima_period_1() {
        let input = vec![1.0, 2.0, 3.0];
        let result = trima(&input, 1).unwrap();
        assert_relative_eq!(result[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result[1], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_mavp() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let periods = vec![3.0, 3.0, 3.0, 3.0, 5.0, 5.0, 5.0, 5.0, 5.0, 3.0];
        let result = mavp(&input, &periods, 2, 10).unwrap();
        assert!(result[0].is_nan());
        assert!(!result[2].is_nan());
        assert_relative_eq!(result[2], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_sarext() {
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let result = sarext(&high, &low, 0.0, 0.0, 0.02, 0.02, 0.2, 0.02, 0.02, 0.2).unwrap();
        assert_eq!(result.sar.len(), 7);
        assert!(!result.sar[0].is_nan());
    }

    #[test]
    fn test_sarext_negative_start() {
        let high = vec![10.0, 9.0, 8.0, 7.0, 6.0];
        let low = vec![9.0, 8.0, 7.0, 6.0, 5.0];
        let result = sarext(&high, &low, -1.0, 0.0, 0.02, 0.02, 0.2, 0.02, 0.02, 0.2).unwrap();
        assert!(result.sar[0] < 0.0);
    }

    #[test]
    fn test_hma() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = hma(&input, 4).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(result[3].is_nan());
        assert!(!result[4].is_nan());
        assert!(result[4] > 0.0);
    }

    #[test]
    fn test_hma_invalid_period() {
        assert!(hma(&[1.0, 2.0, 3.0], 1).is_err());
        assert!(hma(&[], 4).is_err());
    }

    #[test]
    fn test_alma() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = alma(&input, 3, 0.85, 6.0).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(!result[2].is_nan());
        assert!(result[2] > 1.0 && result[2] < 5.0);
    }

    #[test]
    fn test_alma_invalid_sigma() {
        assert!(alma(&[1.0, 2.0, 3.0], 3, 0.85, 0.0).is_err());
        assert!(alma(&[1.0, 2.0, 3.0], 3, 0.85, -1.0).is_err());
    }

    #[test]
    fn test_vidya() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = vidya(&input, 3, 5).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(result[3].is_nan());
        assert!(!result[4].is_nan());
        assert!(result[9] > result[4]);
    }

    #[test]
    fn test_vidya_invalid_params() {
        assert!(vidya(&[1.0, 2.0], 0, 5).is_err());
        assert!(vidya(&[1.0, 2.0], 3, 0).is_err());
    }

    #[test]
    fn test_frama() {
        let input: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let result = frama(&input, 4).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(!result[3].is_nan());
        assert!(result[19] > result[3]);
    }

    #[test]
    fn test_frama_invalid_period() {
        assert!(frama(&[1.0, 2.0, 3.0], 3).is_err());
        assert!(frama(&[1.0, 2.0, 3.0], 2).is_err());
    }
}

// ============================================================================
// Jurik Moving Average (JMA)
// ============================================================================

/// Jurik Moving Average (JMA)
///
/// A low-lag, low-noise adaptive moving average using a three-stage
/// filtering architecture with Jurik's adaptive smoothing algorithm.
///
/// # Arguments
/// * `input` - Input price series
/// * `period` - Smoothing period (typically 7-30)
/// * `phase` - Phase shift (-100 to 100, 0 = neutral)
/// * `power` - Damping power (typically 2, range 1-5)
///
/// # Returns
/// JMA values. First value is set to input[0], subsequent values converge.
pub fn jma(input: &[f64], period: usize, phase: f64, power: f64) -> Result<Array1<f64>> {
    if period < 1 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "at least 1".to_string(),
        });
    }
    validate_input(input.len(), 1)?;

    let len = input.len();
    let mut output = vec![f64::NAN; len];

    // Phase calculation
    let phase_ratio = if phase < -100.0 {
        0.5
    } else if phase > 100.0 {
        2.5
    } else {
        phase / 100.0 + 1.5
    };

    // Beta calculation from period
    let beta = 0.45 * (period as f64 - 1.0) / (0.45 * (period as f64 - 1.0) + 2.0);
    let alpha = beta.powf(power);

    // Initialize filter states
    let mut e0 = 0.0_f64;
    let mut e1 = 0.0_f64;
    let mut e2 = 0.0_f64;
    let mut jma_val = input[0];
    output[0] = jma_val;

    for i in 1..len {
        // Three-stage adaptive EMA filter
        e0 = (1.0 - alpha) * input[i] + alpha * e0;
        e1 = (input[i] - e0) * (1.0 - beta) + beta * e1;
        e2 = (e0 + phase_ratio * e1 - jma_val) * (1.0 - alpha).powi(2) + alpha.powi(2) * e2;
        jma_val += e2;
        output[i] = jma_val;
    }

    Ok(Array1::from_vec(output))
}

#[cfg(test)]
mod jma_tests {
    use super::*;

    #[test]
    fn test_jma_basic() {
        let data: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64 * 0.2).sin() * 5.0).collect();
        let result = jma(&data, 7, 0.0, 2.0).unwrap();
        assert_eq!(result.len(), 50);
        assert!((result[0] - data[0]).abs() < 1e-10);
        for i in 1..50 {
            assert!(result[i].is_finite(), "NaN at index {i}");
        }
    }

    #[test]
    fn test_jma_smoothing() {
        // JMA should be smoother than raw input
        let data: Vec<f64> = (0..100)
            .map(|i| 100.0 + (i as f64 * 0.5).sin() * 10.0 + (i as f64 * 3.7).sin() * 2.0)
            .collect();
        let result = jma(&data, 14, 0.0, 2.0).unwrap();

        let input_roughness: f64 = data.windows(2).map(|w| (w[1] - w[0]).abs()).sum();
        let jma_roughness: f64 = result
            .as_slice()
            .unwrap()
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .sum();
        assert!(jma_roughness < input_roughness, "JMA should be smoother");
    }

    #[test]
    fn test_jma_phase_effect() {
        let data: Vec<f64> = (0..50).map(|i| 100.0 + i as f64).collect();
        let lead = jma(&data, 10, 100.0, 2.0).unwrap();
        let lag = jma(&data, 10, -100.0, 2.0).unwrap();
        // With positive phase, JMA should track faster (closer to input at end)
        let lead_err = (lead[49] - data[49]).abs();
        let lag_err = (lag[49] - data[49]).abs();
        assert!(lead_err < lag_err, "Positive phase should reduce lag");
    }

    #[test]
    fn test_jma_invalid() {
        assert!(jma(&[1.0, 2.0], 0, 0.0, 2.0).is_err());
    }

    #[test]
    fn test_jma_single_value() {
        let result = jma(&[42.0], 7, 0.0, 2.0).unwrap();
        assert_eq!(result.len(), 1);
        assert!((result[0] - 42.0).abs() < 1e-10);
    }
}

// ============================================================================
// Kaufman Efficiency Ratio
// ============================================================================

/// Kaufman Efficiency Ratio (效率比率)
///
/// ER = |Price Change over period| / Sum(|Daily Changes|) over period.
/// Values range from 0 (choppy/noisy) to 1 (perfectly trending).
///
/// # Arguments
/// * `input` - Price series
/// * `period` - Lookback period
///
/// # Returns
/// Efficiency Ratio values. First `period` values are NaN.
pub fn efficiency_ratio(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period < 1 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "at least 1".to_string(),
        });
    }
    validate_input(input.len(), period + 1)?;

    let len = input.len();
    let mut output = vec![f64::NAN; len];

    if len <= period {
        return Ok(Array1::from_vec(output));
    }

    let mut vol_sum = 0.0;
    for j in 1..=period {
        vol_sum += (input[j] - input[j - 1]).abs();
    }

    for i in period..len {
        let direction = (input[i] - input[i - period]).abs();
        output[i] = if vol_sum > 1e-15 {
            direction / vol_sum
        } else {
            0.0
        };
        if i + 1 < len {
            vol_sum += (input[i + 1] - input[i]).abs()
                - (input[i - period + 1] - input[i - period]).abs();
        }
    }

    Ok(Array1::from_vec(output))
}

#[cfg(test)]
mod efficiency_ratio_tests {
    use super::*;

    #[test]
    fn test_er_trending() {
        // Perfectly trending data: ER should be ~1.0
        let data: Vec<f64> = (0..20).map(|i| 100.0 + i as f64).collect();
        let result = efficiency_ratio(&data, 10).unwrap();
        assert_eq!(result.len(), 20);
        for i in 10..20 {
            assert!(
                (result[i] - 1.0).abs() < 1e-10,
                "Expected ~1.0 at {i}, got {}",
                result[i]
            );
        }
    }

    #[test]
    fn test_er_choppy() {
        // Choppy data: ER should be close to 0
        let data = vec![100.0, 101.0, 100.0, 101.0, 100.0, 101.0, 100.0, 101.0, 100.0, 101.0, 100.0];
        let result = efficiency_ratio(&data, 10).unwrap();
        assert!(result[10] < 0.2, "Expected choppy ER near 0, got {}", result[10]);
    }

    #[test]
    fn test_er_warmup() {
        let data: Vec<f64> = (0..15).map(|i| 100.0 + i as f64).collect();
        let result = efficiency_ratio(&data, 10).unwrap();
        for i in 0..10 {
            assert!(result[i].is_nan());
        }
        for i in 10..15 {
            assert!(result[i].is_finite());
        }
    }

    #[test]
    fn test_er_invalid() {
        assert!(efficiency_ratio(&[1.0, 2.0], 0).is_err());
        assert!(efficiency_ratio(&[1.0, 2.0], 5).is_err());
    }
}
