//! Language-neutral quantitative strategy/portfolio evaluation API.

use crate::performance::{
    evaluate_costs, evaluate_quant_performance, CostSummaryReport, EvaluationConfig,
    PortfolioMetricsReport, QuantEvaluationReport,
};
use finkit::performance as core;
use serde::{Deserialize, Serialize};

pub const QUANT_EVALUATION_SCHEMA_VERSION: u32 = 1;

fn default_schema_version() -> u32 {
    QUANT_EVALUATION_SCHEMA_VERSION
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantEvaluationRequest {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub returns: Vec<f64>,
    #[serde(default)]
    pub benchmark_returns: Option<Vec<f64>>,
    #[serde(default)]
    pub trade_returns: Option<Vec<f64>>,
    #[serde(default)]
    pub holding_periods: Option<Vec<usize>>,
    /// Current portfolio weight vector for exposure/concentration metrics.
    #[serde(default)]
    pub weights: Option<Vec<f64>>,
    /// Optional one-way turnover aligned with `returns` for after-cost metrics.
    #[serde(default)]
    pub turnover: Option<Vec<f64>>,
    #[serde(default)]
    pub config: EvaluationConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeMetricsReport {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantEvaluationApiReport {
    pub gross: QuantEvaluationReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_cost: Option<QuantEvaluationReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub costs: Option<CostSummaryReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trades: Option<TradeMetricsReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portfolio: Option<PortfolioMetricsReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuantEvaluationApiError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantEvaluationResponse {
    pub schema_version: u32,
    pub library_version: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<QuantEvaluationApiReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<QuantEvaluationApiError>,
}

fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

impl From<core::TradeMetrics> for TradeMetricsReport {
    fn from(value: core::TradeMetrics) -> Self {
        Self {
            trades: value.trades,
            wins: value.wins,
            losses: value.losses,
            breakeven: value.breakeven,
            win_rate: finite_or_zero(value.win_rate),
            loss_rate: finite_or_zero(value.loss_rate),
            profit_factor: finite_or_zero(value.profit_factor),
            expectancy: finite_or_zero(value.expectancy),
            average_win: finite_or_zero(value.average_win),
            average_loss: finite_or_zero(value.average_loss),
            payoff_ratio: finite_or_zero(value.payoff_ratio),
            best_trade_return: finite_or_zero(value.best_trade_return),
            worst_trade_return: finite_or_zero(value.worst_trade_return),
            max_consecutive_wins: value.max_consecutive_wins,
            max_consecutive_losses: value.max_consecutive_losses,
            average_holding_period: finite_or_zero(value.average_holding_period),
            max_holding_period: value.max_holding_period,
        }
    }
}

fn invalid(message: impl Into<String>) -> QuantEvaluationApiError {
    QuantEvaluationApiError {
        code: "invalid_request".to_string(),
        message: message.into(),
    }
}

pub fn validate_quant_evaluation_request(
    request: &QuantEvaluationRequest,
) -> Result<(), QuantEvaluationApiError> {
    if request.schema_version != QUANT_EVALUATION_SCHEMA_VERSION {
        return Err(QuantEvaluationApiError {
            code: "unsupported_schema".to_string(),
            message: format!(
                "unsupported quant evaluation schema {}; expected {}",
                request.schema_version, QUANT_EVALUATION_SCHEMA_VERSION
            ),
        });
    }
    if request.returns.is_empty() || !request.returns.iter().any(|value| value.is_finite()) {
        return Err(invalid(
            "returns must contain at least one finite observation",
        ));
    }
    if let Some(benchmark) = &request.benchmark_returns {
        if benchmark.len() != request.returns.len() {
            return Err(invalid(format!(
                "benchmark_returns length mismatch: expected {}, got {}",
                request.returns.len(),
                benchmark.len()
            )));
        }
    }
    if let Some(turnover) = &request.turnover {
        if turnover.len() != request.returns.len() {
            return Err(invalid(format!(
                "turnover length mismatch: expected {}, got {}",
                request.returns.len(),
                turnover.len()
            )));
        }
        if turnover
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(invalid("turnover values must be finite and non-negative"));
        }
    }
    if let (Some(trades), Some(holding)) = (&request.trade_returns, &request.holding_periods) {
        if trades.len() != holding.len() {
            return Err(invalid(format!(
                "holding_periods length mismatch: expected {}, got {}",
                trades.len(),
                holding.len()
            )));
        }
    }
    if request.config.annualization == 0
        || !request.config.risk_free_rate.is_finite()
        || !request.config.minimum_acceptable_return.is_finite()
        || !(0.0 < request.config.var_confidence && request.config.var_confidence < 1.0)
        || !request.config.transaction_cost_bps.is_finite()
        || request.config.transaction_cost_bps < 0.0
        || !request.config.slippage_bps.is_finite()
        || request.config.slippage_bps < 0.0
    {
        return Err(invalid("invalid evaluation configuration"));
    }
    Ok(())
}

pub fn run_quant_evaluation(
    request: &QuantEvaluationRequest,
) -> Result<QuantEvaluationApiReport, QuantEvaluationApiError> {
    validate_quant_evaluation_request(request)?;
    let benchmark = request.benchmark_returns.as_deref();
    let gross = evaluate_quant_performance(&request.returns, benchmark, request.config);
    let (after_cost, costs) = if let Some(turnover) = &request.turnover {
        let costs = evaluate_costs(turnover, request.config);
        let net_returns: Vec<f64> = request
            .returns
            .iter()
            .enumerate()
            .map(|(index, value)| {
                if value.is_finite() {
                    *value - costs.estimated_cost_rate_by_date[index]
                } else {
                    *value
                }
            })
            .collect();
        (
            Some(evaluate_quant_performance(
                &net_returns,
                benchmark,
                request.config,
            )),
            Some(costs),
        )
    } else {
        (None, None)
    };
    let trades = request
        .trade_returns
        .as_ref()
        .map(|returns| core::evaluate_trades(returns, request.holding_periods.as_deref()).into());
    let portfolio = request
        .weights
        .as_ref()
        .map(|weights| core::evaluate_portfolio(weights).into());
    Ok(QuantEvaluationApiReport {
        gross,
        after_cost,
        costs,
        trades,
        portfolio,
    })
}

pub fn run_quant_evaluation_response(request: &QuantEvaluationRequest) -> QuantEvaluationResponse {
    match run_quant_evaluation(request) {
        Ok(report) => QuantEvaluationResponse {
            schema_version: QUANT_EVALUATION_SCHEMA_VERSION,
            library_version: env!("CARGO_PKG_VERSION").to_string(),
            ok: true,
            report: Some(report),
            error: None,
        },
        Err(error) => QuantEvaluationResponse {
            schema_version: QUANT_EVALUATION_SCHEMA_VERSION,
            library_version: env!("CARGO_PKG_VERSION").to_string(),
            ok: false,
            report: None,
            error: Some(error),
        },
    }
}

pub fn run_quant_evaluation_json(request_json: &str) -> String {
    let response = match serde_json::from_str::<QuantEvaluationRequest>(request_json) {
        Ok(request) => run_quant_evaluation_response(&request),
        Err(error) => QuantEvaluationResponse {
            schema_version: QUANT_EVALUATION_SCHEMA_VERSION,
            library_version: env!("CARGO_PKG_VERSION").to_string(),
            ok: false,
            report: None,
            error: Some(QuantEvaluationApiError {
                code: "invalid_json".to_string(),
                message: error.to_string(),
            }),
        },
    };
    serde_json::to_string(&response).unwrap_or_else(|error| {
        format!(
            "{{\"schema_version\":{},\"library_version\":\"{}\",\"ok\":false,\"error\":{{\"code\":\"serialization_failed\",\"message\":{:?}}}}}",
            QUANT_EVALUATION_SCHEMA_VERSION,
            env!("CARGO_PKG_VERSION"),
            error.to_string()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> QuantEvaluationRequest {
        QuantEvaluationRequest {
            schema_version: QUANT_EVALUATION_SCHEMA_VERSION,
            returns: vec![0.02, -0.01, 0.03, 0.01],
            benchmark_returns: Some(vec![0.01, -0.005, 0.015, 0.005]),
            trade_returns: Some(vec![0.1, -0.05, 0.03]),
            holding_periods: Some(vec![2, 1, 3]),
            weights: Some(vec![0.5, -0.3, -0.2]),
            turnover: Some(vec![1.0, 0.2, 0.3, 0.1]),
            config: EvaluationConfig {
                transaction_cost_bps: 5.0,
                slippage_bps: 2.0,
                ..EvaluationConfig::default()
            },
        }
    }

    #[test]
    fn generic_api_exposes_strategy_trade_portfolio_and_cost_metrics() {
        let response = run_quant_evaluation_response(&request());
        assert!(response.ok);
        let report = response.report.unwrap();
        assert!(report.gross.returns.total_return > 0.0);
        assert!(
            report.after_cost.unwrap().returns.total_return < report.gross.returns.total_return
        );
        assert_eq!(report.trades.unwrap().trades, 3);
        assert!(report.portfolio.unwrap().gross_exposure > 0.0);
    }

    #[test]
    fn generic_json_boundary_is_stable() {
        let response: QuantEvaluationResponse =
            serde_json::from_str(&run_quant_evaluation_json("not-json")).unwrap();
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, "invalid_json");
    }
}
