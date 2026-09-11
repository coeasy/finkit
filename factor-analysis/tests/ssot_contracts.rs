use finkit::performance::{evaluate_portfolio, evaluate_returns, PerformanceConfig};
use finkit::risk::{max_drawdown, sharpe_ratio, sortino_ratio};
use finkit_factor_analysis::risk_model::{effective_number_of_bets, hhi};

#[test]
fn performance_facade_matches_canonical_risk_metrics() {
    let returns = [0.01, -0.02, 0.03, 0.005, -0.01, 0.02];
    let config = PerformanceConfig::default();
    let evaluation = evaluate_returns(&returns, config);
    assert!((evaluation.risk.sharpe_ratio - sharpe_ratio(&returns, 0.0, 252)).abs() < 1e-12);
    assert!((evaluation.risk.sortino_ratio - sortino_ratio(&returns, 0.0, 252)).abs() < 1e-12);

    let mut equity = vec![1.0];
    for ret in returns {
        equity.push(equity.last().copied().unwrap() * (1.0 + ret));
    }
    assert!((evaluation.drawdown.max_drawdown - max_drawdown(&equity).0).abs() < 1e-12);
}

#[test]
fn research_concentration_facades_match_core_performance_owner() {
    let weights = [0.4, 0.3, -0.2, -0.1];
    let canonical = evaluate_portfolio(&weights);
    assert!((hhi(&weights) - canonical.hhi).abs() < 1e-12);
    assert!(
        (effective_number_of_bets(&weights) - canonical.effective_number_of_bets).abs() < 1e-12
    );
}
