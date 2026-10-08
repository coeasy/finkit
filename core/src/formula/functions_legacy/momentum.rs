//! Momentum-formula shims (delegate to `indicators::momentum*`).

use super::prelude::*;

pub(crate) fn fn_macdext(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    // MACDEXT(close, fast_period, fast_ma, slow_period, slow_ma, signal_period, signal_ma)
    // fast_ma / slow_ma / signal_ma are integer codes: 0 = Sma, 1 = Ema.
    ensure_args_len("MACDEXT", args, 7)?;
    let input = &args[0];
    let fast_period = extract_n(args, 1, "MACDEXT")?;
    let fast_ma = extract_ma_code(args, 2, "MACDEXT")?;
    let slow_period = extract_n(args, 3, "MACDEXT")?;
    let slow_ma = extract_ma_code(args, 4, "MACDEXT")?;
    let signal_period = extract_n(args, 5, "MACDEXT")?;
    let signal_ma = extract_ma_code(args, 6, "MACDEXT")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    if fast_period >= slow_period {
        return Ok(nan_vec(data_len));
    }

    let fast_kind = match fast_ma {
        0 => crate::indicators::overlap::MaType::Sma,
        _ => crate::indicators::overlap::MaType::Ema,
    };
    let slow_kind = match slow_ma {
        0 => crate::indicators::overlap::MaType::Sma,
        _ => crate::indicators::overlap::MaType::Ema,
    };
    let signal_kind = match signal_ma {
        0 => crate::indicators::overlap::MaType::Sma,
        _ => crate::indicators::overlap::MaType::Ema,
    };

    // See `functions::warmup_offset`: the SMA-seeded MACDEXT accumulates its
    // rolling sums from `input[0]` (and the EMA variants seed from it too), so a
    // leading warm-up run has to be skipped rather than folded into the seed.
    let start = crate::formula::functions::warmup_offset(values);
    match lib_momentum::macdext(
        &values[start..],
        fast_period,
        fast_kind,
        slow_period,
        slow_kind,
        signal_period,
        signal_kind,
    ) {
        Ok(r) => Ok(crate::formula::functions::shift_back(
            r.macd, start, data_len,
        )),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_macdfix(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    // MACDFIX(close, signal_period)
    ensure_args_len("MACDFIX", args, 2)?;
    let input = &args[0];
    let signal_period = extract_n(args, 1, "MACDFIX")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    // MACDFIX is *not* MACD with 12/26 periods. TA-Lib pins the EMA smoothing
    // constants at 0.15 / 0.075 rather than recomputing `2/(period+1)`, and the
    // gap accumulates over the series. Delegating to `macd` therefore made the
    // formula path disagree with `indicators::macdfix` (the TA-Lib-parity
    // implementation) on the very same input. `macdfix_with_signal` carries the
    // fixed constants and still honours a caller-supplied signal period.
    //
    // See `functions::warmup_offset`: the recurrence seeds its EMAs from
    // `input[0]`, and `macd` rejected a non-finite value outright, so a leading
    // warm-up run from an upstream indicator turned the whole series NaN.
    let start = crate::formula::functions::warmup_offset(values);
    match lib_momentum::macdfix_with_signal(&values[start..], signal_period) {
        Ok(result) => Ok(crate::formula::functions::shift_back(
            result.macd,
            start,
            data_len,
        )),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_rocp(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ROCP", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "ROCP")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_momentum::rocp(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_rocr(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ROCR", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "ROCR")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_momentum::rocr(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_rocr100(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ROCR100", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "ROCR100")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_momentum::rocr100(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_rsi(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("RSI", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "RSI")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_momentum::rsi(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_macd(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MACD", args, 2)?;
    let input = &args[0];
    let fast_n = extract_n(args, 1, "MACD")?;
    let slow_n = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0] as usize
    } else {
        26
    };
    let signal_n = if args.len() > 3 && !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0] as usize
    } else {
        9
    };

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    if fast_n >= slow_n {
        return Ok(nan_vec(data_len));
    }

    // See `functions::warmup_offset`: `macd` rejects any non-finite input and
    // seeds its EMAs from `input[0]`, so a leading warm-up run from an upstream
    // indicator has to be skipped rather than rejected.
    let start = crate::formula::functions::warmup_offset(values);
    match lib_momentum::macd(&values[start..], fast_n, slow_n, signal_n) {
        Ok(result) => Ok(crate::formula::functions::shift_back(
            result.macd,
            start,
            data_len,
        )),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_diff(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DIFF", args, 3)?;
    let input = &args[0];
    let fast_n = extract_n(args, 1, "DIFF")?;
    let slow_n = extract_n(args, 2, "DIFF")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    if fast_n >= slow_n {
        return Ok(nan_vec(data_len));
    }

    // See `functions::warmup_offset`: `macd` seeds its EMAs from `input[0]` and
    // rejects any non-finite value, so a leading warm-up run has to be skipped
    // rather than handed to the seed. Without this, `DIFF(MA(CLOSE, 5), 12, 26)`
    // was an all-NaN series.
    let start = crate::formula::functions::warmup_offset(values);
    match lib_momentum::macd(&values[start..], fast_n, slow_n, 9) {
        Ok(result) => Ok(crate::formula::functions::shift_back(
            result.macd,
            start,
            data_len,
        )),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_dea(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DEA", args, 2)?;
    let input = &args[0];
    let fast_n = extract_n(args, 1, "DEA")?;
    let slow_n = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0] as usize
    } else {
        26
    };
    let signal_n = if args.len() > 3 && !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0] as usize
    } else {
        9
    };

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    if fast_n >= slow_n {
        return Ok(nan_vec(data_len));
    }

    // See `functions::warmup_offset`: the same seed-from-`input[0]` contract as
    // `fn_diff`, so the leading warm-up run is skipped for the signal leg too.
    let start = crate::formula::functions::warmup_offset(values);
    match lib_momentum::macd(&values[start..], fast_n, slow_n, signal_n) {
        Ok(result) => Ok(crate::formula::functions::shift_back(
            result.signal,
            start,
            data_len,
        )),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_mfi(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MFI", args, 5)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let volume = &args[3];
    let n = extract_n(args, 4, "MFI")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    // `flow(j)` — typical price times volume, classified by the typical-price
    // change — is a per-bar series, so the two flow sums slide in O(1) per
    // bar. `j == 0` has no previous bar and never contributes (its index is
    // never added, so it must never be subtracted either). A bar with a
    // missing flow is excluded from the sums and tracked in `missing`, which
    // reproduces the per-window rescan's "one NaN poisons the window" rule.
    let mut pos_flow = 0.0f64;
    let mut neg_flow = 0.0f64;
    let mut missing = 0usize;
    for i in 0..data_len {
        if i >= 1 {
            let tp = (high[i] + low[i] + close[i]) / 3.0;
            let prev_tp = (high[i - 1] + low[i - 1] + close[i - 1]) / 3.0;
            let mf = tp * volume[i];
            match () {
                _ if mf.is_nan() => missing += 1,
                _ if tp > prev_tp => pos_flow += mf,
                _ => neg_flow += mf,
            }
        }
        if i >= n {
            let out_bar = i - n;
            if out_bar >= 1 {
                let tp = (high[out_bar] + low[out_bar] + close[out_bar]) / 3.0;
                let prev_tp = (high[out_bar - 1] + low[out_bar - 1] + close[out_bar - 1]) / 3.0;
                let mf = tp * volume[out_bar];
                match () {
                    _ if mf.is_nan() => missing -= 1,
                    _ if tp > prev_tp => pos_flow -= mf,
                    _ => neg_flow -= mf,
                }
            }
        }
        if i + 1 >= n && missing == 0 {
            if neg_flow.abs() < crate::utils::NUMERIC_EPSILON {
                result[i] = 100.0;
            } else {
                result[i] = 100.0 - (100.0 / (1.0 + pos_flow / neg_flow));
            }
        }
    }

    Ok(result)
}

pub(crate) fn fn_cci(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    // Two-operand `CCI(source, period)`: the plan path can only carry series the
    // source text names, so this form needs a kernel of its own. It shares
    // `momentum::cci_source_into` with that kernel rather than keeping a second
    // copy of the rolling mean / mean-deviation loop.
    if args.len() == 2 {
        let n = extract_n(args, 1, "CCI")?;
        let data_len = args[0].len().min(ctx.data_len);
        let mut result = nan_vec(data_len);
        if lib_momentum::cci_source_into(
            &args[0].as_slice().unwrap()[..data_len],
            n,
            result.as_slice_mut().unwrap(),
        )
        .is_err()
        {
            return Ok(nan_vec(data_len));
        }
        return Ok(result);
    }

    let (source, n) = match args.len() {
        len if len >= 4 => {
            let high = &args[0];
            let low = &args[1];
            let close = &args[2];
            let n = extract_n(args, 3, "CCI")?;
            let data_len = high.len().min(low.len()).min(close.len());
            let mut typical = Array1::zeros(data_len);
            for i in 0..data_len {
                typical[i] = (high[i] + low[i] + close[i]) / 3.0;
            }
            (typical, n)
        }
        _ => {
            return Err(FormulaError::InvalidParameter(format!(
                "CCI requires (source, period) or (high, low, close, period), got {} arguments",
                args.len()
            )))
        }
    };

    let data_len = source.len().min(ctx.data_len);
    let mut result = nan_vec(data_len);
    for i in (n - 1)..data_len {
        let window_start = i + 1 - n;
        let sum: f64 = (window_start..=i).map(|j| source[j]).sum();
        let mean = sum / n as f64;
        let mean_dev: f64 = (window_start..=i)
            .map(|j| (source[j] - mean).abs())
            .sum::<f64>()
            / n as f64;
        if mean_dev > crate::utils::NUMERIC_EPSILON {
            result[i] = (source[i] - mean) / (0.015 * mean_dev);
        }
    }

    Ok(result)
}

pub(crate) fn fn_willr(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("WILLR", args, 4)?;
    let n = extract_n(args, 3, "WILLR")?;
    let data_len = args[0].len().min(args[1].len()).min(args[2].len());

    // Delegated to the TA-Lib-golden indicator rather than keeping a second
    // hand-rolled copy. The local loop that used to live here agreed with
    // `momentum::willr_into` on every value except a zero high/low range, where
    // it left NaN while the canonical kernel — and the reference it is checked
    // against in `extrema_round6.rs` — yield 0.0. The canonical kernel is the
    // one with golden-file coverage, so that is the behaviour to keep.
    let high = &args[0].as_slice().unwrap()[..data_len];
    let low = &args[1].as_slice().unwrap()[..data_len];
    let close = &args[2].as_slice().unwrap()[..data_len];
    match lib_momentum::willr(high, low, close, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_mom(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MOM", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "MOM")?;

    let data_len = input.len();
    let mut result = nan_vec(data_len);
    for i in n..data_len {
        result[i] = input[i] - input[i - n];
    }
    Ok(result)
}

pub(crate) fn fn_roc(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ROC", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "ROC")?;

    let data_len = input.len();
    let mut result = nan_vec(data_len);
    for i in n..data_len {
        if input[i - n].abs() > crate::utils::NUMERIC_EPSILON {
            result[i] = (input[i] - input[i - n]) / input[i - n] * 100.0;
        }
    }
    Ok(result)
}

pub(crate) fn fn_cmo(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CMO", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "CMO")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let mut sum_up = 0.0f64;
        let mut sum_down = 0.0f64;
        for j in (window_start + 1)..=i {
            let diff = input[j] - input[j - 1];
            if diff > 0.0 {
                sum_up += diff;
            } else {
                sum_down += diff.abs();
            }
        }
        let total = sum_up + sum_down;
        if total > crate::utils::NUMERIC_EPSILON {
            result[i] = (sum_up - sum_down) / total * 100.0;
        }
    }

    Ok(result)
}

pub(crate) fn fn_ppo(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("PPO", args, 3)?;
    let input = &args[0];
    let fast = extract_n(args, 1, "PPO")?;
    let slow = extract_n(args, 2, "PPO")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    let fast_ema = match lib_ma::ema(values, fast) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let slow_ema = match lib_ma::ema(values, slow) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut result = nan_vec(data_len);
    for i in 0..data_len {
        if !fast_ema[i].is_nan()
            && !slow_ema[i].is_nan()
            && slow_ema[i].abs() > crate::utils::NUMERIC_EPSILON
        {
            result[i] = ((fast_ema[i] - slow_ema[i]) / slow_ema[i]) * 100.0;
        }
    }

    Ok(result)
}

pub(crate) fn fn_trix(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TRIX", args, 2)?;
    let n = extract_n(args, 1, "TRIX")?;
    let data_len = args[0].len();
    let input = &args[0].as_slice().unwrap()[..data_len];

    // Delegated to the TA-Lib-golden indicator rather than keeping a second
    // hand-rolled copy. The previous local implementation substituted `0.0`
    // for the warm-up NaN before feeding EMA2 and EMA3, which did two wrong
    // things at once: it produced values in the warm-up region where TA-Lib
    // yields NaN (22 spurious points at period 12), and the injected zeros
    // contaminated the settled region with a residual that only decayed
    // geometrically (~1e-4 still present at bar 120 for period 15).
    match lib_momentum::trix(input, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_bop(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BOP", args, 4)?;
    let open = &args[0];
    let high = &args[1];
    let low = &args[2];
    let close = &args[3];
    let len = open.len().min(high.len()).min(low.len()).min(close.len());
    let mut result = Array1::zeros(len);
    for i in 0..len {
        let range = high[i] - low[i];
        if range.abs() > crate::utils::NUMERIC_EPSILON {
            result[i] = (close[i] - open[i]) / range;
        } else {
            result[i] = 0.0;
        }
    }
    Ok(result)
}

pub(crate) fn fn_apo(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("APO", args, 3)?;
    let input = &args[0];
    let fast = extract_n(args, 1, "APO")?;
    let slow = extract_n(args, 2, "APO")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    let fast_ema = match lib_ma::ema(values, fast) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let slow_ema = match lib_ma::ema(values, slow) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut result = nan_vec(data_len);
    for i in 0..data_len {
        if !fast_ema[i].is_nan() && !slow_ema[i].is_nan() {
            result[i] = fast_ema[i] - slow_ema[i];
        }
    }

    Ok(result)
}

pub(crate) fn fn_dpo(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DPO", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "DPO")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    let ma_vals = match lib_ma::sma(values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let shift = n / 2 + 1;
    let mut result = nan_vec(data_len);
    for i in shift..data_len {
        if !ma_vals[i - shift].is_nan() {
            result[i] = input[i] - ma_vals[i - shift];
        }
    }

    Ok(result)
}

pub(crate) fn fn_percent_rank(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("PERCENT_RANK", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "PERCENT_RANK")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let val = input[i];
        let count = (window_start..=i).filter(|&j| input[j] < val).count();
        result[i] = count as f64 / n as f64 * 100.0;
    }

    Ok(result)
}

pub(crate) fn fn_stoch(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("STOCH", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let fastk = if args.len() > 3 && !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0] as usize
    } else {
        14
    };
    let slowk = if args.len() > 4 && !args[4].is_empty() && !args[4][0].is_nan() {
        args[4][0] as usize
    } else {
        3
    };
    let slowd = if args.len() > 5 && !args[5].is_empty() && !args[5][0].is_nan() {
        args[5][0] as usize
    } else {
        3
    };

    let data_len = ctx.data_len;
    let high_values = high.as_slice().unwrap();
    let low_values = low.as_slice().unwrap();
    let close_values = close.as_slice().unwrap();

    match lib_momentum::stoch(high_values, low_values, close_values, fastk, slowk, slowd) {
        Ok(result) => Ok(result.k),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_fisher(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FISHER", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let n = extract_n(args, 2, "FISHER")?;

    let data_len = ctx.data_len;
    let high_values = high.as_slice().unwrap();
    let low_values = low.as_slice().unwrap();

    match lib_fisher(high_values, low_values, n) {
        Ok(result) => Ok(result.fisher),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_fisher_signal(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FISHER_SIGNAL", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let n = extract_n(args, 2, "FISHER_SIGNAL")?;

    let data_len = ctx.data_len;
    let high_values = high.as_slice().unwrap();
    let low_values = low.as_slice().unwrap();

    match lib_fisher(high_values, low_values, n) {
        Ok(result) => Ok(result.signal),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_tsi(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TSI", args, 2)?;
    let input = &args[0];
    let long_n = extract_n(args, 1, "TSI")?;
    let short_n = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0] as usize
    } else {
        13
    };

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_tsi(values, long_n, short_n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_chop(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CHOP", args, 4)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let n = extract_n(args, 3, "CHOP")?;

    let data_len = ctx.data_len;
    let high_values = high.as_slice().unwrap();
    let low_values = low.as_slice().unwrap();
    let close_values = close.as_slice().unwrap();

    match lib_chop(high_values, low_values, close_values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_adx(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ADX", args, 4)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let di_n = extract_n(args, 3, "ADX")?;

    let high_values = high.as_slice().unwrap();
    let low_values = low.as_slice().unwrap();
    let close_values = close.as_slice().unwrap();
    let data_len = ctx.data_len;

    // Preserve the established four-argument ADX contract exactly.  Pine's
    // ta.dmi(diLength, adxSmoothing) uses the five-argument form below.
    if args.len() == 4 {
        return match lib_momentum::adx(high_values, low_values, close_values, di_n) {
            Ok(result) => Ok(result),
            Err(_) => Ok(nan_vec(data_len)),
        };
    }

    let adx_n = extract_n(args, 4, "ADX")?;
    let plus_di = match lib_momentum::plus_di(high_values, low_values, close_values, di_n) {
        Ok(result) => result,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let minus_di = match lib_momentum::minus_di(high_values, low_values, close_values, di_n) {
        Ok(result) => result,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    // The DX + Wilder/RMA tail lives in the indicator layer so the compiled-plan
    // kernel and this executor cannot drift apart.
    let mut output = nan_vec(data_len);
    if lib_momentum::adx_from_di_into(
        plus_di.as_slice().unwrap(),
        minus_di.as_slice().unwrap(),
        adx_n,
        output.as_slice_mut().unwrap(),
    )
    .is_err()
    {
        return Ok(nan_vec(data_len));
    }

    Ok(output)
}

pub(crate) fn fn_dmi(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    fn_adx(ctx, args)
}

pub(crate) fn fn_dx(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let (high, low, close, n) = resolve_hlc_args("DX", ctx, args)?;
    let data_len = ctx.data_len;
    match crate::indicators::momentum::dx(high, low, close, n) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_plus_di(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let (high, low, close, n) = resolve_hlc_args("PLUS_DI", ctx, args)?;
    let data_len = ctx.data_len;
    match crate::indicators::momentum::plus_di(high, low, close, n) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_minus_di(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let (high, low, close, n) = resolve_hlc_args("MINUS_DI", ctx, args)?;
    let data_len = ctx.data_len;
    match crate::indicators::momentum::minus_di(high, low, close, n) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_adxr(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let (high, low, close, n) = resolve_hlc_args("ADXR", ctx, args)?;
    let data_len = ctx.data_len;
    match crate::indicators::momentum::adxr(high, low, close, n) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_aroonosc(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let (high, low, n) = resolve_hl_args("AROONOSC", ctx, args)?;
    let data_len = ctx.data_len;
    match crate::indicators::momentum::aroonosc(high, low, n) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_aroon_up(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let (high, low, n) = resolve_hl_args("AROON_UP", ctx, args)?;
    let data_len = ctx.data_len;
    match crate::indicators::momentum::aroon(high, low, n) {
        Ok(r) => Ok(r.aroon_up),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_aroon_dn(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let (high, low, n) = resolve_hl_args("AROON_DN", ctx, args)?;
    let data_len = ctx.data_len;
    match crate::indicators::momentum::aroon(high, low, n) {
        Ok(r) => Ok(r.aroon_down),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

// ======================== CYCLE INDICATORS (TA-Lib C compat) ========================

pub(crate) fn fn_stochf(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("STOCHF", args, 3)?;
    let h = args[0].as_slice().unwrap();
    let l = args[1].as_slice().unwrap();
    let c = args[2].as_slice().unwrap();
    let fastk = if args.len() > 3 && !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0] as usize
    } else {
        5
    };
    let fastd = if args.len() > 4 && !args[4].is_empty() && !args[4][0].is_nan() {
        args[4][0] as usize
    } else {
        3
    };
    let data_len = ctx.data_len;
    match lib_momentum::stochf(h, l, c, fastk, fastd) {
        Ok(result) => Ok(result.k),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_stochrsi(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("STOCHRSI", args, 1)?;
    let data = args[0].as_slice().unwrap();
    let rsi_period = if args.len() > 1 && !args[1].is_empty() && !args[1][0].is_nan() {
        args[1][0] as usize
    } else {
        14
    };
    let stoch_period = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0] as usize
    } else {
        14
    };
    let fastk_period = if args.len() > 3 && !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0] as usize
    } else {
        3
    };
    let fastd_period = if args.len() > 4 && !args[4].is_empty() && !args[4][0].is_nan() {
        args[4][0] as usize
    } else {
        3
    };
    let data_len = ctx.data_len;
    match lib_momentum::stochrsi(data, rsi_period, stoch_period, fastk_period, fastd_period) {
        Ok(result) => Ok(result.k),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_ultosc(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let (high, low, close) = resolve_hlc_for_ultosc("ULTOSC", ctx, args)?;
    let p1 = if args.len() > 3 && !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0] as usize
    } else {
        7
    };
    let p2 = if args.len() > 4 && !args[4].is_empty() && !args[4][0].is_nan() {
        args[4][0] as usize
    } else {
        14
    };
    let p3 = if args.len() > 5 && !args[5].is_empty() && !args[5][0].is_nan() {
        args[5][0] as usize
    } else {
        28
    };
    let data_len = ctx.data_len;
    match lib_momentum::ultosc(
        high.as_slice().unwrap(),
        low.as_slice().unwrap(),
        close.as_slice().unwrap(),
        p1,
        p2,
        p3,
    ) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_plus_dm(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let (high, low) = resolve_hl_for_dm("PLUS_DM", ctx, args)?;
    let data_len = ctx.data_len;
    match lib_momentum::plus_dm(high.as_slice().unwrap(), low.as_slice().unwrap()) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_minus_dm(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let (high, low) = resolve_hl_for_dm("MINUS_DM", ctx, args)?;
    let data_len = ctx.data_len;
    match lib_momentum::minus_dm(high.as_slice().unwrap(), low.as_slice().unwrap()) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_imi(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("IMI", args, 3)?;
    let period = extract_n(args, 2, "IMI")?;
    match lib_imi(
        args[0].as_slice().unwrap(),
        args[1].as_slice().unwrap(),
        period,
    ) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

// ======================== TDX ALIASES ========================

pub(crate) fn fn_pdi(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    fn_plus_di(ctx, args)
}

pub(crate) fn fn_mdi(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    fn_minus_di(ctx, args)
}

pub(crate) fn fn_mtm(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    fn_mom(ctx, args)
}
