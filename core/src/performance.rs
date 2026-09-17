//! Canonical quantitative performance evaluation primitives.
//!
//! This module owns reusable strategy/portfolio evaluation math. Backtests,
//! factor research, portfolio studies and language bindings should delegate to
//! these functions instead of reimplementing performance metrics.

use crate::math::statistics::{correlation, covariance, std_dev};
use crate::risk::{cvar, sharpe_ratio, sortino_ratio, var_historical, var_parametric};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerformanceConfig {
    /// Number of observations in one year (252 for daily data).
    pub annualization: usize,
    /// Risk-free return per observation.
    pub risk_free_rate: f64,
    /// Minimum acceptable return per observation used by downside/Omega ratios.
    pub minimum_acceptable_return: f64,
    /// Confidence level used by VaR/CVaR metrics.
    pub var_confidence: f64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            annualization: 252,
            risk_free_rate: 0.0,
            minimum_acceptable_return: 0.0,
            var_confidence: 0.95,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ReturnMetrics {
    pub observations: usize,
    pub total_return: f64,
    pub annualized_return: f64,
    pub arithmetic_mean_return: f64,
    pub geometric_mean_return: f64,
    pub best_period_return: f64,
    pub worst_period_return: f64,
    pub positive_period_ratio: f64,
    pub negative_period_ratio: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DrawdownMetrics {
    pub max_drawdown: f64,
    pub max_drawdown_peak_index: usize,
    pub max_drawdown_trough_index: usize,
    pub current_drawdown: f64,
    pub average_drawdown: f64,
    pub max_drawdown_duration: usize,
    pub average_drawdown_duration: f64,
    pub recovery_duration: Option<usize>,
    pub ulcer_index: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RiskMetrics {
    pub annualized_volatility: f64,
    pub downside_volatility: f64,
    pub upside_volatility: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub calmar_ratio: f64,
    pub omega_ratio: f64,
    pub sterling_ratio: f64,
    pub burke_ratio: f64,
    pub historical_var: f64,
    pub parametric_var: f64,
    pub cvar: f64,
    pub tail_ratio: f64,
    pub skewness: f64,
    pub excess_kurtosis: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BenchmarkMetrics {
    pub active_annualized_return: f64,
    pub tracking_error: f64,
    pub information_ratio: f64,
    pub alpha: f64,
    pub beta: f64,
    pub r_squared: f64,
    pub correlation: f64,
    pub upside_capture: f64,
    pub downside_capture: f64,
    pub treynor_ratio: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TradeMetrics {
    pub trades: usize,
    pub wins: usize,
    pub losses: usize,
    pub breakeven: usize,
    pub win_rate: f64,
    pub loss_rate: f64,
    pub profit_factor: f64,
    pub expectancy: f64,
    pub average_win: f64,
    pub average_loss: f64,
    pub payoff_ratio: f64,
    pub best_trade_return: f64,
    pub worst_trade_return: f64,
    pub max_consecutive_wins: usize,
    pub max_consecutive_losses: usize,
    pub average_holding_period: f64,
    pub max_holding_period: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PortfolioMetrics {
    pub gross_exposure: f64,
    pub net_exposure: f64,
    pub long_exposure: f64,
    pub short_exposure: f64,
    pub leverage: f64,
    pub max_abs_weight: f64,
    pub hhi: f64,
    pub effective_number_of_bets: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PerformanceEvaluation {
    pub returns: ReturnMetrics,
    pub risk: RiskMetrics,
    pub drawdown: DrawdownMetrics,
}

#[inline]
fn finite_values(values: &[f64]) -> Vec<f64> {
    values.iter().copied().filter(|v| v.is_finite()).collect()
}

#[inline]
fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn sample_std(values: &[f64]) -> f64 {
    if values.len() < 2 {
        0.0
    } else {
        std_dev(values).unwrap_or(0.0)
    }
}

/// Compound per-period returns into an equity curve starting at 1.0.
pub fn equity_curve_from_returns(returns: &[f64]) -> Vec<f64> {
    let mut equity = Vec::with_capacity(returns.len() + 1);
    let mut wealth = 1.0_f64;
    equity.push(wealth);
    for &ret in returns {
        if ret.is_finite() {
            wealth *= 1.0 + ret;
        }
        equity.push(wealth);
    }
    equity
}

/// Compound total return from a sequence of per-period returns.
pub fn total_return(returns: &[f64]) -> f64 {
    finite_values(returns)
        .iter()
        .fold(1.0, |wealth, ret| wealth * (1.0 + ret))
        - 1.0
}

/// CAGR/annualized compounded return.
pub fn annualized_return(returns: &[f64], annualization: usize) -> f64 {
    let finite = finite_values(returns);
    if finite.is_empty() || annualization == 0 {
        return 0.0;
    }
    let growth = finite.iter().fold(1.0, |wealth, ret| wealth * (1.0 + ret));
    if growth <= 0.0 {
        return -1.0;
    }
    growth.powf(annualization as f64 / finite.len() as f64) - 1.0
}

/// Annualized sample volatility.
pub fn annualized_volatility(returns: &[f64], annualization: usize) -> f64 {
    let finite = finite_values(returns);
    sample_std(&finite) * (annualization as f64).sqrt()
}

/// Annualized downside deviation relative to a per-period MAR.
pub fn downside_volatility(returns: &[f64], mar: f64, annualization: usize) -> f64 {
    let finite = finite_values(returns);
    if finite.is_empty() {
        return 0.0;
    }
    let variance = finite
        .iter()
        .map(|ret| (ret - mar).min(0.0).powi(2))
        .sum::<f64>()
        / finite.len() as f64;
    variance.sqrt() * (annualization as f64).sqrt()
}

/// Annualized upside deviation relative to a per-period MAR.
pub fn upside_volatility(returns: &[f64], mar: f64, annualization: usize) -> f64 {
    let finite = finite_values(returns);
    if finite.is_empty() {
        return 0.0;
    }
    let variance = finite
        .iter()
        .map(|ret| (ret - mar).max(0.0).powi(2))
        .sum::<f64>()
        / finite.len() as f64;
    variance.sqrt() * (annualization as f64).sqrt()
}

/// Omega ratio relative to a per-period threshold.
pub fn omega_ratio(returns: &[f64], threshold: f64) -> f64 {
    let mut gains = 0.0;
    let mut losses = 0.0;
    for &ret in returns.iter().filter(|v| v.is_finite()) {
        let excess = ret - threshold;
        if excess >= 0.0 {
            gains += excess;
        } else {
            losses -= excess;
        }
    }
    if losses <= f64::EPSILON {
        if gains > 0.0 {
            f64::INFINITY
        } else {
            0.0
        }
    } else {
        gains / losses
    }
}

fn linear_quantile(sorted: &[f64], probability: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let p = probability.clamp(0.0, 1.0);
    let pos = p * (sorted.len() - 1) as f64;
    let lower = pos.floor() as usize;
    let upper = pos.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let weight = pos - lower as f64;
        sorted[lower] * (1.0 - weight) + sorted[upper] * weight
    }
}

/// Ratio of upper-tail magnitude to lower-tail magnitude (95% / |5%|).
pub fn tail_ratio(returns: &[f64]) -> f64 {
    let mut finite = finite_values(returns);
    if finite.is_empty() {
        return 0.0;
    }
    finite.sort_by(f64::total_cmp);
    let upper = linear_quantile(&finite, 0.95);
    let lower = linear_quantile(&finite, 0.05).abs();
    if lower <= f64::EPSILON {
        if upper > 0.0 {
            f64::INFINITY
        } else {
            0.0
        }
    } else {
        upper / lower
    }
}

pub fn skewness(returns: &[f64]) -> f64 {
    let finite = finite_values(returns);
    if finite.len() < 3 {
        return 0.0;
    }
    let m = mean(&finite);
    let m2 = finite.iter().map(|v| (v - m).powi(2)).sum::<f64>() / finite.len() as f64;
    if m2 <= f64::EPSILON {
        return 0.0;
    }
    let m3 = finite.iter().map(|v| (v - m).powi(3)).sum::<f64>() / finite.len() as f64;
    m3 / m2.powf(1.5)
}

pub fn excess_kurtosis(returns: &[f64]) -> f64 {
    let finite = finite_values(returns);
    if finite.len() < 4 {
        return 0.0;
    }
    let m = mean(&finite);
    let m2 = finite.iter().map(|v| (v - m).powi(2)).sum::<f64>() / finite.len() as f64;
    if m2 <= f64::EPSILON {
        return 0.0;
    }
    let m4 = finite.iter().map(|v| (v - m).powi(4)).sum::<f64>() / finite.len() as f64;
    m4 / (m2 * m2) - 3.0
}

/// Full drawdown summary from an equity curve.
pub fn evaluate_drawdowns(equity: &[f64]) -> DrawdownMetrics {
    let finite: Vec<f64> = equity.iter().copied().filter(|v| v.is_finite()).collect();
    if finite.is_empty() {
        return DrawdownMetrics::default();
    }

    let mut peak = finite[0];
    let mut peak_idx = 0usize;
    let mut max_drawdown = 0.0_f64;
    let mut max_peak = 0usize;
    let mut max_trough = 0usize;
    let mut drawdown_sum = 0.0_f64;
    let mut underwater_observations = 0usize;
    let mut squared_sum = 0.0_f64;

    let mut episode_start: Option<usize> = None;
    let mut episode_durations = Vec::new();

    for (i, &value) in finite.iter().enumerate() {
        if value >= peak {
            if let Some(start) = episode_start.take() {
                episode_durations.push(i.saturating_sub(start));
            }
            peak = value;
            peak_idx = i;
        }
        let dd = if peak.abs() > f64::EPSILON {
            ((peak - value) / peak).max(0.0)
        } else {
            0.0
        };
        squared_sum += dd * dd;
        if dd > 0.0 {
            if episode_start.is_none() {
                episode_start = Some(i);
            }
            drawdown_sum += dd;
            underwater_observations += 1;
        }
        if dd > max_drawdown {
            max_drawdown = dd;
            max_peak = peak_idx;
            max_trough = i;
        }
    }
    if let Some(start) = episode_start {
        episode_durations.push(finite.len().saturating_sub(start));
    }

    let current_peak = finite.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let current_drawdown = if current_peak.is_finite() && current_peak.abs() > f64::EPSILON {
        ((current_peak - finite[finite.len() - 1]) / current_peak).max(0.0)
    } else {
        0.0
    };

    let recovery_duration = if max_drawdown > 0.0 {
        let target = finite[max_peak];
        finite
            .iter()
            .enumerate()
            .skip(max_trough + 1)
            .find(|(_, value)| **value >= target)
            .map(|(idx, _)| idx - max_trough)
    } else {
        Some(0)
    };

    DrawdownMetrics {
        max_drawdown,
        max_drawdown_peak_index: max_peak,
        max_drawdown_trough_index: max_trough,
        current_drawdown,
        average_drawdown: if underwater_observations == 0 {
            0.0
        } else {
            drawdown_sum / underwater_observations as f64
        },
        max_drawdown_duration: episode_durations.iter().copied().max().unwrap_or(0),
        average_drawdown_duration: if episode_durations.is_empty() {
            0.0
        } else {
            episode_durations.iter().sum::<usize>() as f64 / episode_durations.len() as f64
        },
        recovery_duration,
        ulcer_index: (squared_sum / finite.len() as f64).sqrt(),
    }
}

fn episode_depths(equity: &[f64]) -> Vec<f64> {
    if equity.is_empty() {
        return Vec::new();
    }
    let mut peak = equity[0];
    let mut current_max = 0.0_f64;
    let mut in_drawdown = false;
    let mut depths = Vec::new();
    for &value in equity {
        if !value.is_finite() {
            continue;
        }
        if value >= peak {
            if in_drawdown {
                depths.push(current_max);
                current_max = 0.0;
                in_drawdown = false;
            }
            peak = value;
            continue;
        }
        if peak.abs() > f64::EPSILON {
            current_max = current_max.max((peak - value) / peak);
            in_drawdown = true;
        }
    }
    if in_drawdown {
        depths.push(current_max);
    }
    depths
}

pub fn evaluate_returns(returns: &[f64], config: PerformanceConfig) -> PerformanceEvaluation {
    let finite = finite_values(returns);
    if finite.is_empty() {
        return PerformanceEvaluation::default();
    }
    let annualization = config.annualization.max(1);
    let total = total_return(&finite);
    let annual = annualized_return(&finite, annualization);
    let arithmetic = mean(&finite);
    let growth = 1.0 + total;
    let geometric = if growth > 0.0 {
        growth.powf(1.0 / finite.len() as f64) - 1.0
    } else {
        -1.0
    };
    let positive = finite.iter().filter(|&&v| v > 0.0).count();
    let negative = finite.iter().filter(|&&v| v < 0.0).count();
    let equity = equity_curve_from_returns(&finite);
    let drawdown = evaluate_drawdowns(&equity);
    let depths = episode_depths(&equity);
    let average_episode_drawdown = if depths.is_empty() {
        0.0
    } else {
        depths.iter().sum::<f64>() / depths.len() as f64
    };
    let burke_denominator = depths.iter().map(|v| v * v).sum::<f64>().sqrt();

    PerformanceEvaluation {
        returns: ReturnMetrics {
            observations: finite.len(),
            total_return: total,
            annualized_return: annual,
            arithmetic_mean_return: arithmetic,
            geometric_mean_return: geometric,
            best_period_return: finite.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            worst_period_return: finite.iter().copied().fold(f64::INFINITY, f64::min),
            positive_period_ratio: positive as f64 / finite.len() as f64,
            negative_period_ratio: negative as f64 / finite.len() as f64,
        },
        risk: RiskMetrics {
            annualized_volatility: annualized_volatility(&finite, annualization),
            downside_volatility: downside_volatility(
                &finite,
                config.minimum_acceptable_return,
                annualization,
            ),
            upside_volatility: upside_volatility(
                &finite,
                config.minimum_acceptable_return,
                annualization,
            ),
            sharpe_ratio: sharpe_ratio(&finite, config.risk_free_rate, annualization),
            sortino_ratio: sortino_ratio(&finite, config.risk_free_rate, annualization),
            calmar_ratio: if drawdown.max_drawdown > f64::EPSILON {
                annual / drawdown.max_drawdown
            } else {
                0.0
            },
            omega_ratio: omega_ratio(&finite, config.minimum_acceptable_return),
            sterling_ratio: if average_episode_drawdown > f64::EPSILON {
                annual / average_episode_drawdown
            } else {
                0.0
            },
            burke_ratio: if burke_denominator > f64::EPSILON {
                annual / burke_denominator
            } else {
                0.0
            },
            historical_var: var_historical(&finite, config.var_confidence),
            parametric_var: var_parametric(&finite, config.var_confidence),
            cvar: cvar(&finite, config.var_confidence),
            tail_ratio: tail_ratio(&finite),
            skewness: skewness(&finite),
            excess_kurtosis: excess_kurtosis(&finite),
        },
        drawdown,
    }
}

/// Benchmark-relative metrics using pairwise finite observations only.
pub fn evaluate_benchmark(
    strategy_returns: &[f64],
    benchmark_returns: &[f64],
    config: PerformanceConfig,
) -> BenchmarkMetrics {
    let pairs: Vec<(f64, f64)> = strategy_returns
        .iter()
        .copied()
        .zip(benchmark_returns.iter().copied())
        .filter(|(strategy, benchmark)| strategy.is_finite() && benchmark.is_finite())
        .collect();
    if pairs.len() < 2 {
        return BenchmarkMetrics::default();
    }
    let strategy: Vec<f64> = pairs.iter().map(|v| v.0).collect();
    let benchmark: Vec<f64> = pairs.iter().map(|v| v.1).collect();
    let active: Vec<f64> = pairs.iter().map(|v| v.0 - v.1).collect();
    let annualization = config.annualization.max(1);
    let tracking_error = sample_std(&active) * (annualization as f64).sqrt();
    let information_ratio = if tracking_error > f64::EPSILON {
        mean(&active) * annualization as f64 / tracking_error
    } else {
        0.0
    };
    let benchmark_variance = sample_std(&benchmark).powi(2);
    let beta = if benchmark_variance > f64::EPSILON {
        covariance(&strategy, &benchmark).unwrap_or(0.0) / benchmark_variance
    } else {
        0.0
    };
    let strategy_mean = mean(&strategy);
    let benchmark_mean = mean(&benchmark);
    let alpha_per_period =
        (strategy_mean - config.risk_free_rate) - beta * (benchmark_mean - config.risk_free_rate);
    let corr = correlation(&strategy, &benchmark).unwrap_or(0.0);

    let upside: Vec<(f64, f64)> = pairs.iter().copied().filter(|v| v.1 > 0.0).collect();
    let downside: Vec<(f64, f64)> = pairs.iter().copied().filter(|v| v.1 < 0.0).collect();
    let capture = |samples: &[(f64, f64)]| -> f64 {
        if samples.is_empty() {
            return 0.0;
        }
        let s = samples.iter().map(|v| v.0).sum::<f64>() / samples.len() as f64;
        let b = samples.iter().map(|v| v.1).sum::<f64>() / samples.len() as f64;
        if b.abs() > f64::EPSILON {
            s / b
        } else {
            0.0
        }
    };
    let excess_annual = annualized_return(&strategy, annualization)
        - ((1.0 + config.risk_free_rate).powi(annualization as i32) - 1.0);

    BenchmarkMetrics {
        active_annualized_return: annualized_return(&strategy, annualization)
            - annualized_return(&benchmark, annualization),
        tracking_error,
        information_ratio,
        alpha: alpha_per_period * annualization as f64,
        beta,
        r_squared: corr * corr,
        correlation: corr,
        upside_capture: capture(&upside),
        downside_capture: capture(&downside),
        treynor_ratio: if beta.abs() > f64::EPSILON {
            excess_annual / beta
        } else {
            0.0
        },
    }
}

/// Trade-level metrics from per-trade returns and optional holding periods.
pub fn evaluate_trades(trade_returns: &[f64], holding_periods: Option<&[usize]>) -> TradeMetrics {
    let finite: Vec<f64> = trade_returns
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .collect();
    if finite.is_empty() {
        return TradeMetrics::default();
    }
    let wins: Vec<f64> = finite.iter().copied().filter(|v| *v > 0.0).collect();
    let losses: Vec<f64> = finite.iter().copied().filter(|v| *v < 0.0).collect();
    let breakeven = finite.len() - wins.len() - losses.len();
    let gross_profit = wins.iter().sum::<f64>();
    let gross_loss = losses.iter().map(|v| v.abs()).sum::<f64>();
    let average_win = mean(&wins);
    let average_loss = if losses.is_empty() {
        0.0
    } else {
        losses.iter().map(|v| v.abs()).sum::<f64>() / losses.len() as f64
    };

    let mut max_wins = 0usize;
    let mut max_losses = 0usize;
    let mut current_wins = 0usize;
    let mut current_losses = 0usize;
    for &ret in &finite {
        if ret > 0.0 {
            current_wins += 1;
            current_losses = 0;
            max_wins = max_wins.max(current_wins);
        } else if ret < 0.0 {
            current_losses += 1;
            current_wins = 0;
            max_losses = max_losses.max(current_losses);
        } else {
            current_wins = 0;
            current_losses = 0;
        }
    }

    let valid_holding: Vec<usize> = holding_periods
        .filter(|periods| periods.len() == trade_returns.len())
        .map(|periods| {
            trade_returns
                .iter()
                .zip(periods.iter().copied())
                .filter_map(|(ret, period)| ret.is_finite().then_some(period))
                .collect()
        })
        .unwrap_or_default();

    TradeMetrics {
        trades: finite.len(),
        wins: wins.len(),
        losses: losses.len(),
        breakeven,
        win_rate: wins.len() as f64 / finite.len() as f64,
        loss_rate: losses.len() as f64 / finite.len() as f64,
        profit_factor: if gross_loss > f64::EPSILON {
            gross_profit / gross_loss
        } else if gross_profit > 0.0 {
            f64::INFINITY
        } else {
            0.0
        },
        expectancy: mean(&finite),
        average_win,
        average_loss,
        payoff_ratio: if average_loss > f64::EPSILON {
            average_win / average_loss
        } else {
            0.0
        },
        best_trade_return: finite.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        worst_trade_return: finite.iter().copied().fold(f64::INFINITY, f64::min),
        max_consecutive_wins: max_wins,
        max_consecutive_losses: max_losses,
        average_holding_period: if valid_holding.is_empty() {
            0.0
        } else {
            valid_holding.iter().sum::<usize>() as f64 / valid_holding.len() as f64
        },
        max_holding_period: valid_holding.iter().copied().max().unwrap_or(0),
    }
}

/// Portfolio exposure/concentration metrics from one cross-sectional weight vector.
pub fn evaluate_portfolio(weights: &[f64]) -> PortfolioMetrics {
    let finite: Vec<f64> = weights.iter().copied().filter(|v| v.is_finite()).collect();
    if finite.is_empty() {
        return PortfolioMetrics::default();
    }
    let long = finite.iter().copied().filter(|v| *v > 0.0).sum::<f64>();
    let short = finite
        .iter()
        .copied()
        .filter(|v| *v < 0.0)
        .map(f64::abs)
        .sum::<f64>();
    let gross = finite.iter().map(|v| v.abs()).sum::<f64>();
    let net = finite.iter().sum::<f64>();
    let hhi = if gross > f64::EPSILON {
        finite.iter().map(|v| (v.abs() / gross).powi(2)).sum()
    } else {
        0.0
    };
    PortfolioMetrics {
        gross_exposure: gross,
        net_exposure: net,
        long_exposure: long,
        short_exposure: short,
        leverage: gross,
        max_abs_weight: finite.iter().map(|v| v.abs()).fold(0.0, f64::max),
        hhi,
        effective_number_of_bets: if hhi > f64::EPSILON { 1.0 / hhi } else { 0.0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation_covers_return_risk_and_drawdown() {
        let eval = evaluate_returns(&[0.10, -0.05, 0.02, 0.03], PerformanceConfig::default());
        assert!(eval.returns.total_return > 0.0);
        assert_eq!(eval.returns.best_period_return, 0.10);
        assert_eq!(eval.returns.worst_period_return, -0.05);
        assert!(eval.drawdown.max_drawdown > 0.0);
        assert!(eval.risk.annualized_volatility > 0.0);
    }

    #[test]
    fn benchmark_metrics_detect_identical_series() {
        let data = [0.01, -0.02, 0.03, 0.01];
        let metrics = evaluate_benchmark(&data, &data, PerformanceConfig::default());
        assert!((metrics.beta - 1.0).abs() < 1e-12);
        assert!((metrics.correlation - 1.0).abs() < 1e-12);
        assert!(metrics.tracking_error.abs() < 1e-12);
    }

    #[test]
    fn trade_metrics_capture_extremes_and_streaks() {
        let metrics = evaluate_trades(
            &[0.1, 0.2, -0.05, -0.1, -0.02, 0.04],
            Some(&[1, 2, 3, 4, 5, 6]),
        );
        assert_eq!(metrics.trades, 6);
        assert_eq!(metrics.max_consecutive_losses, 3);
        assert_eq!(metrics.best_trade_return, 0.2);
        assert_eq!(metrics.max_holding_period, 6);
    }

    #[test]
    fn trade_holding_periods_follow_finite_trade_returns() {
        let metrics = evaluate_trades(&[0.10, f64::NAN, -0.05], Some(&[1, 100, 3]));
        assert_eq!(metrics.trades, 2);
        assert!((metrics.average_holding_period - 2.0).abs() < 1e-12);
        assert_eq!(metrics.max_holding_period, 3);
    }

    #[test]
    fn portfolio_metrics_have_consistent_concentration() {
        let metrics = evaluate_portfolio(&[0.25, 0.25, -0.25, -0.25]);
        assert!((metrics.gross_exposure - 1.0).abs() < 1e-12);
        assert!(metrics.net_exposure.abs() < 1e-12);
        assert!((metrics.hhi - 0.25).abs() < 1e-12);
        assert!((metrics.effective_number_of_bets - 4.0).abs() < 1e-12);
    }
}
