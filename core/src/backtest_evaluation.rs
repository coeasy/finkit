//! Complete quantitative evaluation for the lightweight backtest engine.
//!
//! Kept separate from `backtest.rs` so the existing `BacktestResult` API stays
//! source-compatible while users can opt into the richer canonical metrics.

use crate::backtest::BacktestResult;
use crate::performance::{
    evaluate_returns, evaluate_trades, PerformanceConfig, PerformanceEvaluation, TradeMetrics,
};
use crate::returns::{one_period_returns, ReturnKind};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BacktestEvaluation {
    pub performance: PerformanceEvaluation,
    pub trades: TradeMetrics,
}

/// Evaluate a completed backtest with the same performance kernels used by
/// factor research and portfolio studies.
pub fn evaluate_backtest(
    result: &BacktestResult,
    config: PerformanceConfig,
) -> BacktestEvaluation {
    let equity = result.equity_curve.as_slice().unwrap_or(&[]);
    let mut returns = one_period_returns(equity, ReturnKind::Arithmetic);
    // The legacy backtest defines the pre-first-bar strategy return as zero.
    // Preserve that alignment so old and rich metrics have an exact SSOT contract.
    if let Some(first) = returns.first_mut() {
        *first = 0.0;
    }
    let trade_returns: Vec<f64> = result.trades.iter().map(|trade| trade.return_pct).collect();
    let holding_periods: Vec<usize> = result
        .trades
        .iter()
        .map(|trade| trade.exit_idx.saturating_sub(trade.entry_idx))
        .collect();
    BacktestEvaluation {
        performance: evaluate_returns(&returns, config),
        trades: evaluate_trades(&trade_returns, Some(&holding_periods)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backtest::{backtest, BacktestConfig};
    use crate::patterns::Signal;
    use ndarray::Array1;

    #[test]
    fn rich_backtest_evaluation_reuses_canonical_metrics() {
        let close = [100.0, 102.0, 101.0, 105.0, 108.0];
        let signal = Array1::from(vec![100 as Signal; close.len()]);
        let result = backtest(
            &close,
            &signal,
            &BacktestConfig {
                commission: 0.0,
                slippage: 0.0,
                ..BacktestConfig::default()
            },
        )
        .unwrap();
        let evaluation = evaluate_backtest(&result, PerformanceConfig::default());
        assert!(evaluation.performance.returns.total_return > 0.0);
        assert_eq!(evaluation.trades.trades, result.n_trades);
        assert!(evaluation.performance.drawdown.max_drawdown >= 0.0);
        assert!((evaluation.performance.returns.total_return - result.total_return).abs() < 1e-12);
        assert!((evaluation.performance.risk.sharpe_ratio - result.sharpe).abs() < 1e-12);
        assert!((evaluation.performance.risk.sortino_ratio - result.sortino).abs() < 1e-12);
        assert!((evaluation.performance.drawdown.max_drawdown - result.max_drawdown).abs() < 1e-12);
    }
}