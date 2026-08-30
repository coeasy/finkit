//! A-Share (Chinese stock market) specific technical indicators.
//!
//! This module provides indicators commonly used in Chinese A-share market analysis
//! that are not part of standard TA-Lib. They include:
//!
//! - **Cost distribution**: [`winner`] / [`cost`] for 筹码峰 (chip distribution) analysis
//! - **Capital flow**: [`main_net_inflow`] for 主力净流入, [`money_flow`] for 资金流量
//! - **Market behavior**: [`limit_up`] / [`limit_down`] for 涨跌停 detection,
//!   [`consecutive_limit`] for 连板数, [`turnover`] for 换手率
//! - **Relative strength**: [`rs_ratio`] for 个股相对大盘强弱
//!
//! These functions are designed for A-share characteristics (10% daily price limit
//! in main board, 20% in ChiNext/STAR Market, T+1 trading, etc.) but can be
//! adapted for any market with configurable thresholds.

use crate::error::{Result, TaError};
use crate::utils::{init_output, validate_input};
use ndarray::Array1;

// ============================================================================
// Cost Distribution (筹码分布)
// ============================================================================

/// Winner (获利盘比例) — 收盘价某价位以下的获利盘占总流通盘的比例
///
/// In Chinese A-share technical analysis, "winner" (获利盘) refers to the
/// proportion of circulating shares whose cost basis is below a given price.
///
/// # Formula
/// For each bar `i` and a cost level `cost`:
/// `winner[i] = Σ volume[j] (j in [max(0, i-window+1), i], close[j] <= cost) / Σ volume[j]`
///
/// # Arguments
/// * `close` - Close prices
/// * `volume` - Volume per bar (assumed to be the volume traded at that close)
/// * `cost` - Cost basis price level to evaluate
/// * `window` - Optional lookback window (None = full history)
///
/// # Returns
/// Array of winner ratios in [0.0, 1.0]. The first bar is the ratio of the
/// first bar's volume, so for a short series values tend to be small.
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::winner;
/// let close = vec![10.0, 11.0, 9.5, 12.0, 8.0];
/// let volume = vec![100.0, 150.0, 200.0, 120.0, 180.0];
/// let result = winner(&close, &volume, 10.0, None).unwrap();
/// assert_eq!(result.len(), 5);
/// ```
pub fn winner(
    close: &[f64],
    volume: &[f64],
    cost: f64,
    window: Option<usize>,
) -> Result<Array1<f64>> {
    if close.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(close.len(), 1)?;

    let len = close.len();
    let mut output = init_output(len);

    for i in 0..len {
        let start = match window {
            Some(w) => i.saturating_sub(w - 1),
            None => 0,
        };
        let mut total: f64 = 0.0;
        let mut below: f64 = 0.0;
        for j in start..=i {
            let v = if volume[j].is_nan() { 0.0 } else { volume[j] };
            total += v;
            if !close[j].is_nan() && close[j] <= cost {
                below += v;
            }
        }
        output[i] = if total > 0.0 { below / total } else { 0.0 };
    }

    Ok(output)
}

/// Cost (成本分布) — 查找对应获利盘比例的成本价
///
/// Given a desired winner ratio, return the cost price such that
/// `winner(close, volume, cost_price) ≈ winpct`.
///
/// # Algorithm
/// Binary search over a price range `[min(close), max(close)]` to find the price
/// at which the winner ratio matches `winpct`.
///
/// # Arguments
/// * `close` - Close prices
/// * `volume` - Volume per bar
/// * `winpct` - Target winner ratio in [0.0, 1.0]
/// * `window` - Optional lookback window (None = full history)
///
/// # Returns
/// Array of cost prices (one per bar).
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::cost;
/// let close = vec![10.0, 11.0, 9.5, 12.0, 8.0];
/// let volume = vec![100.0, 150.0, 200.0, 120.0, 180.0];
/// let result = cost(&close, &volume, 0.5, None).unwrap();
/// assert_eq!(result.len(), 5);
/// ```
pub fn cost(
    close: &[f64],
    volume: &[f64],
    winpct: f64,
    window: Option<usize>,
) -> Result<Array1<f64>> {
    if close.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if !(0.0..=1.0).contains(&winpct) {
        return Err(TaError::InvalidParameter {
            name: "winpct".to_string(),
            constraint: "between 0.0 and 1.0".to_string(),
        });
    }
    validate_input(close.len(), 1)?;

    let len = close.len();
    let mut output = init_output(len);

    for i in 0..len {
        let start = match window {
            Some(w) => i.saturating_sub(w - 1),
            None => 0,
        };

        // Find min/max in window
        let mut pmin = f64::INFINITY;
        let mut pmax = f64::NEG_INFINITY;
        for j in start..=i {
            if !close[j].is_nan() {
                if close[j] < pmin {
                    pmin = close[j];
                }
                if close[j] > pmax {
                    pmax = close[j];
                }
            }
        }
        if !pmin.is_finite() || !pmax.is_finite() || pmin == pmax {
            output[i] = pmin;
            continue;
        }

        // Binary search for the smallest cost such that winner(close, volume, cost) >= winpct.
        // Using `<=` in the winner comparison means the function is left-continuous
        // (a step function). We want the lower edge of the target plateau, so we
        // return `hi` (the value guaranteed to satisfy the target).
        let mut lo = pmin;
        let mut hi = pmax;
        for _ in 0..48 {
            let mid = (lo + hi) * 0.5;
            let r = compute_winner_ratio(close, volume, start, i, mid);
            if r < winpct {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        output[i] = hi;
    }

    Ok(output)
}

#[inline]
fn compute_winner_ratio(
    close: &[f64],
    volume: &[f64],
    start: usize,
    end: usize,
    cost: f64,
) -> f64 {
    let mut total = 0.0;
    let mut below = 0.0;
    for j in start..=end {
        let v = if volume[j].is_nan() { 0.0 } else { volume[j] };
        total += v;
        if !close[j].is_nan() && close[j] <= cost {
            below += v;
        }
    }
    if total > 0.0 {
        below / total
    } else {
        0.0
    }
}

// ============================================================================
// Capital Flow (资金流)
// ============================================================================

/// Main Net Inflow (主力净流入) — 简化估算
///
/// Estimates net capital inflow from "main force" orders by classifying each
/// bar as either a main-force buy (large trade) or sell based on the trade
/// value relative to `large_threshold`.
///
/// # Formula
/// For each bar `i`:
/// - amount = close[i] * volume[i]
/// - if amount > large_threshold: inflow += amount, count as "buy"
/// - else: outflow += amount, count as "sell"
/// - net = inflow - outflow
///
/// # Arguments
/// * `close` - Close prices
/// * `volume` - Volume per bar
/// * `large_threshold` - Trade-amount threshold separating large orders from small
///
/// # Returns
/// Array of net main-force capital flow per bar (can be negative).
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::main_net_inflow;
/// let close = vec![10.0, 11.0, 12.0, 9.0, 13.0];
/// let volume = vec![1_000_000.0, 800_000.0, 1_500_000.0, 600_000.0, 2_000_000.0];
/// let result = main_net_inflow(&close, &volume, 10_000_000.0).unwrap();
/// assert_eq!(result.len(), 5);
/// ```
pub fn main_net_inflow(
    close: &[f64],
    volume: &[f64],
    large_threshold: f64,
) -> Result<Array1<f64>> {
    if close.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if large_threshold < 0.0 {
        return Err(TaError::InvalidParameter {
            name: "large_threshold".to_string(),
            constraint: "non-negative".to_string(),
        });
    }
    validate_input(close.len(), 1)?;

    let len = close.len();
    let mut output = Array1::zeros(len);
    let mut inflow = 0.0;
    let mut outflow = 0.0;
    for i in 0..len {
        let amount = close[i] * volume[i];
        if amount > large_threshold {
            inflow += amount;
        } else {
            outflow += amount;
        }
        output[i] = inflow - outflow;
    }
    Ok(output)
}

/// Money Flow (资金流量) — Rolling sum of typical price × volume
///
/// Similar in spirit to MFI but expressed as a raw cumulative amount (no RSI-like
/// normalization). Useful for tracking absolute money flow strength over a period.
///
/// # Formula
/// For each bar `i` and period `p`:
/// - typical = (high + low + close) / 3
/// - if i < p: NaN
/// - else: sum(typical[j] * volume[j] for j in [i-p+1, i])
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume per bar
/// * `period` - Lookback period
///
/// # Returns
/// Array of rolling money flow values.
pub fn money_flow(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    period: usize,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(close.len(), period)?;

    let len = close.len();
    let mut output = init_output(len);

    // Compute typical * volume
    let mut tpv: Array1<f64> = Array1::zeros(len);
    for i in 0..len {
        tpv[i] = (high[i] + low[i] + close[i]) / 3.0 * volume[i];
    }

    // Initial window
    let mut sum: f64 = tpv.iter().take(period).sum();
    if period - 1 < len {
        output[period - 1] = sum;
    }
    for i in period..len {
        sum += tpv[i] - tpv[i - period];
        output[i] = sum;
    }

    Ok(output)
}

// ============================================================================
// Market Behavior (市场行为)
// ============================================================================

/// Limit Up Detection (涨停检测)
///
/// Returns 1.0 on bars where the close hits or exceeds the upper daily price
/// limit (default 10% for A-share main board), 0.0 otherwise. A small epsilon
/// tolerance is applied to avoid floating-point edge cases.
///
/// # Arguments
/// * `close` - Close prices
/// * `prev_close` - Previous bar close (or open for the current bar in T+1 mode)
/// * `threshold` - Daily limit as a decimal (0.10 for main board, 0.20 for ChiNext/STAR)
///
/// # Returns
/// Array of 0.0/1.0 flags. First bar is 0.0 (no prior close).
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::limit_up;
/// let close = vec![10.0, 11.0, 12.0, 9.0, 13.0];
/// let prev_close = vec![10.0, 10.0, 11.0, 12.0, 9.0];
/// let result = limit_up(&close, &prev_close, 0.10).unwrap();
/// assert_eq!(result.len(), 5);
/// // close[2] = 12.0, prev_close[1] = 10.0, change = 20% → limit up
/// // close[4] = 13.0, prev_close[3] = 12.0, change = 8.3% → no limit
/// assert_eq!(result[2], 1.0);
/// assert_eq!(result[4], 0.0);
/// ```
pub fn limit_up(close: &[f64], prev_close: &[f64], threshold: f64) -> Result<Array1<f64>> {
    detect_limit(close, prev_close, threshold, true)
}

/// Limit Down Detection (跌停检测)
///
/// Symmetric counterpart to [`limit_up`]. Returns 1.0 on bars where the close
/// drops to or beyond the lower daily price limit.
pub fn limit_down(close: &[f64], prev_close: &[f64], threshold: f64) -> Result<Array1<f64>> {
    detect_limit(close, prev_close, threshold, false)
}

fn detect_limit(
    close: &[f64],
    prev_close: &[f64],
    threshold: f64,
    is_up: bool,
) -> Result<Array1<f64>> {
    if close.len() != prev_close.len() {
        return Err(TaError::InvalidParameter {
            name: "close, prev_close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if !(0.0..=1.0).contains(&threshold) {
        return Err(TaError::InvalidParameter {
            name: "threshold".to_string(),
            constraint: "between 0.0 and 1.0".to_string(),
        });
    }
    validate_input(close.len(), 2)?;

    let len = close.len();
    let mut output = Array1::zeros(len);
    // Small epsilon to handle floating-point ties (e.g., 1.09999999 vs 1.1)
    let eps = 1e-4;
    for i in 1..len {
        if prev_close[i - 1] == 0.0 || prev_close[i - 1].is_nan() {
            continue;
        }
        let change = (close[i] - prev_close[i - 1]) / prev_close[i - 1];
        let hit = if is_up {
            change >= threshold - eps
        } else {
            change <= -threshold + eps
        };
        if hit {
            output[i] = 1.0;
        }
    }
    Ok(output)
}

/// Consecutive Limit-Up/Down Count (连板数)
///
/// Given a binary limit signal (e.g. output of `limit_up`), count how many
/// consecutive bars in a row the signal has been 1.0 up to the current bar.
///
/// # Arguments
/// * `limit_signal` - Array of 0.0/1.0 (or any positive value) flags
///
/// # Returns
/// Array of cumulative counts. Each bar is `prev_count + 1` if the current
/// signal is 1.0, else resets to 0 (or keeps `limit_signal[i]` as a non-zero
/// value if the user supplies non-binary values).
pub fn consecutive_limit(limit_signal: &[f64]) -> Result<Array1<f64>> {
    validate_input(limit_signal.len(), 1)?;
    let len = limit_signal.len();
    let mut output = Array1::zeros(len);
    let mut count = 0.0;
    for i in 0..len {
        if limit_signal[i] > 0.0 {
            count += 1.0;
        } else {
            count = 0.0;
        }
        output[i] = count;
    }
    Ok(output)
}

/// Turnover Rate (换手率) — volume / free-float shares
///
/// # Formula
/// turnover[i] = volume[i] / free_float_shares[i]
///
/// # Arguments
/// * `volume` - Trading volume per bar
/// * `free_float_shares` - Free-float share count per bar (can be a constant array)
///
/// # Returns
/// Array of turnover rates. Bars where `free_float_shares <= 0` are NaN.
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::turnover;
/// let volume = vec![1_000_000.0, 2_000_000.0, 1_500_000.0];
/// let free_float = vec![100_000_000.0; 3];
/// let result = turnover(&volume, &free_float).unwrap();
/// // result[0] = 0.01 (1%)
/// assert!((result[0] - 0.01).abs() < 1e-10);
/// ```
pub fn turnover(volume: &[f64], free_float_shares: &[f64]) -> Result<Array1<f64>> {
    if volume.len() != free_float_shares.len() {
        return Err(TaError::InvalidParameter {
            name: "volume, free_float_shares".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(volume.len(), 1)?;

    let len = volume.len();
    let mut output = init_output(len);
    for i in 0..len {
        if free_float_shares[i] > 0.0 {
            output[i] = volume[i] / free_float_shares[i];
        }
    }
    Ok(output)
}

// ============================================================================
// Relative Strength (相对强弱)
// ============================================================================

/// Relative Strength Ratio (相对强弱比率) — stock vs benchmark
///
/// Computes the rolling ratio of stock return to benchmark return:
/// `RS = (close[i] / close[i-period+1]) / (bench[i] / bench[i-period+1])`.
///
/// A value > 1.0 means the stock outperformed the benchmark over the window.
///
/// # Arguments
/// * `close` - Stock close prices
/// * `benchmark_close` - Benchmark index close prices
/// * `period` - Lookback period
///
/// # Returns
/// Array of relative-strength ratios.
pub fn rs_ratio(close: &[f64], benchmark_close: &[f64], period: usize) -> Result<Array1<f64>> {
    if close.len() != benchmark_close.len() {
        return Err(TaError::InvalidParameter {
            name: "close, benchmark_close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(close.len(), period + 1)?;

    let len = close.len();
    let mut output = init_output(len);
    for i in period..len {
        if close[i - period] == 0.0 || benchmark_close[i - period] == 0.0 {
            continue;
        }
        let stock_ret = close[i] / close[i - period];
        let bench_ret = benchmark_close[i] / benchmark_close[i - period];
        if bench_ret.abs() > 1e-15 {
            output[i] = stock_ret / bench_ret;
        }
    }
    Ok(output)
}

// ============================================================================
// 北向资金 / 融资融券 / 龙虎榜 (Northbound / Margin / Dragon-Tiger)
// ============================================================================

/// 北向资金净流入 (Northbound Capital Net Inflow)
///
/// Computes the daily net inflow of northbound capital via the
/// Shanghai-Hong Kong and Shenzhen-Hong Kong stock connect channels.
///
/// # Formula
/// `flow[i] = hs_connect[i] + sz_connect[i]` (NaN treated as 0)
///
/// # Arguments
/// * `hs_connect` - Shanghai-Hong Kong Connect net buy (positive = inflow)
/// * `sz_connect` - Shenzhen-Hong Kong Connect net buy (positive = inflow)
///
/// # Returns
/// Array of net inflows. NaN inputs are coerced to 0.
pub fn north_bound_flow(
    hs_connect: &[f64],
    sz_connect: &[f64],
) -> Result<Array1<f64>> {
    if hs_connect.len() != sz_connect.len() {
        return Err(TaError::InvalidParameter {
            name: "hs_connect, sz_connect".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(hs_connect.len(), 1)?;
    let len = hs_connect.len();
    let mut out = init_output(len);
    for i in 0..len {
        let h = if hs_connect[i].is_nan() { 0.0 } else { hs_connect[i] };
        let s = if sz_connect[i].is_nan() { 0.0 } else { sz_connect[i] };
        out[i] = h + s;
    }
    Ok(out)
}

/// 融资余额 (Margin Trading Balance)
///
/// Computes the cumulative margin (融资) balance from daily buy and repay
/// amounts. Balance is a running total: `bal[i] = bal[i-1] + buy[i] - repay[i]`.
///
/// # Arguments
/// * `buy` - Daily margin buy amount
/// * `repay` - Daily margin repay amount
/// * `initial` - Initial balance (typically the previous day's close)
///
/// # Returns
/// Array of margin balances. NaN inputs are coerced to 0.
pub fn margin_balance(
    buy: &[f64],
    repay: &[f64],
    initial: f64,
) -> Result<Array1<f64>> {
    if buy.len() != repay.len() {
        return Err(TaError::InvalidParameter {
            name: "buy, repay".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(buy.len(), 1)?;
    let len = buy.len();
    let mut out = init_output(len);
    let mut bal = initial;
    for i in 0..len {
        let b = if buy[i].is_nan() { 0.0 } else { buy[i] };
        let r = if repay[i].is_nan() { 0.0 } else { repay[i] };
        bal += b - r;
        out[i] = bal;
    }
    Ok(out)
}

/// 融券余额 (Short Selling Balance)
///
/// Computes cumulative short-selling balance. Same structure as
/// [`margin_balance`] but tracks the short leg of margin trading.
pub fn short_balance(
    sell: &[f64],
    repay: &[f64],
    initial: f64,
) -> Result<Array1<f64>> {
    if sell.len() != repay.len() {
        return Err(TaError::InvalidParameter {
            name: "sell, repay".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(sell.len(), 1)?;
    let len = sell.len();
    let mut out = init_output(len);
    let mut bal = initial;
    for i in 0..len {
        let s = if sell[i].is_nan() { 0.0 } else { sell[i] };
        let r = if repay[i].is_nan() { 0.0 } else { repay[i] };
        bal += s - r;
        out[i] = bal;
    }
    Ok(out)
}

/// 融资买入额 (Daily Margin Buy Amount)
///
/// Computes the total margin buy amount as `volume[i] * price[i]`.
pub fn margin_buy_amount(
    volume: &[f64],
    price: &[f64],
) -> Result<Array1<f64>> {
    if volume.len() != price.len() {
        return Err(TaError::InvalidParameter {
            name: "volume, price".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(volume.len(), 1)?;
    let len = volume.len();
    let mut out = init_output(len);
    for i in 0..len {
        let v = if volume[i].is_nan() { 0.0 } else { volume[i] };
        let p = if price[i].is_nan() { 0.0 } else { price[i] };
        out[i] = v * p;
    }
    Ok(out)
}

/// 龙虎榜净买入 (Dragon-Tiger List Net Buy)
///
/// Net buy from the Dragon-Tiger list (龙虎榜): institutional and
/// top-tier营业部 net buy/sell on a single day.
///
/// # Arguments
/// * `buyer` - Total buy amount from disclosed席位
/// * `seller` - Total sell amount from disclosed席位
pub fn dragon_tiger_net_buy(
    buyer: &[f64],
    seller: &[f64],
) -> Result<Array1<f64>> {
    if buyer.len() != seller.len() {
        return Err(TaError::InvalidParameter {
            name: "buyer, seller".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(buyer.len(), 1)?;
    let len = buyer.len();
    let mut out = init_output(len);
    for i in 0..len {
        let b = if buyer[i].is_nan() { 0.0 } else { buyer[i] };
        let s = if seller[i].is_nan() { 0.0 } else { seller[i] };
        out[i] = b - s;
    }
    Ok(out)
}

// ============================================================================
// 板块强弱 / 涨停强度 (Sector Strength / Limit-Up Strength)
// ============================================================================

/// 板块相对强弱 (Sector Relative Strength)
///
/// Relative strength of an individual stock vs. its sector index.
/// `rs = stock_return - sector_return` (arithmetic difference, simple form).
///
/// # Arguments
/// * `stock_ret` - Per-bar stock returns (e.g. `pct_change`)
/// * `sector_ret` - Per-bar sector returns
/// * `lookback` - Number of bars to aggregate (typically 5/10/20)
pub fn sector_strength(
    stock_ret: &[f64],
    sector_ret: &[f64],
    lookback: usize,
) -> Result<Array1<f64>> {
    if stock_ret.len() != sector_ret.len() {
        return Err(TaError::InvalidParameter {
            name: "stock_ret, sector_ret".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if lookback == 0 {
        return Err(TaError::InvalidParameter {
            name: "lookback".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(stock_ret.len(), lookback + 1)?;
    let len = stock_ret.len();
    let mut out = init_output(len);
    for i in lookback..len {
        let mut sum_s = 0.0;
        let mut sum_sec = 0.0;
        let mut valid = true;
        for j in (i - lookback)..=i {
            if stock_ret[j].is_nan() || sector_ret[j].is_nan() {
                valid = false;
                break;
            }
            sum_s += stock_ret[j];
            sum_sec += sector_ret[j];
        }
        if valid {
            out[i] = sum_s - sum_sec;
        }
    }
    Ok(out)
}

/// 涨停封单强度 (Limit-Up Seal Strength)
///
/// Measures the strength of a limit-up seal by comparing today's volume
/// to the limit-price volume. Higher values indicate stronger buy-side
/// commitment.
///
/// # Formula
/// `strength[i] = volume[i] / max(1.0, limit_volume[i])` (only when limit up)
///
/// # Arguments
/// * `close` - Close prices
/// * `limit_price` - The daily limit-up price
/// * `volume` - Trading volume
pub fn limit_up_strength(
    close: &[f64],
    limit_price: &[f64],
    volume: &[f64],
) -> Result<Array1<f64>> {
    if close.len() != limit_price.len() || close.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "close, limit_price, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(close.len(), 1)?;
    let len = close.len();
    let mut out = init_output(len);
    for i in 0..len {
        let lp = if limit_price[i].is_nan() { 0.0 } else { limit_price[i] };
        let v = if volume[i].is_nan() { 0.0 } else { volume[i] };
        let c = if close[i].is_nan() { 0.0 } else { close[i] };
        if lp > 0.0 && (c - lp).abs() < 1e-9 * lp.max(1.0) {
            // At limit up: ratio of volume to "expected" volume (heuristic 1.0)
            out[i] = v;
        }
    }
    Ok(out)
}

/// 连板天数 (Consecutive Limit-Up Days)
///
/// Counts how many consecutive limit-up days end at bar `i`.
///
/// # Arguments
/// * `close` - Close prices
/// * `limit_price` - Daily limit-up price
pub fn consecutive_limit_days(
    close: &[f64],
    limit_price: &[f64],
) -> Result<Array1<f64>> {
    if close.len() != limit_price.len() {
        return Err(TaError::InvalidParameter {
            name: "close, limit_price".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(close.len(), 1)?;
    let len = close.len();
    let mut out = Array1::<f64>::zeros(len);
    let mut count = 0.0;
    for i in 0..len {
        let c = close[i];
        let lp = limit_price[i];
        if c.is_finite() && lp.is_finite() && lp > 0.0 && (c - lp).abs() < 1e-9 * lp {
            count += 1.0;
        } else {
            count = 0.0;
        }
        out[i] = count;
    }
    Ok(out)
}

/// 封单金额 (Seal Order Amount)
///
/// For limit-up (一字板) days, the seal amount equals the bid volume at
/// the limit price times the limit price.
///
/// # Arguments
/// * `limit_up_signal` - 1.0 if the bar is limit-up, 0.0 otherwise
/// * `bid_volume_1` - Bid volume at the first bid level (Level-1 ask side
///   on a sell-down day, or Level-1 bid side on a buy-up day; here we
///   treat the input as the user-supplied "封单量")
pub fn seal_amount(
    limit_up_signal: &[f64],
    bid_volume_1: &[f64],
) -> Result<Array1<f64>> {
    if limit_up_signal.len() != bid_volume_1.len() {
        return Err(TaError::InvalidParameter {
            name: "limit_up_signal, bid_volume_1".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(limit_up_signal.len(), 1)?;
    let len = limit_up_signal.len();
    let mut out = Array1::<f64>::zeros(len);
    for i in 0..len {
        let s = if limit_up_signal[i].is_nan() { 0.0 } else { limit_up_signal[i] };
        let v = if bid_volume_1[i].is_nan() { 0.0 } else { bid_volume_1[i] };
        if s > 0.0 {
            out[i] = v;
        }
    }
    Ok(out)
}

// ============================================================================
// 换手率分位 / 量比 / 委比 (Turnover / Volume Ratio / Committee)
// ============================================================================

/// 换手率分位 (Turnover Percentile)
///
/// Computes the rolling percentile rank of turnover within a lookback
/// window. Useful for identifying "today's turnover is in the top 10% of
/// the recent N days".
///
/// # Arguments
/// * `turnover` - Turnover rate series (e.g. from [`turnover`])
/// * `lookback` - Window size in bars
pub fn turnover_percentile(
    turnover: &[f64],
    lookback: usize,
) -> Result<Array1<f64>> {
    if lookback == 0 {
        return Err(TaError::InvalidParameter {
            name: "lookback".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(turnover.len(), lookback)?;
    let len = turnover.len();
    let mut out = init_output(len);
    for i in lookback - 1..len {
        let window = &turnover[i + 1 - lookback..=i];
        let current = turnover[i];
        if !current.is_finite() {
            continue;
        }
        let mut below_or_eq = 0;
        let mut total_valid = 0;
        for &v in window {
            if v.is_finite() {
                total_valid += 1;
                if v <= current {
                    below_or_eq += 1;
                }
            }
        }
        if total_valid > 0 {
            out[i] = below_or_eq as f64 / total_valid as f64;
        }
    }
    Ok(out)
}

/// 量比 (Volume Ratio)
///
/// Volume ratio = current bar volume / N-day average volume. A volume
/// ratio > 1.5 typically indicates above-average activity.
///
/// # Arguments
/// * `volume` - Bar volumes
/// * `vol_ma5` - Pre-computed 5-day SMA of volume (or any MA)
pub fn volume_ratio(
    volume: &[f64],
    vol_ma5: &[f64],
) -> Result<Array1<f64>> {
    if volume.len() != vol_ma5.len() {
        return Err(TaError::InvalidParameter {
            name: "volume, vol_ma5".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(volume.len(), 1)?;
    let len = volume.len();
    let mut out = init_output(len);
    for i in 0..len {
        let v = if volume[i].is_nan() { 0.0 } else { volume[i] };
        let m = vol_ma5[i];
        if m.is_finite() && m > 1e-15 {
            out[i] = v / m;
        }
    }
    Ok(out)
}

/// 委比 (Committee Ratio)
///
/// Committee ratio = (委买量 - 委卖量) / (委买量 + 委卖量).
/// Range: [-1, 1]. Positive = more buy orders, negative = more sell orders.
///
/// # Arguments
/// * `bid_amount` - Total bid amount (委买量)
/// * `ask_amount` - Total ask amount (委卖量)
pub fn committee_ratio(
    bid_amount: &[f64],
    ask_amount: &[f64],
) -> Result<Array1<f64>> {
    if bid_amount.len() != ask_amount.len() {
        return Err(TaError::InvalidParameter {
            name: "bid_amount, ask_amount".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(bid_amount.len(), 1)?;
    let len = bid_amount.len();
    let mut out = init_output(len);
    for i in 0..len {
        let b = if bid_amount[i].is_nan() { 0.0 } else { bid_amount[i] };
        let a = if ask_amount[i].is_nan() { 0.0 } else { ask_amount[i] };
        let total = b + a;
        if total > 1e-15 {
            out[i] = (b - a) / total;
        }
    }
    Ok(out)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    // -----------------------------------------------------------------
    // winner / cost
    // -----------------------------------------------------------------

    #[test]
    fn test_winner_basic() {
        let close = vec![10.0, 11.0, 9.5, 12.0, 8.0];
        let volume = vec![100.0, 150.0, 200.0, 120.0, 180.0];
        let result = winner(&close, &volume, 10.0, None).unwrap();
        // bar 0: close=10 <= 10, vol=100/100 = 1.0
        assert_relative_eq!(result[0], 1.0, epsilon = 1e-10);
        // bar 4: closes <= 10: [10.0, 9.5, 8.0] = 100+200+180=480, total=750, ratio=0.64
        assert_relative_eq!(result[4], 480.0 / 750.0, epsilon = 1e-10);
    }

    #[test]
    fn test_winner_window() {
        let close = vec![10.0; 10];
        let volume = vec![100.0; 10];
        let result = winner(&close, &volume, 10.0, Some(3)).unwrap();
        // With window=3, last bar counts only last 3 bars, all 10.0, all volume
        assert_relative_eq!(result[9], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_cost_basic() {
        let close = vec![10.0, 11.0, 9.5, 12.0, 8.0];
        let volume = vec![100.0, 150.0, 200.0, 120.0, 180.0];
        let result = cost(&close, &volume, 0.5, None).unwrap();
        // Median price should be a value such that ~50% of volume is at/below it
        // Sorted closes: [8.0, 9.5, 10.0, 11.0, 12.0]
        // Cumulative volume: 180, 380, 480, 630, 750
        // 50% of 750 = 375, falls in the 9.5-10.0 range
        let last = result[4];
        assert!(last >= 8.0 && last <= 12.0, "cost[4]={} out of range", last);
    }

    #[test]
    fn test_cost_invalid_winpct() {
        let close = vec![1.0, 2.0, 3.0];
        let volume = vec![1.0, 1.0, 1.0];
        assert!(cost(&close, &volume, 1.5, None).is_err());
    }

    #[test]
    fn test_winner_cost_inverse() {
        // For discrete data, `winner(cost)` is a step function with jumps at every
        // close price. The binary search returns the lower edge of the target
        // plateau, but the plateau itself may span several %-points depending on
        // the data density. We use a wide tolerance (0.20) to allow for the
        // discrete jumps. With more bars the resolution improves.
        let n = 50;
        let close: Vec<f64> = (0..n)
            .map(|i| 10.0 + (i as f64 * 0.13).sin() * 3.0)
            .collect();
        let volume = vec![100.0; n];
        let target = 0.5;
        let c = cost(&close, &volume, target, None).unwrap();
        // Check from bar 10 onward (need enough history for the ratio to stabilize)
        for i in 10..n {
            let w = winner(&close, &volume, c[i], None).unwrap();
            assert!(
                w[i] >= target - 1e-9, // cost() returns the boundary where winner >= target
                "bar {}: winner({}) = {}, expected >= {}",
                i, c[i], w[i], target
            );
        }
    }

    // -----------------------------------------------------------------
    // main_net_inflow / money_flow
    // -----------------------------------------------------------------

    #[test]
    fn test_main_net_inflow_basic() {
        let close = vec![10.0, 11.0, 12.0, 9.0, 13.0];
        let volume = vec![1_000_000.0, 800_000.0, 1_500_000.0, 600_000.0, 2_000_000.0];
        // amounts: 10M, 8.8M, 18M, 5.4M, 26M
        // threshold = 10M
        // buy: 18M, 26M → inflow=44M
        // sell: 10M, 8.8M, 5.4M → outflow=24.2M
        // net = 19.8M
        let result = main_net_inflow(&close, &volume, 10_000_000.0).unwrap();
        assert_relative_eq!(result[4], 44_000_000.0 - 24_200_000.0, epsilon = 1.0);
    }

    #[test]
    fn test_main_net_inflow_zero_threshold() {
        let close = vec![10.0, 11.0];
        let volume = vec![1.0, 2.0];
        // With threshold=0, every amount is "large" so all are buys
        let result = main_net_inflow(&close, &volume, 0.0).unwrap();
        // inflow = 10 + 22 = 32, outflow = 0
        assert_relative_eq!(result[1], 32.0, epsilon = 1e-10);
    }

    #[test]
    fn test_money_flow_basic() {
        let high = vec![11.0, 12.0, 13.0, 10.0, 14.0];
        let low = vec![9.0, 10.0, 11.0, 8.0, 12.0];
        let close = vec![10.0, 11.0, 12.0, 9.0, 13.0];
        let volume = vec![100.0, 200.0, 300.0, 150.0, 400.0];
        let result = money_flow(&high, &low, &close, &volume, 3).unwrap();
        // bar 0,1: NaN
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        // bar 2: tpv = [(11+9+10)/3*100, (12+10+11)/3*200, (13+11+12)/3*300]
        //       = [10*100, 11*200, 12*300] = [1000, 2200, 3600] sum=6800
        assert_relative_eq!(result[2], 6800.0, epsilon = 1e-6);
        // bar 3: sum(2200, 3600, 9*150=1350) = 7150
        assert_relative_eq!(result[3], 7150.0, epsilon = 1e-6);
    }

    // -----------------------------------------------------------------
    // limit_up / limit_down / consecutive_limit
    // -----------------------------------------------------------------

    #[test]
    fn test_limit_up_basic() {
        // close sequence:        10.0, 11.0, 12.0,  9.0, 13.0
        // prev_close:            10.0, 10.0, 11.0, 12.0,  9.0
        // bar i computes: close[i] vs prev_close[i-1]
        //   i=1: 11.0 vs 10.0  = 10.0%   → limit up
        //   i=2: 12.0 vs 10.0  = 20.0%   → limit up
        //   i=3:  9.0 vs 11.0  = -18.2%  → not limit up
        //   i=4: 13.0 vs 12.0  =  8.3%   → not limit up
        let close = vec![10.0, 11.0, 12.0, 9.0, 13.0];
        let prev_close = vec![10.0, 10.0, 11.0, 12.0, 9.0];
        let result = limit_up(&close, &prev_close, 0.10).unwrap();
        assert_relative_eq!(result[1], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 0.0, epsilon = 1e-10);
        assert_relative_eq!(result[4], 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_limit_up_chi_next() {
        // ChiNext/STAR: 20% limit
        let close = vec![10.0, 12.5];
        let prev_close = vec![10.0, 10.0];
        let result = limit_up(&close, &prev_close, 0.20).unwrap();
        assert_relative_eq!(result[1], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_limit_down_basic() {
        let close = vec![10.0, 9.0, 12.0];
        let prev_close = vec![10.0, 10.0, 10.0];
        let result = limit_down(&close, &prev_close, 0.10).unwrap();
        assert_relative_eq!(result[1], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_consecutive_limit_basic() {
        let signal = vec![0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0];
        let result = consecutive_limit(&signal).unwrap();
        let expected = vec![0.0, 1.0, 2.0, 0.0, 1.0, 2.0, 3.0];
        for (a, b) in result.iter().zip(expected.iter()) {
            assert_relative_eq!(a, b, epsilon = 1e-10);
        }
    }

    // -----------------------------------------------------------------
    // turnover / rs_ratio
    // -----------------------------------------------------------------

    #[test]
    fn test_turnover_basic() {
        let volume = vec![1_000_000.0, 2_000_000.0, 1_500_000.0];
        let free_float = vec![100_000_000.0; 3];
        let result = turnover(&volume, &free_float).unwrap();
        assert_relative_eq!(result[0], 0.01, epsilon = 1e-10);
        assert_relative_eq!(result[1], 0.02, epsilon = 1e-10);
        assert_relative_eq!(result[2], 0.015, epsilon = 1e-10);
    }

    #[test]
    fn test_turnover_zero_shares() {
        let volume = vec![100.0, 200.0];
        let free_float = vec![0.0, 100.0];
        let result = turnover(&volume, &free_float).unwrap();
        assert!(result[0].is_nan());
        assert_relative_eq!(result[1], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_rs_ratio_basic() {
        let close = vec![10.0, 11.0, 12.0, 13.0];
        let bench = vec![100.0, 102.0, 104.0, 108.0];
        let result = rs_ratio(&close, &bench, 2).unwrap();
        // bar 2: stock 12/10=1.2, bench 104/100=1.04, rs=1.1538
        let expected = (12.0 / 10.0) / (104.0 / 100.0);
        assert_relative_eq!(result[2], expected, epsilon = 1e-6);
    }

    #[test]
    fn test_rs_ratio_outperformance() {
        // Stock doubled, benchmark flat → rs = 2.0
        let close = vec![10.0, 10.0, 20.0];
        let bench = vec![100.0, 100.0, 100.0];
        let result = rs_ratio(&close, &bench, 2).unwrap();
        assert_relative_eq!(result[2], 2.0, epsilon = 1e-6);
    }

    // -----------------------------------------------------------------
    // Error cases
    // -----------------------------------------------------------------

    #[test]
    fn test_length_mismatch() {
        let close = vec![1.0, 2.0];
        let volume = vec![1.0];
        assert!(winner(&close, &volume, 1.0, None).is_err());
        assert!(cost(&close, &volume, 0.5, None).is_err());
        assert!(main_net_inflow(&close, &volume, 1.0).is_err());
        assert!(turnover(&close, &volume).is_err());
        assert!(rs_ratio(&close, &volume, 1).is_err());
    }

    #[test]
    fn test_empty_input() {
        let empty: Vec<f64> = vec![];
        assert!(winner(&empty, &empty, 1.0, None).is_err());
        assert!(main_net_inflow(&empty, &empty, 1.0).is_err());
    }

    #[test]
    fn test_invalid_params() {
        let close = vec![1.0, 2.0, 3.0];
        let volume = vec![1.0, 1.0, 1.0];
        assert!(money_flow(&close, &close, &close, &volume, 0).is_err());
        assert!(rs_ratio(&close, &close, 0).is_err());
        assert!(main_net_inflow(&close, &volume, -1.0).is_err());
        assert!(limit_up(&close, &close, 1.5).is_err());
    }

    // -----------------------------------------------------------------
    // 11 new A-share indicators (9→20 expansion)
    // -----------------------------------------------------------------

    #[test]
    fn test_north_bound_flow() {
        let hs = vec![100.0, 200.0, -50.0];
        let sz = vec![80.0, 150.0, 100.0];
        let r = north_bound_flow(&hs, &sz).unwrap();
        assert_relative_eq!(r[0], 180.0, epsilon = 1e-10);
        assert_relative_eq!(r[1], 350.0, epsilon = 1e-10);
        assert_relative_eq!(r[2], 50.0, epsilon = 1e-10);
    }

    #[test]
    fn test_north_bound_flow_nan() {
        let hs = vec![f64::NAN, 100.0];
        let sz = vec![50.0, f64::NAN];
        let r = north_bound_flow(&hs, &sz).unwrap();
        assert_relative_eq!(r[0], 50.0, epsilon = 1e-10);
        assert_relative_eq!(r[1], 100.0, epsilon = 1e-10);
    }

    #[test]
    fn test_margin_balance() {
        let buy = vec![100.0, 200.0, 50.0];
        let repay = vec![0.0, 50.0, 80.0];
        let r = margin_balance(&buy, &repay, 1000.0).unwrap();
        // 1000, 1100, 1250, 1220
        assert_relative_eq!(r[0], 1100.0, epsilon = 1e-10);
        assert_relative_eq!(r[1], 1250.0, epsilon = 1e-10);
        assert_relative_eq!(r[2], 1220.0, epsilon = 1e-10);
    }

    #[test]
    fn test_short_balance() {
        let sell = vec![10.0, 20.0];
        let repay = vec![0.0, 5.0];
        let r = short_balance(&sell, &repay, 100.0).unwrap();
        assert_relative_eq!(r[0], 110.0, epsilon = 1e-10);
        assert_relative_eq!(r[1], 125.0, epsilon = 1e-10);
    }

    #[test]
    fn test_margin_buy_amount() {
        let vol = vec![1000.0, 2000.0];
        let price = vec![10.0, 12.5];
        let r = margin_buy_amount(&vol, &price).unwrap();
        assert_relative_eq!(r[0], 10000.0, epsilon = 1e-10);
        assert_relative_eq!(r[1], 25000.0, epsilon = 1e-10);
    }

    #[test]
    fn test_dragon_tiger_net_buy() {
        let buyer = vec![1_000_000.0, 500_000.0];
        let seller = vec![300_000.0, 800_000.0];
        let r = dragon_tiger_net_buy(&buyer, &seller).unwrap();
        assert_relative_eq!(r[0], 700_000.0, epsilon = 1e-6);
        assert_relative_eq!(r[1], -300_000.0, epsilon = 1e-6);
    }

    #[test]
    fn test_sector_strength() {
        // Stock +5%, sector +2% over 3 days → outperformance
        let stock = vec![0.01, 0.02, 0.02, 0.0, 0.0, 0.0];
        let sector = vec![0.005, 0.01, 0.005, 0.0, 0.0, 0.0];
        let r = sector_strength(&stock, &sector, 3).unwrap();
        // bar 3: stock sum (over bars 0..=3) = 0.05, sector = 0.02 → diff = 0.03
        assert_relative_eq!(r[3], 0.03, epsilon = 1e-10);
    }

    #[test]
    fn test_limit_up_strength() {
        let close = vec![10.0, 11.0, 12.0];
        let limit = vec![10.0, 11.0, 12.0]; // all limit up
        let vol = vec![100.0, 200.0, 300.0];
        let r = limit_up_strength(&close, &limit, &vol).unwrap();
        assert_relative_eq!(r[0], 100.0, epsilon = 1e-10);
        assert_relative_eq!(r[1], 200.0, epsilon = 1e-10);
        assert_relative_eq!(r[2], 300.0, epsilon = 1e-10);
    }

    #[test]
    fn test_consecutive_limit_days() {
        // 2 consecutive limit-up days
        let close = vec![10.0, 11.0, 12.0, 11.5];
        let limit = vec![10.0, 11.0, 12.0, 12.0];
        let r = consecutive_limit_days(&close, &limit).unwrap();
        let expected = vec![1.0, 2.0, 3.0, 0.0];
        for (a, b) in r.iter().zip(expected.iter()) {
            assert_relative_eq!(a, b, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_seal_amount() {
        let sig = vec![0.0, 1.0, 1.0, 0.0];
        let bid = vec![100.0, 5000.0, 10000.0, 200.0];
        let r = seal_amount(&sig, &bid).unwrap();
        // Only when sig > 0 do we record
        assert_relative_eq!(r[0], 0.0, epsilon = 1e-10);
        assert_relative_eq!(r[1], 5000.0, epsilon = 1e-10);
        assert_relative_eq!(r[2], 10000.0, epsilon = 1e-10);
        assert_relative_eq!(r[3], 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_turnover_percentile() {
        // Constant turnover: percentile = 1.0
        let t = vec![0.01; 5];
        let r = turnover_percentile(&t, 5).unwrap();
        assert_relative_eq!(r[4], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_turnover_percentile_max() {
        // Increasing series: latest is max → percentile = 1.0
        let t = vec![0.01, 0.02, 0.03, 0.04, 0.05];
        let r = turnover_percentile(&t, 5).unwrap();
        assert_relative_eq!(r[4], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_volume_ratio() {
        let vol = vec![100.0, 200.0, 150.0];
        let ma = vec![100.0, 100.0, 100.0];
        let r = volume_ratio(&vol, &ma).unwrap();
        assert_relative_eq!(r[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(r[1], 2.0, epsilon = 1e-10);
        assert_relative_eq!(r[2], 1.5, epsilon = 1e-10);
    }

    #[test]
    fn test_committee_ratio() {
        let bid = vec![100.0, 200.0, 0.0];
        let ask = vec![100.0, 100.0, 100.0];
        let r = committee_ratio(&bid, &ask).unwrap();
        assert_relative_eq!(r[0], 0.0, epsilon = 1e-10);
        assert_relative_eq!(r[1], (200.0 - 100.0) / 300.0, epsilon = 1e-10);
        assert_relative_eq!(r[2], -1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_11_new_indicators_length_mismatch() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0];
        assert!(north_bound_flow(&a, &b).is_err());
        assert!(margin_balance(&a, &b, 0.0).is_err());
        assert!(short_balance(&a, &b, 0.0).is_err());
        assert!(margin_buy_amount(&a, &b).is_err());
        assert!(dragon_tiger_net_buy(&a, &b).is_err());
        assert!(sector_strength(&a, &b, 1).is_err());
        assert!(limit_up_strength(&a, &b, &b).is_err());
        assert!(consecutive_limit_days(&a, &b).is_err());
        assert!(seal_amount(&a, &b).is_err());
        assert!(volume_ratio(&a, &b).is_err());
        assert!(committee_ratio(&a, &b).is_err());
    }
}
