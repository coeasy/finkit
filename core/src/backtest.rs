//! Lightweight Vectorized Backtest Engine (简化回测框架).
//!
//! A fast vectorized backtester for signal validation. **Not** a full
//! event-driven backtester (use backtrader / rqalpha / nautilus for that).
//!
//! # Workflow
//!
//! 1. Compute a signal array (e.g. RSI > 70 → -100, < 30 → 100).
//! 2. Map the signal to a position (long / flat / short).
//! 3. Apply transaction costs (commission + slippage) on position changes.
//! 4. Compute bar-to-bar returns, equity curve, and risk metrics.
//!
//! # Limitations
//!
//! - No partial fills, no order book, no tick-level simulation.
//! - All-in/all-out position sizing (no pyramiding).
//! - No leverage, no margin.
//! - No corporate actions (splits / dividends).
//!
//! # Example
//!
//! ```
//! use alpha_ta_core::backtest::{backtest, BacktestConfig};
//! use alpha_ta_core::patterns::Signal;
//! use ndarray::Array1;
//!
//! let close: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64) * 0.1).collect();
//! let signal: Array1<Signal> = Array1::from(vec![0; 50]).mapv(|_| 100);
//! let config = BacktestConfig { initial_cash: 100_000.0, commission: 0.001, slippage: 0.0005, allow_short: false };
//! let result = backtest(&close, &signal, &config).unwrap();
//! assert!(result.total_return > 0.0);
//! ```

use crate::error::{Result, TaError};
use crate::patterns::Signal;
use ndarray::Array1;

/// 持仓方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Position {
    Flat,
    Long,
    Short,
}

impl Position {
    fn from_signal(s: Signal, allow_short: bool) -> Self {
        if s > 0 {
            Position::Long
        } else if s < 0 {
            if allow_short { Position::Short } else { Position::Flat }
        } else {
            Position::Flat
        }
    }
}

/// 回测配置
#[derive(Debug, Clone)]
pub struct BacktestConfig {
    /// 初始资金
    pub initial_cash: f64,
    /// 手续费率（如 0.001 = 10 bps）
    pub commission: f64,
    /// 滑点（如 0.0005 = 5 bps）
    pub slippage: f64,
    /// 是否允许做空
    pub allow_short: bool,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            initial_cash: 100_000.0,
            commission: 0.001,
            slippage: 0.0,
            allow_short: false,
        }
    }
}

/// 单笔交易记录
#[derive(Debug, Clone, PartialEq)]
pub struct Trade {
    pub entry_idx: usize,
    pub exit_idx: usize,
    /// +1 for long, -1 for short
    pub direction: i32,
    pub entry_price: f64,
    pub exit_price: f64,
    pub pnl: f64,
    /// Return on a unit notional, including costs.
    pub return_pct: f64,
}

/// 回测结果
#[derive(Debug, Clone)]
pub struct BacktestResult {
    pub total_return: f64,
    pub annual_return: f64,
    pub sharpe: f64,
    pub sortino: f64,
    pub max_drawdown: f64,
    pub win_rate: f64,
    pub profit_loss_ratio: f64,
    pub n_trades: usize,
    pub equity_curve: Array1<f64>,
    pub trades: Vec<Trade>,
}

/// Vectorized backtest.
///
/// * `close` — closing prices (one per bar).
/// * `signal` — signal array (must be same length as `close`).
///   * `signal > 0` → go long at next bar's open
///   * `signal < 0` → go short (if `allow_short`) at next bar's open
///   * `signal == 0` → flat
/// * `config` — initial capital, costs, and short permission.
///
/// # Algorithm
///
/// For each bar `i`:
///   1. Compute the position based on `signal[i-1]` (next-bar execution).
///   2. Apply the per-bar return: `ret[i] = (close[i] - close[i-1]) / close[i-1]`.
///   3. Multiply by position (`Long` → +ret, `Short` → -ret, `Flat` → 0).
///   4. Deduct costs when position changes (commission + slippage on the
///      trade notional, applied to the bar's return).
///
/// # Returns
///
/// A [`BacktestResult`] with the equity curve, trade list, and summary
/// statistics.
pub fn backtest(
    close: &[f64],
    signal: &Array1<Signal>,
    config: &BacktestConfig,
) -> Result<BacktestResult> {
    let n = close.len();
    if signal.len() != n {
        return Err(TaError::InvalidParameter {
            name: "close, signal".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if n < 2 {
        return Err(TaError::InvalidParameter {
            name: "close".to_string(),
            constraint: "length must be >= 2".to_string(),
        });
    }

    // Position time series: 1 = long, -1 = short, 0 = flat
    let mut pos_arr = vec![0i32; n];
    pos_arr[0] = match Position::from_signal(signal[0], config.allow_short) {
        Position::Long => 1,
        Position::Short => -1,
        Position::Flat => 0,
    };
    for i in 1..n {
        pos_arr[i] = match Position::from_signal(signal[i - 1], config.allow_short) {
            Position::Long => 1,
            Position::Short => -1,
            Position::Flat => 0,
        };
    }

    // Returns: per-bar simple return of close
    let mut bar_returns = vec![0.0_f64; n];
    for i in 1..n {
        if close[i - 1] > 0.0 {
            bar_returns[i] = (close[i] - close[i - 1]) / close[i - 1];
        }
    }

    // Apply position to returns
    let mut strat_returns = vec![0.0_f64; n];
    let mut trades: Vec<Trade> = Vec::new();
    let mut current_pos = 0i32;
    let mut entry_idx = 0usize;
    let mut entry_price = 0.0_f64;
    let total_cost = config.commission + config.slippage;

    for i in 0..n {
        let prev_pos = current_pos;
        let new_pos = pos_arr[i];
        // Trade cost on position change
        if new_pos != prev_pos {
            // Close current position if any
            if prev_pos != 0 {
                let exit_price = close[i] * (1.0 - total_cost * prev_pos as f64);
                let pnl = prev_pos as f64 * (exit_price - entry_price);
                let return_pct = pnl / entry_price.max(1e-15);
                trades.push(Trade {
                    entry_idx,
                    exit_idx: i,
                    direction: prev_pos,
                    entry_price,
                    exit_price,
                    pnl,
                    return_pct,
                });
            }
            // Open new position
            if new_pos != 0 {
                entry_idx = i;
                entry_price = close[i] * (1.0 + total_cost * new_pos as f64);
            }
            current_pos = new_pos;
            strat_returns[i] -= total_cost;
        }
        // Strategy return for this bar
        if current_pos != 0 {
            strat_returns[i] += (current_pos as f64) * bar_returns[i];
        }
    }

    // Close any open position at the end
    if current_pos != 0 {
        let exit_idx = n - 1;
        let exit_price = close[exit_idx] * (1.0 - total_cost * current_pos as f64);
        let pnl = current_pos as f64 * (exit_price - entry_price);
        let return_pct = pnl / entry_price.max(1e-15);
        trades.push(Trade {
            entry_idx,
            exit_idx,
            direction: current_pos,
            entry_price,
            exit_price,
            pnl,
            return_pct,
        });
    }

    // Equity curve
    let mut equity = Array1::<f64>::zeros(n);
    equity[0] = config.initial_cash;
    for i in 1..n {
        equity[i] = equity[i - 1] * (1.0 + strat_returns[i]);
    }

    // Summary stats
    let total_return = (equity[n - 1] / config.initial_cash) - 1.0;
    // Annualized: assume 252 trading days, scale by N
    let annual_return = if n > 1 {
        (1.0 + total_return).powf(252.0 / (n - 1) as f64) - 1.0
    } else {
        0.0
    };

    // Sharpe: mean/std of strategy returns (per-bar), annualized
    let (sharpe, sortino, max_dd) = compute_risk_metrics(&strat_returns, &equity);

    let (win_rate, pl_ratio) = compute_trade_stats(&trades);

    Ok(BacktestResult {
        total_return,
        annual_return,
        sharpe,
        sortino,
        max_drawdown: max_dd,
        win_rate,
        profit_loss_ratio: pl_ratio,
        n_trades: trades.len(),
        equity_curve: equity,
        trades,
    })
}

fn compute_risk_metrics(returns: &[f64], equity: &Array1<f64>) -> (f64, f64, f64) {
    let n = returns.len();
    if n < 2 {
        return (0.0, 0.0, 0.0);
    }
    let mean = returns.iter().sum::<f64>() / n as f64;
    let var: f64 = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
    let std = var.sqrt();
    let downside: f64 = returns.iter()
        .filter(|&&r| r < 0.0)
        .map(|r| r.powi(2))
        .sum::<f64>() / n as f64;
    let dstd = downside.sqrt();
    let ann = (252_f64).sqrt();
    let sharpe = if std > 1e-15 { mean / std * ann } else { 0.0 };
    let sortino = if dstd > 1e-15 { mean / dstd * ann } else { 0.0 };

    // Max drawdown
    let mut peak = equity[0];
    let mut max_dd = 0.0_f64;
    for &v in equity.iter() {
        if v > peak { peak = v; }
        let dd = (peak - v) / peak;
        if dd > max_dd { max_dd = dd; }
    }

    (sharpe, sortino, max_dd)
}

fn compute_trade_stats(trades: &[Trade]) -> (f64, f64) {
    if trades.is_empty() {
        return (0.0, 0.0);
    }
    let wins: Vec<&Trade> = trades.iter().filter(|t| t.pnl > 0.0).collect();
    let losses: Vec<&Trade> = trades.iter().filter(|t| t.pnl < 0.0).collect();
    let win_rate = wins.len() as f64 / trades.len() as f64;
    let avg_win = if !wins.is_empty() {
        wins.iter().map(|t| t.pnl).sum::<f64>() / wins.len() as f64
    } else {
        0.0
    };
    let avg_loss = if !losses.is_empty() {
        losses.iter().map(|t| t.pnl.abs()).sum::<f64>() / losses.len() as f64
    } else {
        0.0
    };
    let pl_ratio = if avg_loss > 1e-15 { avg_win / avg_loss } else { 0.0 };
    (win_rate, pl_ratio)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_backtest_monotonic_up_long() {
        let n = 20;
        let close: Vec<f64> = (0..n).map(|i| 100.0 + i as f64).collect();
        let signal: Array1<Signal> = Array1::from(vec![100i32; n]);
        let config = BacktestConfig {
            initial_cash: 100_000.0,
            commission: 0.0,
            slippage: 0.0,
            allow_short: false,
        };
        let r = backtest(&close, &signal, &config).unwrap();
        // Monotonic uptrend with no costs: total return > 0
        assert!(r.total_return > 0.0, "got {}", r.total_return);
        // Exactly 1 trade (entered at i=0, closed at i=n-1)
        assert_eq!(r.n_trades, 1);
        assert!(r.trades[0].pnl > 0.0);
    }

    #[test]
    fn test_backtest_monotonic_down_short() {
        let n = 20;
        let close: Vec<f64> = (0..n).map(|i| 100.0 - i as f64 * 0.5).collect();
        let signal: Array1<Signal> = Array1::from(vec![-100i32; n]);
        let config = BacktestConfig {
            initial_cash: 100_000.0,
            commission: 0.0,
            slippage: 0.0,
            allow_short: true,
        };
        let r = backtest(&close, &signal, &config).unwrap();
        // Downtrend with short: positive return
        assert!(r.total_return > 0.0, "got {}", r.total_return);
    }

    #[test]
    fn test_backtest_short_disallowed() {
        let n = 10;
        let close: Vec<f64> = (0..n).map(|i| 100.0 - i as f64).collect();
        let signal: Array1<Signal> = Array1::from(vec![-100i32; n]);
        let config = BacktestConfig {
            initial_cash: 100_000.0,
            commission: 0.0,
            slippage: 0.0,
            allow_short: false,
        };
        let r = backtest(&close, &signal, &config).unwrap();
        // Short disabled → flat → no return
        assert_relative_eq!(r.total_return, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_backtest_commission_drag() {
        let n = 10;
        let close: Vec<f64> = (0..n).map(|i| 100.0 + i as f64).collect();
        // Alternate long/flat to generate multiple round trips
        let signal: Array1<Signal> = Array1::from((0..n).map(|i| if i % 2 == 0 { 100_i32 } else { 0_i32 }).collect::<Vec<i32>>());
        let config = BacktestConfig {
            initial_cash: 100_000.0,
            commission: 0.01, // 1% per trade
            slippage: 0.0,
            allow_short: false,
        };
        let r = backtest(&close, &signal, &config).unwrap();
        // Commission eats into return
        let zero_cost = BacktestConfig { commission: 0.0, ..config.clone() };
        let r0 = backtest(&close, &signal, &zero_cost).unwrap();
        assert!(r.total_return < r0.total_return, "commission should reduce return");
    }

    #[test]
    fn test_backtest_max_drawdown() {
        // Up then down — drawdown should be the depth of the down leg
        let close: Vec<f64> = vec![100.0, 110.0, 120.0, 110.0, 100.0, 90.0, 80.0];
        let signal: Array1<Signal> = Array1::from(vec![100i32; 7]);
        let config = BacktestConfig {
            initial_cash: 100_000.0,
            commission: 0.0,
            slippage: 0.0,
            allow_short: false,
        };
        let r = backtest(&close, &signal, &config).unwrap();
        // Max DD = (120-80)/120 = 0.333
        assert!(r.max_drawdown > 0.3 && r.max_drawdown < 0.4, "got {}", r.max_drawdown);
    }

    #[test]
    fn test_backtest_length_mismatch() {
        let close: Vec<f64> = vec![100.0, 101.0];
        let signal: Array1<Signal> = Array1::from(vec![100i32]);
        let config = BacktestConfig::default();
        assert!(backtest(&close, &signal, &config).is_err());
    }

    #[test]
    fn test_backtest_too_short() {
        let close: Vec<f64> = vec![100.0];
        let signal: Array1<Signal> = Array1::from(vec![100i32]);
        let config = BacktestConfig::default();
        assert!(backtest(&close, &signal, &config).is_err());
    }

    #[test]
    fn test_position_from_signal() {
        assert_eq!(Position::from_signal(100, true), Position::Long);
        assert_eq!(Position::from_signal(-100, true), Position::Short);
        assert_eq!(Position::from_signal(0, true), Position::Flat);
        // No short: -100 → Flat
        assert_eq!(Position::from_signal(-100, false), Position::Flat);
    }
}
