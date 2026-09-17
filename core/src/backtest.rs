//! Lightweight vectorized backtest engine for fast signal validation.
//!
//! This remains intentionally separate from the multi-asset factor holding
//! engine in `finkit-factor-analysis`. Return and risk math is delegated to
//! the canonical core SSOT modules.

use crate::error::{Result, TaError};
use crate::patterns::Signal;
use crate::returns::{one_period_returns, ReturnKind};
use crate::risk::{max_drawdown, sharpe_ratio, sortino_ratio};
use ndarray::Array1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Position {
    Flat,
    Long,
    Short,
}

impl Position {
    fn from_signal(s: Signal, allow_short: bool) -> Self {
        if s > 0 {
            Self::Long
        } else if s < 0 && allow_short {
            Self::Short
        } else {
            Self::Flat
        }
    }
}

#[derive(Debug, Clone)]
pub struct BacktestConfig {
    pub initial_cash: f64,
    pub commission: f64,
    pub slippage: f64,
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

#[derive(Debug, Clone, PartialEq)]
pub struct Trade {
    pub entry_idx: usize,
    pub exit_idx: usize,
    pub direction: i32,
    pub entry_price: f64,
    pub exit_price: f64,
    pub pnl: f64,
    pub return_pct: f64,
}

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

    let mut bar_returns = one_period_returns(close, ReturnKind::Arithmetic);
    if !bar_returns.is_empty() {
        bar_returns[0] = 0.0;
    }

    let mut strat_returns = vec![0.0_f64; n];
    let mut trades = Vec::new();
    let mut current_pos = 0i32;
    let mut entry_idx = 0usize;
    let mut entry_price = 0.0_f64;
    let total_cost = config.commission + config.slippage;

    for i in 0..n {
        let prev_pos = current_pos;
        let new_pos = pos_arr[i];
        if new_pos != prev_pos {
            if prev_pos != 0 {
                let exit_price = close[i] * (1.0 - total_cost * prev_pos as f64);
                let pnl = prev_pos as f64 * (exit_price - entry_price);
                trades.push(Trade {
                    entry_idx,
                    exit_idx: i,
                    direction: prev_pos,
                    entry_price,
                    exit_price,
                    pnl,
                    return_pct: pnl / entry_price.max(1e-15),
                });
            }
            if new_pos != 0 {
                entry_idx = i;
                entry_price = close[i] * (1.0 + total_cost * new_pos as f64);
            }
            current_pos = new_pos;
            strat_returns[i] -= total_cost;
        }
        if current_pos != 0 {
            strat_returns[i] += current_pos as f64 * bar_returns[i];
        }
    }

    if current_pos != 0 {
        let exit_idx = n - 1;
        let exit_price = close[exit_idx] * (1.0 - total_cost * current_pos as f64);
        let pnl = current_pos as f64 * (exit_price - entry_price);
        trades.push(Trade {
            entry_idx,
            exit_idx,
            direction: current_pos,
            entry_price,
            exit_price,
            pnl,
            return_pct: pnl / entry_price.max(1e-15),
        });
    }

    let mut equity = Array1::<f64>::zeros(n);
    equity[0] = config.initial_cash;
    for i in 1..n {
        equity[i] = equity[i - 1] * (1.0 + strat_returns[i]);
    }

    let total_return = equity[n - 1] / config.initial_cash - 1.0;
    let annual_return = (1.0 + total_return).powf(252.0 / (n - 1) as f64) - 1.0;
    let sharpe = sharpe_ratio(&strat_returns, 0.0, 252);
    let sortino = sortino_ratio(&strat_returns, 0.0, 252);
    let equity_slice = equity.as_slice().expect("owned ndarray is contiguous");
    let (max_dd, _, _) = max_drawdown(equity_slice);
    let (win_rate, profit_loss_ratio) = compute_trade_stats(&trades);

    Ok(BacktestResult {
        total_return,
        annual_return,
        sharpe,
        sortino,
        max_drawdown: max_dd,
        win_rate,
        profit_loss_ratio,
        n_trades: trades.len(),
        equity_curve: equity,
        trades,
    })
}

fn compute_trade_stats(trades: &[Trade]) -> (f64, f64) {
    if trades.is_empty() {
        return (0.0, 0.0);
    }
    let wins: Vec<&Trade> = trades.iter().filter(|trade| trade.pnl > 0.0).collect();
    let losses: Vec<&Trade> = trades.iter().filter(|trade| trade.pnl < 0.0).collect();
    let win_rate = wins.len() as f64 / trades.len() as f64;
    let avg_win = if wins.is_empty() {
        0.0
    } else {
        wins.iter().map(|trade| trade.pnl).sum::<f64>() / wins.len() as f64
    };
    let avg_loss = if losses.is_empty() {
        0.0
    } else {
        losses.iter().map(|trade| trade.pnl.abs()).sum::<f64>() / losses.len() as f64
    };
    (
        win_rate,
        if avg_loss > 1e-15 {
            avg_win / avg_loss
        } else {
            0.0
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_backtest_monotonic_up_long() {
        let n = 20;
        let close: Vec<f64> = (0..n).map(|i| 100.0 + i as f64).collect();
        let signal = Array1::from(vec![100i32; n]);
        let r = backtest(
            &close,
            &signal,
            &BacktestConfig {
                initial_cash: 100_000.0,
                commission: 0.0,
                slippage: 0.0,
                allow_short: false,
            },
        )
        .unwrap();
        assert!(r.total_return > 0.0);
        assert_eq!(r.n_trades, 1);
    }

    #[test]
    fn test_backtest_monotonic_down_short() {
        let n = 20;
        let close: Vec<f64> = (0..n).map(|i| 100.0 - i as f64 * 0.5).collect();
        let signal = Array1::from(vec![-100i32; n]);
        let r = backtest(
            &close,
            &signal,
            &BacktestConfig {
                initial_cash: 100_000.0,
                commission: 0.0,
                slippage: 0.0,
                allow_short: true,
            },
        )
        .unwrap();
        assert!(r.total_return > 0.0);
    }

    #[test]
    fn test_backtest_short_disallowed() {
        let n = 10;
        let close: Vec<f64> = (0..n).map(|i| 100.0 - i as f64).collect();
        let signal = Array1::from(vec![-100i32; n]);
        let r = backtest(
            &close,
            &signal,
            &BacktestConfig {
                initial_cash: 100_000.0,
                commission: 0.0,
                slippage: 0.0,
                allow_short: false,
            },
        )
        .unwrap();
        assert_relative_eq!(r.total_return, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_backtest_commission_drag() {
        let n = 10;
        let close: Vec<f64> = (0..n).map(|i| 100.0 + i as f64).collect();
        let signal = Array1::from(
            (0..n)
                .map(|i| if i % 2 == 0 { 100_i32 } else { 0_i32 })
                .collect::<Vec<_>>(),
        );
        let config = BacktestConfig {
            initial_cash: 100_000.0,
            commission: 0.01,
            slippage: 0.0,
            allow_short: false,
        };
        let r = backtest(&close, &signal, &config).unwrap();
        let r0 = backtest(
            &close,
            &signal,
            &BacktestConfig {
                commission: 0.0,
                ..config.clone()
            },
        )
        .unwrap();
        assert!(r.total_return < r0.total_return);
    }

    #[test]
    fn test_backtest_max_drawdown() {
        let close = vec![100.0, 110.0, 120.0, 110.0, 100.0, 90.0, 80.0];
        let signal = Array1::from(vec![100i32; 7]);
        let r = backtest(
            &close,
            &signal,
            &BacktestConfig {
                initial_cash: 100_000.0,
                commission: 0.0,
                slippage: 0.0,
                allow_short: false,
            },
        )
        .unwrap();
        assert!(r.max_drawdown > 0.3 && r.max_drawdown < 0.4);
    }

    #[test]
    fn validation_errors_are_preserved() {
        assert!(backtest(
            &[100.0, 101.0],
            &Array1::from(vec![100i32]),
            &BacktestConfig::default()
        )
        .is_err());
        assert!(backtest(
            &[100.0],
            &Array1::from(vec![100i32]),
            &BacktestConfig::default()
        )
        .is_err());
    }
}
