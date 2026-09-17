//! Thin Alphalens-style compatibility facade.
//!
//! This module intentionally owns naming/default translation only. Numerical
//! work delegates to the native analyzers so compatibility cannot fork into a
//! second implementation.

pub mod alphalens {
    use crate::analysis::{self, WeightConfig};
    use crate::data::ResearchFrame;
    use crate::error::ResearchResult;
    use std::collections::BTreeMap;

    pub fn factor_information_coefficient(
        factor_data: &ResearchFrame,
        factor_column: &str,
        forward_returns: &BTreeMap<usize, Vec<f64>>,
    ) -> ResearchResult<BTreeMap<usize, Vec<f64>>> {
        analysis::information_coefficient(factor_data, factor_column, forward_returns)
    }

    pub fn mean_information_coefficient(ic: &BTreeMap<usize, Vec<f64>>) -> BTreeMap<usize, f64> {
        analysis::mean_information_coefficient(ic)
    }

    pub fn factor_weights(
        factor_data: &ResearchFrame,
        factor_column: &str,
        demeaned: bool,
        group_adjust: Option<String>,
        equal_weight: bool,
    ) -> ResearchResult<Vec<f64>> {
        analysis::factor_weights(
            factor_data,
            factor_column,
            &WeightConfig {
                demeaned,
                group_adjust,
                equal_weight,
            },
        )
    }

    pub fn factor_returns(
        factor_data: &ResearchFrame,
        weights: &[f64],
        forward_returns: &BTreeMap<usize, Vec<f64>>,
    ) -> ResearchResult<BTreeMap<usize, Vec<f64>>> {
        analysis::factor_returns(factor_data, weights, forward_returns)
    }

    pub fn mean_return_by_quantile(
        quantiles: &[u16],
        forward_returns: &BTreeMap<usize, Vec<f64>>,
    ) -> BTreeMap<u16, BTreeMap<usize, f64>> {
        analysis::mean_return_by_quantile(quantiles, forward_returns)
    }

    pub fn quantile_turnover(
        factor_data: &ResearchFrame,
        quantiles: &[u16],
        quantile: u16,
        period: usize,
    ) -> ResearchResult<Vec<f64>> {
        analysis::quantile_turnover(factor_data, quantiles, quantile, period)
    }

    pub fn factor_rank_autocorrelation(
        factor_data: &ResearchFrame,
        factor_column: &str,
        period: usize,
    ) -> ResearchResult<Vec<f64>> {
        analysis::rank_autocorrelation(factor_data, factor_column, period)
    }
}
