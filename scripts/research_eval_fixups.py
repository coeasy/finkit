from pathlib import Path


def replace(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    if new in text:
        return
    if old not in text:
        raise RuntimeError(f'missing expected source fragment in {path}')
    p.write_text(text.replace(old, new, 1))


# FactorStudy tests follow the tradable overlapping-cohort report shape.
replace(
    'factor-analysis/src/api.rs',
    'assert!(report.performance.by_horizon.contains_key(&1));',
    'assert!(report.performance.by_holding_period.contains_key(&1));',
)

# Jensen alpha is defined on excess returns rather than raw returns.
replace(
    'core/src/performance.rs',
    'let alpha_per_period = strategy_mean - beta * benchmark_mean;',
    'let alpha_per_period = (strategy_mean - config.risk_free_rate)\n        - beta * (benchmark_mean - config.risk_free_rate);',
)

# SegmentLayout::map already materializes a Vec; do not collect a Vec again.
replace(
    'factor-analysis/src/performance.rs',
    '        })\n        .collect()\n}\n\n/// One-way weight turnover.',
    '        })\n}\n\n/// One-way weight turnover.',
)
replace(
    'factor-analysis/src/performance.rs',
    '        .map(|range| core::evaluate_portfolio(&weights[range]).into())\n        .collect();',
    '        .map(|range| core::evaluate_portfolio(&weights[range]).into());',
)
replace(
    'factor-analysis/src/portfolio_performance.rs',
    '        })\n        .collect())\n}\n\npub fn evaluate_factor_holding_periods(',
    '        }))\n}\n\npub fn evaluate_factor_holding_periods(',
)

# Trade holding-period statistics must stay aligned with the finite trade
# observations retained by the trade-return metrics.
replace(
    'core/src/performance.rs',
    '    let valid_holding: Vec<usize> = holding_periods.unwrap_or(&[]).iter().copied().collect();',
    '''    let valid_holding: Vec<usize> = holding_periods
        .filter(|periods| periods.len() == trade_returns.len())
        .map(|periods| {
            trade_returns
                .iter()
                .zip(periods.iter().copied())
                .filter_map(|(ret, period)| ret.is_finite().then_some(period))
                .collect()
        })
        .unwrap_or_default();''',
)
replace(
    'core/src/performance.rs',
    '''    #[test]
    fn portfolio_metrics_have_consistent_concentration() {''',
    '''    #[test]
    fn trade_holding_periods_follow_finite_trade_returns() {
        let metrics = evaluate_trades(&[0.10, f64::NAN, -0.05], Some(&[1, 100, 3]));
        assert_eq!(metrics.trades, 2);
        assert!((metrics.average_holding_period - 2.0).abs() < 1e-12);
        assert_eq!(metrics.max_holding_period, 3);
    }

    #[test]
    fn portfolio_metrics_have_consistent_concentration() {''',
)

# Direct parametric VaR callers must use the finite observation count for both
# the mean and sample variance, matching the performance SSOT behavior.
replace(
    'core/src/risk.rs',
    '''pub fn var_parametric(returns: &[f64], confidence: f64) -> f64 {
    if returns.len() < 2 || confidence <= 0.0 || confidence >= 1.0 {
        return 0.0;
    }
    let n = returns.len() as f64;
    let mean = returns.iter().filter(|r| r.is_finite()).sum::<f64>() / n;
    let var: f64 = returns
        .iter()
        .filter(|r| r.is_finite())
        .map(|r| (r - mean).powi(2))
        .sum::<f64>()
        / (n - 1.0);
    let std = var.sqrt();
    let z = normal_quantile(confidence);
    -(mean - z * std)
}''',
    '''pub fn var_parametric(returns: &[f64], confidence: f64) -> f64 {
    if confidence <= 0.0 || confidence >= 1.0 {
        return 0.0;
    }
    let finite: Vec<f64> = returns.iter().copied().filter(|r| r.is_finite()).collect();
    if finite.len() < 2 {
        return 0.0;
    }
    let n = finite.len() as f64;
    let mean = finite.iter().sum::<f64>() / n;
    let var = finite.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let std = var.sqrt();
    let z = normal_quantile(confidence);
    -(mean - z * std)
}''',
)

# Preserve the old zero-lag positions API while adding explicit execution lag.
replace(
    'factor-analysis/src/portfolio.rs',
    '''    /// Produce one normalized asset-weight map per research date.
    pub fn positions(
        &self,
        frame: &ResearchFrame,
        row_weights: &[f64],
    ) -> ResearchResult<Vec<BTreeMap<AssetId, f64>>> {
        if row_weights.len() != frame.index().len() {''',
    '''    /// Produce one normalized asset-weight map per research date using zero execution lag.
    pub fn positions(
        &self,
        frame: &ResearchFrame,
        row_weights: &[f64],
    ) -> ResearchResult<Vec<BTreeMap<AssetId, f64>>> {
        self.positions_with_lag(frame, row_weights, 0)
    }

    /// Produce positions after delaying factor formation by `execution_lag` research dates.
    pub fn positions_with_lag(
        &self,
        frame: &ResearchFrame,
        row_weights: &[f64],
        execution_lag: usize,
    ) -> ResearchResult<Vec<BTreeMap<AssetId, f64>>> {
        if row_weights.len() != frame.index().len() {''',
)
replace(
    'factor-analysis/src/portfolio.rs',
    '''            let range = frame.index().date_segments().range(date_idx).unwrap();
            let cohort_weights: BTreeMap<AssetId, f64> = range
                .filter_map(|row| {
                    let weight = row_weights[row];
                    (weight.is_finite() && weight != 0.0)
                        .then_some((frame.index().assets()[row], weight))
                })
                .collect();
            if !cohort_weights.is_empty() {
                active.push_back(Cohort {
                    expires_on_date: date_idx + self.holding_dates,
                    weights: cohort_weights,
                });
            }''',
    '''            if let Some(formation_date) = date_idx.checked_sub(execution_lag) {
                let range = frame
                    .index()
                    .date_segments()
                    .range(formation_date)
                    .expect("valid formation date");
                let cohort_weights: BTreeMap<AssetId, f64> = range
                    .filter_map(|row| {
                        let weight = row_weights[row];
                        (weight.is_finite() && weight != 0.0)
                            .then_some((frame.index().assets()[row], weight))
                    })
                    .collect();
                if !cohort_weights.is_empty() {
                    active.push_back(Cohort {
                        expires_on_date: date_idx + self.holding_dates,
                        weights: cohort_weights,
                    });
                }
            }''',
)
replace(
    'factor-analysis/src/portfolio.rs',
    '''    #[test]
    fn holding_engine_overlaps_cohorts() {''',
    '''    #[test]
    fn execution_lag_delays_cohort_activation() {
        let index = PanelIndex::new(
            vec![1, 1, 2, 2],
            vec![AssetId(1), AssetId(2), AssetId(1), AssetId(2)],
        )
        .unwrap();
        let frame = ResearchFrame::new(index);
        let positions = HoldingPeriodPortfolioEngine::new(1)
            .unwrap()
            .positions_with_lag(&frame, &[0.5, -0.5, 0.8, -0.2], 1)
            .unwrap();
        assert!(positions[0].is_empty());
        assert_eq!(positions[1].get(&AssetId(1)).copied(), Some(0.5));
        assert_eq!(positions[1].get(&AssetId(2)).copied(), Some(-0.5));
    }

    #[test]
    fn holding_engine_overlaps_cohorts() {''',
)

# Tradable multi-period performance exposes the actual daily P&L and wealth
# curve. This prevents overlapping 5D/10D/etc forward returns from being
# compounded as though they were independent daily observations.
replace(
    'factor-analysis/src/portfolio_performance.rs',
    'use finkit::performance as core;\n',
    'use finkit::performance as core;\nuse finkit::returns::cumulative_returns;\n',
)
replace(
    'factor-analysis/src/portfolio_performance.rs',
    '''pub struct HorizonPortfolioPerformance {
    /// Actual daily P&L of overlapping active cohorts before costs.
    pub gross: QuantEvaluationReport,
    /// Actual daily P&L after linear turnover costs/slippage.
    pub after_cost: QuantEvaluationReport,
    pub portfolio: PortfolioSummaryReport,
    pub costs: CostSummaryReport,
}''',
    '''pub struct HorizonPortfolioPerformance {
    /// Actual daily P&L of overlapping active cohorts before costs.
    pub gross: QuantEvaluationReport,
    /// Actual daily P&L after linear turnover costs/slippage.
    pub after_cost: QuantEvaluationReport,
    /// Daily tradable return stream before costs.
    pub gross_daily_returns: Vec<f64>,
    /// Daily tradable return stream after costs.
    pub after_cost_daily_returns: Vec<f64>,
    /// Wealth curve compounded from the daily tradable gross return stream.
    pub gross_cumulative_wealth: Vec<f64>,
    /// Wealth curve compounded from the daily tradable net return stream.
    pub after_cost_cumulative_wealth: Vec<f64>,
    pub portfolio: PortfolioSummaryReport,
    pub costs: CostSummaryReport,
}''',
)
replace(
    'factor-analysis/src/portfolio_performance.rs',
    '''pub fn evaluate_factor_holding_periods(
    frame: &ResearchFrame,
    target_row_weights: &[f64],
    daily_asset_returns: &[f64],
    holding_periods: &[usize],
    config: EvaluationConfig,
) -> ResearchResult<FactorPortfolioPerformanceReport> {''',
    '''pub fn evaluate_factor_holding_periods(
    frame: &ResearchFrame,
    target_row_weights: &[f64],
    daily_asset_returns: &[f64],
    holding_periods: &[usize],
    execution_lag: usize,
    config: EvaluationConfig,
) -> ResearchResult<FactorPortfolioPerformanceReport> {''',
)
replace(
    'factor-analysis/src/portfolio_performance.rs',
    '''        let positions =
            HoldingPeriodPortfolioEngine::new(period)?.positions(frame, target_row_weights)?;''',
    '''        let positions = HoldingPeriodPortfolioEngine::new(period)?.positions_with_lag(
            frame,
            target_row_weights,
            execution_lag,
        )?;''',
)
replace(
    'factor-analysis/src/portfolio_performance.rs',
    '''        by_holding_period.insert(
            period,
            HorizonPortfolioPerformance {
                gross: evaluate_quant_performance(&gross_returns, Some(&benchmark), config),
                after_cost: evaluate_quant_performance(&net_returns, Some(&benchmark), config),
                portfolio,
                costs,
            },
        );''',
    '''        let gross = evaluate_quant_performance(&gross_returns, Some(&benchmark), config);
        let after_cost = evaluate_quant_performance(&net_returns, Some(&benchmark), config);
        let gross_cumulative_wealth = cumulative_returns(&gross_returns, 1.0);
        let after_cost_cumulative_wealth = cumulative_returns(&net_returns, 1.0);
        by_holding_period.insert(
            period,
            HorizonPortfolioPerformance {
                gross,
                after_cost,
                gross_daily_returns: gross_returns,
                after_cost_daily_returns: net_returns,
                gross_cumulative_wealth,
                after_cost_cumulative_wealth,
                portfolio,
                costs,
            },
        );''',
)
replace(
    'factor-analysis/src/portfolio_performance.rs',
    '''            &[1, 2],
            EvaluationConfig::default(),''',
    '''            &[1, 2],
            0,
            EvaluationConfig::default(),''',
)

# FactorStudy makes execution timing explicit and sources cumulative wealth
# from the tradable overlapping-cohort return stream.
replace(
    'factor-analysis/src/report.rs',
    '    cumulative_factor_returns, factor_alpha_beta, factor_returns, factor_weights,\n',
    '    factor_alpha_beta, factor_returns, factor_weights,\n',
)
replace(
    'factor-analysis/src/report.rs',
    '''    pub quantiles: u16,
    pub periods: Vec<usize>,''',
    '''    pub quantiles: u16,
    pub periods: Vec<usize>,
    /// Number of research dates between signal formation and portfolio activation.
    pub execution_lag: usize,''',
)
replace(
    'factor-analysis/src/report.rs',
    '''    evaluation: EvaluationConfig,
    mode: AnalysisMode,''',
    '''    evaluation: EvaluationConfig,
    execution_lag: usize,
    mode: AnalysisMode,''',
)
replace(
    'factor-analysis/src/report.rs',
    '''            evaluation: EvaluationConfig::default(),
            mode: AnalysisMode::Native,''',
    '''            evaluation: EvaluationConfig::default(),
            execution_lag: 0,
            mode: AnalysisMode::Native,''',
)
replace(
    'factor-analysis/src/report.rs',
    '''    pub fn mode(mut self, mode: AnalysisMode) -> Self {
        self.mode = mode;
        self
    }''',
    '''    pub fn execution_lag(mut self, execution_lag: usize) -> Self {
        self.execution_lag = execution_lag;
        self
    }

    pub fn mode(mut self, mode: AnalysisMode) -> Self {
        self.mode = mode;
        self
    }''',
)
replace(
    'factor-analysis/src/report.rs',
    '        let cumulative = cumulative_factor_returns(&factor_ret);\n',
    '',
)
replace(
    'factor-analysis/src/report.rs',
    '''            &daily_asset_returns,
            &self.periods,
            self.evaluation,
        )?;

        Ok(FactorStudyReport {''',
    '''            &daily_asset_returns,
            &self.periods,
            self.execution_lag,
            self.evaluation,
        )?;
        let cumulative = performance
            .by_holding_period
            .iter()
            .map(|(&period, result)| (period, result.gross_cumulative_wealth.clone()))
            .collect();

        Ok(FactorStudyReport {''',
)
replace(
    'factor-analysis/src/report.rs',
    '''            quantiles: self.quantize.quantiles,
            periods: self.periods.clone(),
            returns: ReturnsReport {''',
    '''            quantiles: self.quantize.quantiles,
            periods: self.periods.clone(),
            execution_lag: self.execution_lag,
            returns: ReturnsReport {''',
)
replace(
    'factor-analysis/src/report.rs',
    '''        assert_eq!(report.periods, vec![1]);
        assert!(report.information.mean_ic[&1] > 0.9);''',
    '''        assert_eq!(report.periods, vec![1]);
        assert_eq!(report.execution_lag, 0);
        assert_eq!(
            report.returns.cumulative_returns[&1],
            report.performance.by_holding_period[&1].gross_cumulative_wealth
        );
        assert!(report.information.mean_ic[&1] > 0.9);''',
)

# Schema v2 stays backward compatible because execution_lag defaults to zero.
replace(
    'factor-analysis/src/api.rs',
    '''    #[serde(default = "default_mode")]
    pub mode: AnalysisMode,
    /// Quantitative evaluation settings shared by every language binding.''',
    '''    #[serde(default = "default_mode")]
    pub mode: AnalysisMode,
    /// Research-date lag between factor formation and portfolio activation.
    #[serde(default)]
    pub execution_lag: usize,
    /// Quantitative evaluation settings shared by every language binding.''',
)
replace(
    'factor-analysis/src/api.rs',
    '''        .mode(request.mode)
        .quantize_config''',
    '''        .mode(request.mode)
        .execution_lag(request.execution_lag)
        .quantize_config''',
)
replace(
    'factor-analysis/src/api.rs',
    '''            mode: AnalysisMode::Native,
            evaluation: EvaluationConfig::default(),''',
    '''            mode: AnalysisMode::Native,
            execution_lag: 0,
            evaluation: EvaluationConfig::default(),''',
)
replace(
    'factor-analysis/src/api.rs',
    '''        assert_eq!(report.periods, vec![1]);
        assert_eq!(report.quantiles, 3);''',
    '''        assert_eq!(report.periods, vec![1]);
        assert_eq!(report.quantiles, 3);
        assert_eq!(report.execution_lag, 0);''',
)
