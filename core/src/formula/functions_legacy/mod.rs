//! Legacy compatibility catalogue of built-in formula functions.
//!
//! This used to be a single 7.2k-line file. It is now split **by the
//! indicator family each entry delegates to**; the module path
//! `crate::formula::functions_legacy` and the public surface are
//! unchanged, so this is a pure move. Shared argument-extraction
//! helpers and the [`get_builtin_functions`] routing table stay here so
//! that every bucket can see them through `prelude`.

use ndarray::{s, Array1};
use std::collections::{HashMap, VecDeque};

use crate::formula::simd::SimdOps;
use crate::formula::types::*;
use crate::indicators::astock as lib_astock;
use crate::indicators::chart::zigzag as lib_zigzag;
use crate::indicators::china::{bias as lib_bias, kdj as lib_kdj, psy as lib_psy};
use crate::indicators::classic_patterns as lib_classic;
use crate::indicators::cycle as lib_cycle;
use crate::indicators::math_operators as lib_math_operators;
use crate::indicators::momentum as lib_momentum;
use crate::indicators::momentum_ext::imi as lib_imi;
use crate::indicators::momentum_ext::{chop as lib_chop, fisher as lib_fisher, tsi as lib_tsi};
use crate::indicators::overlap::sarext as lib_sarext;
use crate::indicators::statistics::{avgdev as lib_avgdev, zscore as lib_zscore};
use crate::indicators::volume_ext::cmf as lib_cmf;
use crate::math::kernels::{rolling_beta_into, rolling_correlation_into};
use crate::math::linear as lib_linear;
use crate::math::moving_avg as lib_ma;
use crate::math::statistics as lib_stat;
use crate::math::statistics::rolling_minmax_visit;
use crate::patterns::candlestick as lib_candlestick;

// Bucket modules, plus the shared prelude. Declared before the glob
// re-exports below so the routing table can name every `fn_*` unqualified.
mod astock;
mod candlestick;
mod classic;
mod cycle;
mod datetime;
mod em;
mod finance;
mod linear;
mod math_ops;
mod misc;
mod momentum;
mod overlap;
pub(crate) mod prelude;
mod price;
mod reference;
mod statistic;
#[cfg(test)]
mod tests;
mod volatility;
mod volume;
mod zigzag;

// One facade per bucket: every moved item is `pub(crate)`, so a glob
// re-export is all that is needed to keep sibling buckets (and the
// table below) resolving exactly as they did in the single-file era.
pub(crate) use astock::*;
pub(crate) use candlestick::*;
pub(crate) use classic::*;
pub(crate) use cycle::*;
pub(crate) use datetime::*;
pub(crate) use em::*;
pub(crate) use finance::*;
pub(crate) use linear::*;
pub(crate) use math_ops::*;
pub(crate) use misc::*;
pub(crate) use momentum::*;
pub(crate) use overlap::*;
pub(crate) use price::*;
pub(crate) use reference::*;
pub(crate) use statistic::*;
pub(crate) use volatility::*;
pub(crate) use volume::*;
pub(crate) use zigzag::*;

pub(crate) type FormulaFn =
    fn(&FormulaContext, &[Array1<f64>]) -> Result<Array1<f64>, FormulaError>;

pub(crate) fn nan_vec(len: usize) -> Array1<f64> {
    Array1::from_elem(len, f64::NAN)
}

/// Sliding arg-extreme over a trailing window, backed by a monotonic deque.
///
/// Amortized O(1) per bar. `WANT_MAX` selects the direction. Ties keep the
/// earliest bar (the strict pop leaves equal bars queued, so the front stays
/// the first occurrence), matching the per-window rescans this replaces.
/// Missing bars are never queued, so a window with no finite bar yields
/// `None` from [`Self::extreme_index`].
#[derive(Default)]
pub(crate) struct ArgExtremeDeque<const WANT_MAX: bool> {
    deque: VecDeque<(usize, f64)>,
}

impl<const WANT_MAX: bool> ArgExtremeDeque<WANT_MAX> {
    #[inline]
    fn offer(&mut self, index: usize, value: f64) {
        if value.is_nan() {
            return;
        }
        // Strict comparisons keep *equal* bars in the deque instead of popping
        // them: the front must stay the earliest extreme, and an eagerly popped
        // tie would surface a later bar's offset once the earlier one expires.
        while self
            .deque
            .back()
            .is_some_and(|&(_, v)| if WANT_MAX { v < value } else { v > value })
        {
            self.deque.pop_back();
        }
        self.deque.push_back((index, value));
    }

    /// Drop every entry that left the window `[window_start, ..]`.
    #[inline]
    fn expire_before(&mut self, window_start: usize) {
        while self
            .deque
            .front()
            .is_some_and(|&(index, _)| index < window_start)
        {
            self.deque.pop_front();
        }
    }

    #[inline]
    fn extreme_index(&self) -> Option<usize> {
        self.deque.front().map(|&(index, _)| index)
    }
}

pub(crate) fn ensure_args_len(
    name: &str,
    args: &[Array1<f64>],
    expected: usize,
) -> Result<(), FormulaError> {
    if args.len() < expected {
        return Err(FormulaError::InvalidParameter(format!(
            "{} requires at least {} arguments, got {}",
            name,
            expected,
            args.len()
        )));
    }
    Ok(())
}

pub(crate) fn get_string_from_hash(ctx: &FormulaContext, idx_val: f64) -> Option<String> {
    if idx_val.is_nan() || idx_val < 0.0 {
        return None;
    }
    let idx = idx_val as usize;
    ctx.string_table.get(idx).cloned()
}

pub(crate) fn extract_n(
    args: &[Array1<f64>],
    idx: usize,
    name: &str,
) -> Result<usize, FormulaError> {
    if idx >= args.len() {
        return Err(FormulaError::RuntimeError(format!(
            "{}: missing argument at index {}",
            name, idx
        )));
    }
    let n = args[idx][0] as usize;
    if n == 0 {
        return Err(FormulaError::InvalidParameter(format!(
            "{}: period must be > 0",
            name
        )));
    }
    Ok(n)
}

/// Read a TA-Lib `MAType` selector argument out of `args`.
///
/// This deliberately accepts `0`, which is the code for `SMA` and therefore the
/// *documented default* of every `matype` slot. Routing these through
/// [`extract_n`] made `0` a hard "period must be > 0" error, which left the
/// `0 => Sma` match arm unreachable and made the canonical TA-Lib call
/// `MACDEXT(close, 12, 0, 26, 0, 9, 0)` fail outright on the formula path.
// Every `MAType` code is a tiny non-negative integer, so the saturating
// `f64 as usize` cast is exact; a NaN slot (omitted argument) lands on 0 = SMA.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn extract_ma_code(
    args: &[Array1<f64>],
    idx: usize,
    name: &str,
) -> Result<usize, FormulaError> {
    if idx >= args.len() {
        return Err(FormulaError::RuntimeError(format!(
            "{name}: missing argument at index {idx}"
        )));
    }
    Ok(args[idx][0] as usize)
}

pub(crate) fn extract_f64_arg(
    args: &[Array1<f64>],
    idx: usize,
    name: &str,
) -> Result<f64, FormulaError> {
    if idx >= args.len() {
        return Err(FormulaError::RuntimeError(format!(
            "{}: missing argument at index {}",
            name, idx
        )));
    }
    Ok(args[idx][0])
}

/// Resolve (HIGH, LOW, CLOSE, N) or (CLOSE, N) → auto-expand from context
#[allow(clippy::type_complexity)]
pub(crate) fn resolve_hlc_args<'a>(
    name: &str,
    ctx: &'a FormulaContext,
    args: &'a [Array1<f64>],
) -> Result<(&'a [f64], &'a [f64], &'a [f64], usize), FormulaError> {
    if args.len() >= 4 {
        let n = extract_n(args, 3, name)?;
        Ok((
            args[0].as_slice().unwrap(),
            args[1].as_slice().unwrap(),
            args[2].as_slice().unwrap(),
            n,
        ))
    } else if args.len() >= 2 {
        let n = extract_n(args, 1, name)?;
        Ok((&ctx.high, &ctx.low, &ctx.close, n))
    } else {
        Err(FormulaError::RuntimeError(format!(
            "{name} requires at least 2 arguments (CLOSE,N) or 4 arguments (HIGH,LOW,CLOSE,N), got {}",
            args.len()
        )))
    }
}

/// Resolve (HIGH, LOW, N) or (CLOSE, N) → auto-expand from context
pub(crate) fn resolve_hl_args<'a>(
    name: &str,
    ctx: &'a FormulaContext,
    args: &'a [Array1<f64>],
) -> Result<(&'a [f64], &'a [f64], usize), FormulaError> {
    if args.len() >= 3 {
        let n = extract_n(args, 2, name)?;
        Ok((args[0].as_slice().unwrap(), args[1].as_slice().unwrap(), n))
    } else if !args.is_empty() {
        let n = extract_n(args, 0, name)?;
        Ok((&ctx.high, &ctx.low, n))
    } else {
        Err(FormulaError::RuntimeError(format!(
            "{name} requires at least 1 argument (N) or 3 arguments (HIGH,LOW,N), got {}",
            args.len()
        )))
    }
}

/// Resolve (HIGH, LOW, CLOSE) for ULTOSC: either 3 args or auto-fill from context.
pub(crate) fn resolve_hlc_for_ultosc<'a>(
    name: &str,
    _ctx: &'a FormulaContext,
    args: &'a [Array1<f64>],
) -> Result<(&'a Array1<f64>, &'a Array1<f64>, &'a Array1<f64>), FormulaError> {
    if args.len() >= 3 {
        Ok((&args[0], &args[1], &args[2]))
    } else {
        Err(FormulaError::RuntimeError(format!(
            "{name} requires at least 3 arguments (HIGH,LOW,CLOSE), got {}",
            args.len()
        )))
    }
}

pub(crate) fn resolve_hl_for_dm<'a>(
    name: &str,
    _ctx: &'a FormulaContext,
    args: &'a [Array1<f64>],
) -> Result<(&'a Array1<f64>, &'a Array1<f64>), FormulaError> {
    if args.len() >= 2 {
        Ok((&args[0], &args[1]))
    } else {
        Err(FormulaError::RuntimeError(format!(
            "{name} requires at least 2 arguments (HIGH,LOW), got {}",
            args.len()
        )))
    }
}

pub fn get_builtin_functions() -> HashMap<String, FormulaFn> {
    let mut map: HashMap<String, FormulaFn> = HashMap::new();

    map.insert("MA".to_string(), fn_ma);
    map.insert("EMA".to_string(), fn_ema);
    map.insert("RMA".to_string(), fn_rma);
    map.insert("SMA".to_string(), fn_sma);
    map.insert("WMA".to_string(), fn_wma);
    map.insert("DMA".to_string(), fn_dma);
    map.insert("DEMA".to_string(), fn_dema);
    map.insert("TEMA".to_string(), fn_tema);
    map.insert("KAMA".to_string(), fn_kama);
    map.insert("T3".to_string(), fn_t3);
    map.insert("TRIMA".to_string(), fn_trima);
    map.insert("MAVP".to_string(), fn_mavp);
    map.insert("SAREXT".to_string(), fn_sarext);

    map.insert("HHV".to_string(), fn_hhv);
    map.insert("LLV".to_string(), fn_llv);
    map.insert("HHVBARS".to_string(), fn_hhvbars);
    map.insert("LLVBARS".to_string(), fn_llvbars);

    map.insert("REF".to_string(), fn_ref);
    map.insert("CROSS".to_string(), fn_cross);
    map.insert("CROSSBELOW".to_string(), fn_crossbelow);
    map.insert("LONGCROSS".to_string(), fn_longcross);
    map.insert("IF".to_string(), fn_if);
    map.insert("IFTHEN".to_string(), fn_ifthen);
    map.insert("COUNT".to_string(), fn_count);
    map.insert("SUM".to_string(), fn_sum);
    map.insert("MINUS".to_string(), fn_minus);
    map.insert("EVERY".to_string(), fn_every);
    map.insert("EXIST".to_string(), fn_exist);
    map.insert("FILTER".to_string(), fn_filter);
    map.insert("BARSLAST".to_string(), fn_barslast);
    map.insert("BACKSET".to_string(), fn_backset);
    map.insert("BETWEEN".to_string(), fn_between);
    map.insert("NOT".to_string(), fn_not);

    map.insert("ABS".to_string(), fn_abs);
    map.insert("MAX".to_string(), fn_max);
    map.insert("MIN".to_string(), fn_min);
    map.insert("MAXINDEX".to_string(), fn_maxindex);
    map.insert("MININDEX".to_string(), fn_minindex);
    map.insert("MINMAX".to_string(), fn_minmax);
    map.insert("MINMAXINDEX".to_string(), fn_minmaxindex);
    map.insert("SQRT".to_string(), fn_sqrt);
    map.insert("POW".to_string(), fn_pow);
    map.insert("ADD".to_string(), fn_add);
    map.insert("SUB".to_string(), fn_sub);
    map.insert("MULT".to_string(), fn_mult);
    map.insert("DIV".to_string(), fn_div);
    map.insert("EXP".to_string(), fn_exp);
    map.insert("LOG".to_string(), fn_log);
    map.insert("LN".to_string(), fn_log);
    map.insert("LOG10".to_string(), fn_log10);
    map.insert("SIGN".to_string(), fn_sign);
    map.insert("FLOOR".to_string(), fn_floor);
    map.insert("CEIL".to_string(), fn_ceil);
    map.insert("ROUND".to_string(), fn_round);
    map.insert("SIN".to_string(), fn_sin);
    map.insert("COS".to_string(), fn_cos);
    map.insert("TAN".to_string(), fn_tan);
    map.insert("SINH".to_string(), fn_sinh);
    map.insert("COSH".to_string(), fn_cosh);
    map.insert("TANH".to_string(), fn_tanh);
    map.insert("ASIN".to_string(), fn_asin);
    map.insert("ACOS".to_string(), fn_acos);
    map.insert("ATAN".to_string(), fn_atan);

    map.insert("STD".to_string(), fn_std);
    map.insert("STDDEV".to_string(), fn_std);
    map.insert("VAR".to_string(), fn_var);
    map.insert("ZSCORE".to_string(), fn_zscore);
    map.insert("CORREL".to_string(), fn_correl);
    map.insert("BETA".to_string(), fn_beta);
    map.insert("LINEAR_REG".to_string(), fn_linear_reg);
    // TA-Lib spelling compatibility: LINEARREG is the public function name.
    map.insert("LINEARREG".to_string(), fn_linear_reg);
    map.insert("LINEARREG_ANGLE".to_string(), fn_linear_reg_angle);
    map.insert("LINEARREG_INTERCEPT".to_string(), fn_linear_reg_intercept);
    map.insert("LINEARREG_SLOPE".to_string(), fn_linear_reg_slope);
    map.insert("TSF".to_string(), fn_tsf);
    map.insert("PERCENT_RANK".to_string(), fn_percent_rank);
    map.insert("MIDPOINT".to_string(), fn_midpoint);
    map.insert("MIDPRICE".to_string(), fn_midprice);

    map.insert("RSI".to_string(), fn_rsi);
    map.insert("MACD".to_string(), fn_macd);
    map.insert("DIFF".to_string(), fn_diff);
    map.insert("DEA".to_string(), fn_dea);
    map.insert("BOLL".to_string(), fn_boll);
    map.insert("BOLLUP".to_string(), fn_bollup);
    map.insert("BOLLDN".to_string(), fn_bolldn);
    map.insert("BOLLMID".to_string(), fn_bollmid);
    map.insert("BOLLWIDTH".to_string(), fn_bollwidth);
    map.insert("BBANDS".to_string(), fn_boll);

    map.insert("ATR".to_string(), fn_atr);
    map.insert("NATR".to_string(), fn_natr);
    map.insert("TRANGE".to_string(), fn_trange);

    map.insert("AVGPRICE".to_string(), fn_avgprice);
    map.insert("MEDPRICE".to_string(), fn_medprice);
    map.insert("TYPPRICE".to_string(), fn_typprice);
    map.insert("WCLPRICE".to_string(), fn_wclprice);

    map.insert("OBV".to_string(), fn_obv);
    map.insert("AD".to_string(), fn_ad);
    map.insert("ADOSC".to_string(), fn_adosc);
    map.insert("MFI".to_string(), fn_mfi);

    // Pine-compat helpers used by the formula engine's Pine Script mapper.
    map.insert("MATH_AVG".to_string(), fn_math_avg);
    map.insert("ISNA".to_string(), fn_isna);
    map.insert("FIXNAN".to_string(), fn_fixnan);
    map.insert("VWMA".to_string(), fn_vwma_indicator);

    map.insert("CCI".to_string(), fn_cci);
    map.insert("WILLR".to_string(), fn_willr);
    map.insert("WR".to_string(), fn_willr);
    map.insert("MOM".to_string(), fn_mom);
    map.insert("ROC".to_string(), fn_roc);
    // Rate-of-change ratio variants. These were implemented against the
    // canonical momentum functions but never registered, so a formula could not
    // name them even though the indicator API, the registry and the FFI all
    // expose them.
    map.insert("ROCP".to_string(), fn_rocp);
    map.insert("ROCR".to_string(), fn_rocr);
    map.insert("ROCR100".to_string(), fn_rocr100);
    map.insert("CMO".to_string(), fn_cmo);
    map.insert("TRIX".to_string(), fn_trix);
    map.insert("BOP".to_string(), fn_bop);
    map.insert("APO".to_string(), fn_apo);
    map.insert("PPO".to_string(), fn_ppo);
    map.insert("DPO".to_string(), fn_dpo);

    map.insert("ADX".to_string(), fn_adx);
    map.insert("ADXR".to_string(), fn_adxr);
    map.insert("DMI".to_string(), fn_dmi);
    map.insert("DX".to_string(), fn_dx);
    map.insert("PLUS_DI".to_string(), fn_plus_di);
    map.insert("MINUS_DI".to_string(), fn_minus_di);
    map.insert("AROONOSC".to_string(), fn_aroonosc);
    map.insert("AROON_UP".to_string(), fn_aroon_up);
    map.insert("AROON_DN".to_string(), fn_aroon_dn);
    map.insert("SAR".to_string(), fn_sar);
    map.insert("PSAR".to_string(), fn_sar);

    map.insert("STOCH".to_string(), fn_stoch);
    map.insert("KDJ".to_string(), fn_kdj);
    map.insert("KD".to_string(), fn_kdj);
    map.insert("KDJ_K".to_string(), fn_kdj);
    map.insert("KDJ_D".to_string(), fn_kdj_d);
    map.insert("KDJ_J".to_string(), fn_kdj_j);
    map.insert("BIAS".to_string(), fn_bias);
    map.insert("PSY".to_string(), fn_psy);
    map.insert("HMA".to_string(), fn_hma);
    map.insert("ALMA".to_string(), fn_alma);
    map.insert("CMF".to_string(), fn_cmf);
    map.insert("FISHER".to_string(), fn_fisher);
    map.insert("FISHER_SIGNAL".to_string(), fn_fisher_signal);
    map.insert("TSI".to_string(), fn_tsi);
    map.insert("CHOP".to_string(), fn_chop);

    map.insert("ICHIMOKU_TENKAN".to_string(), fn_ichimoku_tenkan);
    map.insert("ICHIMOKU_KIJUN".to_string(), fn_ichimoku_kijun);
    map.insert("SUPERTREND".to_string(), fn_supertrend);
    map.insert("VWAP".to_string(), fn_vwap);

    map.insert("DONCHIAN".to_string(), fn_donchian);
    map.insert("DONCHIAN_UPPER".to_string(), fn_donchian_upper);
    map.insert("DONCHIAN_LOWER".to_string(), fn_donchian_lower);
    map.insert("DONCHIAN_MIDDLE".to_string(), fn_donchian_middle);
    map.insert("DONCHIAN_WIDTH".to_string(), fn_donchian_width);

    map.insert("STRCAT".to_string(), fn_strcat);
    map.insert("HISTVOL".to_string(), fn_histvol);
    map.insert("OBV_ENHANCED".to_string(), fn_obv_enhanced);
    map.insert("ATR_ENHANCED".to_string(), fn_atr_enhanced);
    map.insert("BOLL_ENHANCED".to_string(), fn_boll_enhanced);

    // Time / Bar functions (TDX)
    map.insert("DATE".to_string(), fn_date_tdx);
    map.insert("TIME".to_string(), fn_time);
    map.insert("YEAR".to_string(), fn_year);
    map.insert("MONTH".to_string(), fn_month);
    map.insert("DAY".to_string(), fn_day);
    map.insert("HOUR".to_string(), fn_hour);
    map.insert("MINUTE".to_string(), fn_minute);
    map.insert("WEEKDAY".to_string(), fn_weekday);
    map.insert("CURRBARSCOUNT".to_string(), fn_currbarscount);
    map.insert("TOTALBARSCOUNT".to_string(), fn_totalbarscount);
    map.insert("BARSSINCE".to_string(), fn_barssince);
    map.insert("BARSSINCEN".to_string(), fn_barssincen);
    map.insert("BARSCOUNT".to_string(), fn_barscount);
    map.insert("BARSTATUS".to_string(), fn_barstatus);
    map.insert("ISLASTBAR".to_string(), fn_islastbar);
    map.insert("FROMOPEN".to_string(), fn_fromopen);

    // Math / Statistics extensions (TDX)
    map.insert("AVEDEV".to_string(), fn_avedev);
    map.insert("AVGDEV".to_string(), fn_avgdev);
    map.insert("DEVSQ".to_string(), fn_devsq);
    map.insert("SLOPE".to_string(), fn_slope);
    map.insert("FORCAST".to_string(), fn_forcast);
    map.insert("RANGE".to_string(), fn_range);
    map.insert("ROLLING_RANGE".to_string(), fn_rolling_range);
    map.insert("CONST".to_string(), fn_const_val);
    map.insert("SUMBARS".to_string(), fn_sumbars);
    map.insert("INTPART".to_string(), fn_intpart);
    map.insert("FRACPART".to_string(), fn_fracpart);
    map.insert("MOD".to_string(), fn_mod);
    map.insert("REVERSE".to_string(), fn_reverse);
    map.insert("TR".to_string(), fn_tr);

    // Index / Finance / Chip data (TDX)
    map.insert("INDEXC".to_string(), fn_indexc);
    map.insert("INDEXO".to_string(), fn_indexo);
    map.insert("INDEXH".to_string(), fn_indexh);
    map.insert("INDEXL".to_string(), fn_indexl);
    map.insert("INDEXV".to_string(), fn_indexv);
    map.insert("INDEXA".to_string(), fn_indexa);
    map.insert("CAPITAL".to_string(), fn_capital);
    map.insert("FINANCE".to_string(), fn_finance);
    map.insert("DYNAINFO".to_string(), fn_dynainfo);
    map.insert("WINNER".to_string(), fn_winner);
    map.insert("LWINNER".to_string(), fn_lwinner);
    map.insert("COST".to_string(), fn_cost);

    // DZH Block functions (大智慧板块引用)
    map.insert("BLOCKDATA".to_string(), fn_blockdata as FormulaFn);
    map.insert("BLOCKINDEX".to_string(), fn_blockindex as FormulaFn);
    map.insert("BLOCKAVG".to_string(), fn_blockavg as FormulaFn);

    // DZH Money flow functions (大智慧资金流向)
    map.insert("MONEYFLOW".to_string(), fn_moneyflow as FormulaFn);
    map.insert("NETINFLOW".to_string(), fn_netinflow as FormulaFn);
    map.insert("BIGORDER".to_string(), fn_bigorder as FormulaFn);
    map.insert("SMALLORDER".to_string(), fn_smallorder as FormulaFn);
    map.insert("MAININFLOW".to_string(), fn_maininflow as FormulaFn);
    map.insert("MAININFLOWPCT".to_string(), fn_maininflowpct as FormulaFn);
    map.insert("SUPERBIGORDER".to_string(), fn_superbigorder as FormulaFn);

    // TDX Aliases
    map.insert("PDI".to_string(), fn_pdi);
    map.insert("MDI".to_string(), fn_mdi);
    map.insert("MTM".to_string(), fn_mtm);

    // THS (同花顺) Aliases
    map.insert("CLOSE1".to_string(), fn_close1);
    map.insert("OPEN1".to_string(), fn_open1);

    // Core reference/condition functions
    map.insert("VALUEWHEN".to_string(), fn_valuewhen);
    map.insert("LAST".to_string(), fn_last);
    map.insert("BARSLASTCOUNT".to_string(), fn_barslastcount);

    // ZigZag series functions
    map.insert("PEAK".to_string(), fn_peak as FormulaFn);
    map.insert("TROUGH".to_string(), fn_trough as FormulaFn);
    map.insert("PEAKBARS".to_string(), fn_peakbars as FormulaFn);
    map.insert("TROUGHBARS".to_string(), fn_troughbars as FormulaFn);
    map.insert("ZIGZAG".to_string(), fn_zigzag as FormulaFn);

    // Advanced find functions
    map.insert("FINDHIGH".to_string(), fn_findhigh as FormulaFn);
    map.insert("FINDLOW".to_string(), fn_findlow as FormulaFn);
    map.insert("DRAWNULL".to_string(), fn_drawnull as FormulaFn);
    map.insert("CEILING".to_string(), fn_ceiling as FormulaFn);

    // Cumulative / sequence operations
    map.insert("CUMSUM".to_string(), fn_cumsum as FormulaFn);
    map.insert("CUM".to_string(), fn_cumsum as FormulaFn);
    map.insert("CUMMAX".to_string(), fn_cummax as FormulaFn);
    map.insert("CUMMIN".to_string(), fn_cummin as FormulaFn);
    map.insert("PERCENTILE".to_string(), fn_percentile as FormulaFn);
    map.insert("MEDIAN".to_string(), fn_median as FormulaFn);

    // Higher-order statistics
    map.insert("SKEW".to_string(), fn_skew as FormulaFn);
    map.insert("KURT".to_string(), fn_kurt as FormulaFn);
    map.insert("MODE".to_string(), fn_mode as FormulaFn);
    map.insert("SORT".to_string(), fn_sort as FormulaFn);
    map.insert("RANK".to_string(), fn_rank as FormulaFn);

    // Multi-period functions
    map.insert("PERIODTYPE".to_string(), fn_periodtype as FormulaFn);
    map.insert("REFDATE".to_string(), fn_refdate as FormulaFn);

    // THS Alert Functions
    map.insert("ALERT".to_string(), fn_alert as FormulaFn);
    map.insert("ALERTONCE".to_string(), fn_alertonce as FormulaFn);

    // THS Statistical Functions
    map.insert("AVGPRICE_N".to_string(), fn_avgprice_n as FormulaFn);
    map.insert("TOTALVOL".to_string(), fn_totalvol as FormulaFn);
    map.insert("MAXPRICE".to_string(), fn_maxprice as FormulaFn);
    map.insert("MINPRICE".to_string(), fn_minprice as FormulaFn);

    // THS Additional Aliases (already implemented as CLOSE1/OPEN1)
    map.insert("HIGH1".to_string(), fn_high1 as FormulaFn);
    map.insert("LOW1".to_string(), fn_low1 as FormulaFn);
    map.insert("VOL1".to_string(), fn_vol1 as FormulaFn);

    // EM (东方财富) Functions
    map.insert("DKCOL".to_string(), fn_dkcol as FormulaFn);
    map.insert("EM_CROSS".to_string(), fn_em_cross as FormulaFn);
    map.insert("EM_REF".to_string(), fn_em_ref as FormulaFn);
    map.insert("EM_ZIG".to_string(), fn_em_zig as FormulaFn);
    map.insert("EM_TROUGH".to_string(), fn_em_trough as FormulaFn);
    map.insert("EM_PEAK".to_string(), fn_em_peak as FormulaFn);
    map.insert("EM_TROUGHBARS".to_string(), fn_em_troughbars as FormulaFn);
    map.insert("EM_PEAKBARS".to_string(), fn_em_peakbars as FormulaFn);
    map.insert("EM_COSTEX".to_string(), fn_em_costex as FormulaFn);
    map.insert("EM_ZLCCV".to_string(), fn_em_zlccv as FormulaFn);

    // FoxTrader (飞狐交易师) compatibility functions
    map.insert("FOX_ZIG".to_string(), fn_fox_zig as FormulaFn);
    map.insert("FOX_TROUGH".to_string(), fn_fox_trough as FormulaFn);
    map.insert("FOX_PEAK".to_string(), fn_fox_peak as FormulaFn);
    map.insert("FOX_TROUGHBARS".to_string(), fn_fox_troughbars as FormulaFn);
    map.insert("FOX_PEAKBARS".to_string(), fn_fox_peakbars as FormulaFn);

    // TA-Lib C compatibility — additional momentum indicators
    map.insert("STOCHF".to_string(), fn_stochf);
    map.insert("STOCHRSI".to_string(), fn_stochrsi);
    map.insert("ULTOSC".to_string(), fn_ultosc);
    map.insert("PLUS_DM".to_string(), fn_plus_dm);
    map.insert("MINUS_DM".to_string(), fn_minus_dm);

    // TA-Lib C compatibility — Hilbert Transform cycle indicators
    map.insert("HT_PHASOR".to_string(), fn_ht_phasor_inner as FormulaFn);
    map.insert("HT_SINE".to_string(), fn_ht_sine_inner as FormulaFn);
    map.insert("HT_DCPERIOD".to_string(), fn_ht_dcperiod as FormulaFn);
    map.insert("HT_DCPHASE".to_string(), fn_ht_dcphase as FormulaFn);
    map.insert("HT_TRENDMODE".to_string(), fn_ht_trendmode as FormulaFn);
    map.insert("HT_TRENDLINE".to_string(), fn_ht_trendline as FormulaFn);
    map.insert("HT_MEASUREMENT".to_string(), fn_ht_measurement as FormulaFn);

    // TA-Lib C compatibility — common candlestick pattern detectors.
    map.insert("CDLDOJI".to_string(), fn_cdl_doji as FormulaFn);
    map.insert(
        "CDLDRAGONFLYDOJI".to_string(),
        fn_cdl_dragonflydoji as FormulaFn,
    );
    map.insert(
        "CDLGRAVESTONEDOJI".to_string(),
        fn_cdl_gravestonedoji as FormulaFn,
    );
    map.insert("CDLENGULFING".to_string(), fn_cdl_engulfing as FormulaFn);
    map.insert("CDLHAMMER".to_string(), fn_cdl_hammer as FormulaFn);
    map.insert("CDLHANGINGMAN".to_string(), fn_cdl_hangingman as FormulaFn);
    map.insert("CDLHARAMI".to_string(), fn_cdl_harami as FormulaFn);
    map.insert("CDLMARUBOZU".to_string(), fn_cdl_marubozu as FormulaFn);
    map.insert("CDLPIERCING".to_string(), fn_cdl_piercing as FormulaFn);
    map.insert(
        "CDLSHOOTINGSTAR".to_string(),
        fn_cdl_shootingstar as FormulaFn,
    );
    map.insert(
        "CDLSPINNINGTOP".to_string(),
        fn_cdl_spinningtop as FormulaFn,
    );
    map.insert("CDL2CROWS".to_string(), fn_cdl_2crows as FormulaFn);
    map.insert(
        "CDL3BLACKCROWS".to_string(),
        fn_cdl_3blackcrows as FormulaFn,
    );
    map.insert("CDL3INSIDE".to_string(), fn_cdl_3inside as FormulaFn);
    map.insert(
        "CDL3LINESTRIKE".to_string(),
        fn_cdl_3linestrike as FormulaFn,
    );
    map.insert("CDL3OUTSIDE".to_string(), fn_cdl_3outside as FormulaFn);
    map.insert(
        "CDL3STARSINSOUTH".to_string(),
        fn_cdl_3starsinsouth as FormulaFn,
    );
    map.insert(
        "CDL3WHITESOLDIERS".to_string(),
        fn_cdl_3whitesoldiers as FormulaFn,
    );
    map.insert(
        "CDLABANDONEDBABY".to_string(),
        fn_cdl_abandonedbaby as FormulaFn,
    );
    map.insert(
        "CDLADVANCEBLOCK".to_string(),
        fn_cdl_advanceblock as FormulaFn,
    );
    map.insert("CDLBELTHOLD".to_string(), fn_cdl_belthold as FormulaFn);
    map.insert("CDLBREAKAWAY".to_string(), fn_cdl_breakaway as FormulaFn);
    map.insert(
        "CDLCLOSINGMARUBOZU".to_string(),
        fn_cdl_closingmarubozu as FormulaFn,
    );
    map.insert(
        "CDLCONCEALBABYSWALL".to_string(),
        fn_cdl_concealbabyswall as FormulaFn,
    );
    map.insert(
        "CDLCOUNTERATTACK".to_string(),
        fn_cdl_counterattack as FormulaFn,
    );
    map.insert(
        "CDLDARKCLOUDCOVER".to_string(),
        fn_cdl_darkcloudcover as FormulaFn,
    );
    map.insert("CDLDOJISTAR".to_string(), fn_cdl_dojistar as FormulaFn);
    map.insert(
        "CDLEVENINGDOJISTAR".to_string(),
        fn_cdl_eveningdojistar as FormulaFn,
    );
    map.insert(
        "CDLEVENINGSTAR".to_string(),
        fn_cdl_eveningstar as FormulaFn,
    );
    map.insert(
        "CDLGAPSIDESIDEWHITE".to_string(),
        fn_cdl_gapsidesidewhite as FormulaFn,
    );
    map.insert(
        "CDLHARAMICROSS".to_string(),
        fn_cdl_haramicross as FormulaFn,
    );
    map.insert("CDLHIGHWAVE".to_string(), fn_cdl_highwave as FormulaFn);
    map.insert("CDLHIKKAKE".to_string(), fn_cdl_hikkake as FormulaFn);
    map.insert("CDLHIKKAKEMOD".to_string(), fn_cdl_hikkakemod as FormulaFn);
    map.insert(
        "CDLHOMINGPIGEON".to_string(),
        fn_cdl_homingpigeon as FormulaFn,
    );
    map.insert(
        "CDLIDENTICAL3CROWS".to_string(),
        fn_cdl_identical3crows as FormulaFn,
    );
    map.insert("CDLINNECK".to_string(), fn_cdl_inneck as FormulaFn);
    map.insert(
        "CDLINVERTEDHAMMER".to_string(),
        fn_cdl_invertedhammer as FormulaFn,
    );
    map.insert("CDLKICKING".to_string(), fn_cdl_kicking as FormulaFn);
    map.insert(
        "CDLKICKINGBYLENGTH".to_string(),
        fn_cdl_kickingbylength as FormulaFn,
    );
    map.insert(
        "CDLLADDERBOTTOM".to_string(),
        fn_cdl_ladderbottom as FormulaFn,
    );
    map.insert(
        "CDLLONGLEGGEDDOJI".to_string(),
        fn_cdl_longleggeddoji as FormulaFn,
    );
    map.insert("CDLLONGLINE".to_string(), fn_cdl_longline as FormulaFn);
    map.insert(
        "CDLMATCHINGLOW".to_string(),
        fn_cdl_matchinglow as FormulaFn,
    );
    map.insert("CDLMATHOLD".to_string(), fn_cdl_mathold as FormulaFn);
    map.insert(
        "CDLMORNINGDOJISTAR".to_string(),
        fn_cdl_morningdojistar as FormulaFn,
    );
    map.insert(
        "CDLMORNINGSTAR".to_string(),
        fn_cdl_morningstar as FormulaFn,
    );
    map.insert("CDLONNECK".to_string(), fn_cdl_onneck as FormulaFn);
    map.insert(
        "CDLRICKSHAWMAN".to_string(),
        fn_cdl_rickshawman as FormulaFn,
    );
    map.insert(
        "CDLRISEFALL3METHODS".to_string(),
        fn_cdl_risefall3methods as FormulaFn,
    );
    map.insert(
        "CDLSEPARATINGLINES".to_string(),
        fn_cdl_separatinglines as FormulaFn,
    );
    map.insert("CDLSHORTLINE".to_string(), fn_cdl_shortline as FormulaFn);
    map.insert(
        "CDLSTALLEDPATTERN".to_string(),
        fn_cdl_stalledpattern as FormulaFn,
    );
    map.insert(
        "CDLSTICKSANDWICH".to_string(),
        fn_cdl_sticksandwich as FormulaFn,
    );
    map.insert("CDLTAKURI".to_string(), fn_cdl_takuri as FormulaFn);
    map.insert("CDLTASUKIGAP".to_string(), fn_cdl_tasukigap as FormulaFn);
    map.insert("CDLTHRUSTING".to_string(), fn_cdl_thrusting as FormulaFn);
    map.insert("CDLTRISTAR".to_string(), fn_cdl_tristar as FormulaFn);
    map.insert(
        "CDLUNIQUE3RIVER".to_string(),
        fn_cdl_unique3river as FormulaFn,
    );
    map.insert(
        "CDLUPSIDEGAP2CROWS".to_string(),
        fn_cdl_upsidegap2crows as FormulaFn,
    );
    map.insert(
        "CDLXSIDEGAP3METHODS".to_string(),
        fn_cdl_xsidegap3methods as FormulaFn,
    );
    map.insert("IMI".to_string(), fn_imi as FormulaFn);

    // TA-Lib C compatibility — additional momentum / statistics
    map.insert("MACDEXT".to_string(), fn_macdext);
    map.insert("MACDFIX".to_string(), fn_macdfix);
    map.insert("PR".to_string(), fn_percent_rank);

    // Classic stock-trading chart patterns (FTA-native, not in TA-Lib)
    map.insert("DARVAS_BOX".to_string(), fn_darvas_box_top as FormulaFn);
    map.insert("RENKO".to_string(), fn_renko as FormulaFn);
    map.insert("KAGI".to_string(), fn_kagi as FormulaFn);
    map.insert(
        "POINT_AND_FIGURE".to_string(),
        fn_point_and_figure as FormulaFn,
    );
    map.insert(
        "THREE_LINE_BREAK".to_string(),
        fn_three_line_break as FormulaFn,
    );
    map.insert(
        "WILLIAMS_ALLIGATOR".to_string(),
        fn_williams_alligator_lips as FormulaFn,
    );
    map.insert("HEIKIN_ASHI".to_string(), fn_heikin_ashi_close as FormulaFn);

    // A-share specific indicators
    map.insert(
        "MAIN_NET_INFLOW".to_string(),
        fn_main_net_inflow as FormulaFn,
    );
    map.insert("MONEY_FLOW".to_string(), fn_money_flow as FormulaFn);
    map.insert("LIMIT_UP".to_string(), fn_limit_up as FormulaFn);
    map.insert("LIMIT_DOWN".to_string(), fn_limit_down as FormulaFn);
    map.insert(
        "CONSECUTIVE_LIMIT".to_string(),
        fn_consecutive_limit as FormulaFn,
    );
    map.insert("TURNOVER".to_string(), fn_turnover as FormulaFn);
    map.insert("RS_RATIO".to_string(), fn_rs_ratio as FormulaFn);

    // Registry aliases are executable compatibility names, not merely documentation.
    // If an alias already has an implementation, it must be the exact same function as
    // its canonical target. This fail-fast invariant prevents MA/SMA-style semantic
    // collisions from silently entering the formula runtime again.
    let registry = crate::registry::builtin_function_registry();
    for spec in registry.iter() {
        let Some(&canonical_fn) = map.get(spec.name) else {
            continue;
        };
        for &alias in spec.aliases {
            if let Some(&existing_fn) = map.get(alias) {
                assert!(
                    std::ptr::fn_addr_eq(existing_fn, canonical_fn),
                    "formula alias {alias} resolves to a different implementation than canonical {}",
                    spec.name
                );
            } else {
                map.insert(alias.to_string(), canonical_fn);
            }
        }
    }

    map
}
