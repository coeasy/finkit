# Quantitative Evaluation

Finkit provides a single-source-of-truth quantitative evaluation stack shared by strategy backtests, factor research, portfolio studies, and every supported language binding.

The design rule is:

```text
core::returns + core::risk + core::math
                 ↓
          core::performance
                 ↓
     factor-analysis adapters
                 ↓
Rust / Python / Node / Java / Android / Go / .NET / C / C++ / iOS / WASM
```

Language bindings do not reimplement Sharpe, drawdown, return, concentration, or benchmark formulas.

## Metric coverage

### Return metrics

- observations
- total compounded return
- annualized compounded return / CAGR
- arithmetic mean return
- geometric mean return
- best period return
- worst period return
- positive period ratio
- negative period ratio

### Volatility and risk-adjusted metrics

- annualized volatility
- downside volatility
- upside volatility
- Sharpe ratio
- Sortino ratio
- Calmar ratio
- Omega ratio
- Sterling ratio
- Burke ratio

### Tail-risk and distribution metrics

- historical VaR
- parametric Gaussian VaR
- CVaR / Expected Shortfall
- tail ratio
- skewness
- excess kurtosis

### Drawdown metrics

- maximum drawdown
- maximum-drawdown peak index
- maximum-drawdown trough index
- current drawdown
- average underwater drawdown
- maximum drawdown duration
- average drawdown duration
- recovery duration after the maximum drawdown
- Ulcer Index

### Benchmark-relative metrics

- active annualized return
- tracking error
- information ratio
- Jensen-style alpha
- beta
- R-squared
- correlation
- upside capture
- downside capture
- Treynor ratio

### Trade metrics

- trade count
- wins / losses / breakeven trades
- win rate / loss rate
- profit factor
- expectancy
- average win
- average loss
- payoff ratio
- best trade return
- worst trade return
- maximum consecutive wins
- maximum consecutive losses
- average holding period
- maximum holding period

### Portfolio metrics

- gross exposure
- net exposure
- long exposure
- short exposure
- leverage
- maximum absolute position weight
- HHI concentration
- effective number of bets
- one-way weight turnover
- average turnover

### Transaction cost and capacity metrics

Factor research reports support explicit linear commission and slippage assumptions:

- transaction cost bps
- slippage bps
- cost rate by rebalance date
- average cost rate
- cumulative estimated cost rate
- gross performance
- after-cost performance

The existing portfolio research layer additionally exposes pluggable linear and square-root impact cost models and capacity curves.

### Factor quality metrics

- Information Coefficient (IC)
- mean IC
- IC standard deviation
- ICIR
- naive IC t-stat
- HAC / Newey-West IC t-stat
- positive IC ratio
- negative IC ratio
- quantile mean returns
- top-minus-bottom quantile spread
- quantile monotonicity
- quantile membership turnover
- rank autocorrelation
- factor stability / PSI / health metrics

### Robustness and anti-overfitting

The factor-research validation layer includes:

- Purged K-Fold
- embargo
- Combinatorial Purged Cross Validation (CPCV)
- walk-forward splits
- label-interval overlap purging
- block bootstrap confidence intervals
- deterministic sign-flip randomization p-values
- Bonferroni correction
- Holm correction
- Benjamini-Hochberg FDR
- Probabilistic Sharpe Ratio (PSR)
- Deflated Sharpe Ratio (DSR)
- Probability of Backtest Overfitting (PBO)

All random/resampling utilities accept deterministic seeds.

## Critical multi-period semantics

A multi-day forward factor return is a **research label**, not an independent daily P&L observation.

For example, consecutive 5-day forward returns overlap. Multiplying every daily 5-day forward return would count the same market days multiple times and produces an invalid CAGR, Sharpe, and drawdown curve.

Finkit therefore separates:

```text
Forward Returns / IC / Quantile Spread
    = prediction-horizon diagnostics

HoldingPeriodPortfolioEngine
    = overlapping active cohorts
    ↓
Daily Portfolio P&L
    ↓
Performance / Drawdown / Cost metrics
```

For every requested holding period, `FactorStudyReport.performance.by_holding_period` is computed from the daily P&L of the real overlapping cohort portfolio. This is the authoritative strategy-performance surface.

The compatibility `returns.cumulative_returns` field remains a horizon-observation research curve for Alphalens-style analysis and must not be confused with tradable multi-day portfolio equity.

## Rust APIs

### Arbitrary return series

```rust
use finkit::performance::{evaluate_returns, PerformanceConfig};

let returns = [0.01, -0.02, 0.03, 0.005];
let report = evaluate_returns(&returns, PerformanceConfig::default());
println!("Sharpe = {}", report.risk.sharpe_ratio);
println!("Max DD = {}", report.drawdown.max_drawdown);
println!("CAGR = {}", report.returns.annualized_return);
```

### Rich legacy backtest evaluation

```rust
use finkit::backtest_evaluation::evaluate_backtest;
use finkit::performance::PerformanceConfig;

let rich = evaluate_backtest(&backtest_result, PerformanceConfig::default());
println!("profit factor = {}", rich.trades.profit_factor);
println!("ulcer index = {}", rich.performance.drawdown.ulcer_index);
```

The original `BacktestResult` API remains source-compatible. Its existing total-return, Sharpe, Sortino, and maximum-drawdown values are contract-tested against the canonical performance engine.

## Language-neutral generic API

For languages that prefer a stable data contract, use the JSON-in/JSON-out quantitative evaluation API.

Request schema version: `1`.

```json
{
  "schema_version": 1,
  "returns": [0.02, -0.01, 0.03, 0.01],
  "benchmark_returns": [0.01, -0.005, 0.015, 0.005],
  "trade_returns": [0.10, -0.05, 0.03],
  "holding_periods": [2, 1, 3],
  "weights": [0.5, -0.3, -0.2],
  "turnover": [1.0, 0.2, 0.3, 0.1],
  "config": {
    "annualization": 252,
    "risk_free_rate": 0.0,
    "minimum_acceptable_return": 0.0,
    "var_confidence": 0.95,
    "transaction_cost_bps": 5.0,
    "slippage_bps": 2.0
  }
}
```

The response contains:

```text
gross
  returns / risk / drawdown / benchmark
after_cost
  returns / risk / drawdown / benchmark
costs
trades
portfolio
```

If `turnover` is omitted, after-cost metrics are intentionally omitted rather than inventing a cost assumption.

## FactorStudy contract

The factor-study response schema is version `2`. Request schema versions `1` and `2` remain accepted during the migration period. Schema v2 adds quantitative evaluation configuration and the complete performance section.

Every supported language consumes the same Rust-owned JSON schema. Current surfaces are:

| Language / platform | Factor study | Generic quant evaluation |
| --- | --- | --- |
| Rust | typed + JSON | typed + JSON |
| Python | native extension JSON facade | native extension JSON facade |
| Node.js | N-API | N-API |
| Java | JNI | JNI |
| Android | JNI | JNI |
| C | stable C ABI | stable C ABI |
| C++ | C ABI wrapper | C ABI wrapper |
| Go | C ABI wrapper | C ABI wrapper |
| .NET | P/Invoke | P/Invoke |
| iOS / Swift | C ABI wrapper | C ABI wrapper |
| WASM / JavaScript | wasm-bindgen | wasm-bindgen |

## Configuration semantics

`annualization` is the number of observations per year for the evaluated P&L series, normally 252 for daily trading days. `risk_free_rate` and `minimum_acceptable_return` are per-observation rates. `var_confidence` must be between zero and one. Transaction-cost and slippage inputs are non-negative basis points.

For factor holding-period performance, Finkit first converts each holding period to actual daily overlapping portfolio P&L, so the annualization remains based on daily observations rather than incorrectly treating every multi-day forward label as an independent period.

## Engineering gates

The quantitative evaluation implementation is protected by:

- canonical SSOT checks for Sharpe, Sortino, maximum drawdown and shared research kernels;
- cross-module equality tests between legacy backtest metrics and the canonical evaluator;
- HHI/effective-bets equality tests between factor-risk facades and the core owner;
- no-lookahead and future-mutation locality tests;
- factor-research unit tests;
- workspace all-target compilation;
- Rust format / lint gates;
- Java, .NET, C/C++, Python, Node, Android, Go, Swift and WASM binding gates;
- Alphalens compatibility and performance-regression gates before release.
