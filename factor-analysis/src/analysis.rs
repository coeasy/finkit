use crate::data::{AssetId, GroupId, ResearchFrame};
use crate::error::{ResearchError, ResearchResult};
use finkit::math::information::pairwise_pearson;
use finkit::math::rank::fractional_ranks;
use finkit::math::regression::simple_ols;
use finkit::returns::cumulative_returns;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Factor-weight construction controls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightConfig {
    pub demeaned: bool,
    pub group_adjust: Option<String>,
    pub equal_weight: bool,
}

impl Default for WeightConfig {
    fn default() -> Self {
        Self {
            demeaned: true,
            group_adjust: None,
            equal_weight: false,
        }
    }
}

fn normalize_group(values: &[(usize, f64)], out: &mut [f64], demeaned: bool, equal_weight: bool) {
    if values.is_empty() {
        return;
    }
    let mean = values.iter().map(|(_, v)| *v).sum::<f64>() / values.len() as f64;
    let mut scores: Vec<(usize, f64)> = values
        .iter()
        .map(|(row, value)| {
            let mut score = if demeaned { *value - mean } else { *value };
            if equal_weight {
                score = if score > 0.0 {
                    1.0
                } else if score < 0.0 {
                    -1.0
                } else {
                    0.0
                };
            }
            (*row, score)
        })
        .collect();
    if equal_weight && demeaned {
        let positives = scores.iter().filter(|(_, v)| *v > 0.0).count() as f64;
        let negatives = scores.iter().filter(|(_, v)| *v < 0.0).count() as f64;
        for (_, score) in &mut scores {
            if *score > 0.0 && positives > 0.0 {
                *score /= positives;
            }
            if *score < 0.0 && negatives > 0.0 {
                *score /= negatives;
            }
        }
    }
    let gross = scores.iter().map(|(_, score)| score.abs()).sum::<f64>();
    if gross <= f64::EPSILON {
        return;
    }
    for (row, score) in scores {
        out[row] = score / gross;
    }
}

/// Build date-local factor weights. Group adjustment gives each group equal gross contribution.
pub fn factor_weights(
    frame: &ResearchFrame,
    factor_column: &str,
    config: &WeightConfig,
) -> ResearchResult<Vec<f64>> {
    let factor = frame.column(factor_column)?;
    let groups = match &config.group_adjust {
        Some(name) => Some(frame.group(name)?),
        None => None,
    };
    let mut out = vec![0.0; factor.len()];
    for segment in 0..frame.index().date_segments().len() {
        let range = frame.index().date_segments().range(segment).unwrap();
        if let Some(group_values) = groups {
            let mut by_group: BTreeMap<GroupId, Vec<(usize, f64)>> = BTreeMap::new();
            for row in range {
                if factor[row].is_finite() {
                    by_group
                        .entry(group_values[row])
                        .or_default()
                        .push((row, factor[row]));
                }
            }
            if by_group.is_empty() {
                continue;
            }
            let group_scale = 1.0 / by_group.len() as f64;
            for values in by_group.values() {
                let mut tmp = vec![0.0; out.len()];
                normalize_group(values, &mut tmp, config.demeaned, config.equal_weight);
                for (row, _) in values {
                    out[*row] = tmp[*row] * group_scale;
                }
            }
            let gross =
                range_weights_gross(&out, frame.index().date_segments().range(segment).unwrap());
            if gross > f64::EPSILON {
                for row in frame.index().date_segments().range(segment).unwrap() {
                    out[row] /= gross;
                }
            }
        } else {
            let values: Vec<(usize, f64)> = range
                .filter(|&row| factor[row].is_finite())
                .map(|row| (row, factor[row]))
                .collect();
            normalize_group(&values, &mut out, config.demeaned, config.equal_weight);
        }
    }
    Ok(out)
}

fn range_weights_gross(weights: &[f64], range: std::ops::Range<usize>) -> f64 {
    range.map(|row| weights[row].abs()).sum()
}

/// Aggregate asset-level forward returns into date-level factor portfolio returns.
pub fn factor_returns(
    frame: &ResearchFrame,
    weights: &[f64],
    forward_returns: &BTreeMap<usize, Vec<f64>>,
) -> ResearchResult<BTreeMap<usize, Vec<f64>>> {
    if weights.len() != frame.index().len() {
        return Err(ResearchError::LengthMismatch {
            name: "weights".to_string(),
            expected: frame.index().len(),
            actual: weights.len(),
        });
    }
    let mut result = BTreeMap::new();
    for (&period, returns) in forward_returns {
        if returns.len() != frame.index().len() {
            return Err(ResearchError::LengthMismatch {
                name: format!("forward return {period}"),
                expected: frame.index().len(),
                actual: returns.len(),
            });
        }
        let values = frame.index().date_segments().map(|range| {
            let mut sum = 0.0;
            let mut any = false;
            for row in range {
                if returns[row].is_finite() && weights[row].is_finite() {
                    sum += returns[row] * weights[row];
                    any = true;
                }
            }
            if any {
                sum
            } else {
                f64::NAN
            }
        });
        result.insert(period, values);
    }
    Ok(result)
}

/// Date-wise Spearman information coefficient for every horizon.
pub fn information_coefficient(
    frame: &ResearchFrame,
    factor_column: &str,
    forward_returns: &BTreeMap<usize, Vec<f64>>,
) -> ResearchResult<BTreeMap<usize, Vec<f64>>> {
    let factor = frame.column(factor_column)?;
    let mut result = BTreeMap::new();
    for (&period, returns) in forward_returns {
        let per_date = frame.index().date_segments().map(|range| {
            let pairs: Vec<(f64, f64)> = range
                .filter_map(|row| {
                    let f = factor[row];
                    let r = returns[row];
                    (f.is_finite() && r.is_finite()).then_some((f, r))
                })
                .collect();
            if pairs.len() < 2 {
                return f64::NAN;
            }
            let f: Vec<f64> = pairs.iter().map(|v| v.0).collect();
            let r: Vec<f64> = pairs.iter().map(|v| v.1).collect();
            let rf = fractional_ranks(&f);
            let rr = fractional_ranks(&r);
            pairwise_pearson(&rf, &rr)
        });
        result.insert(period, per_date);
    }
    Ok(result)
}

/// Mean IC by horizon.
pub fn mean_information_coefficient(ic: &BTreeMap<usize, Vec<f64>>) -> BTreeMap<usize, f64> {
    ic.iter()
        .map(|(&period, values)| {
            let finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
            let mean = if finite.is_empty() {
                f64::NAN
            } else {
                finite.iter().sum::<f64>() / finite.len() as f64
            };
            (period, mean)
        })
        .collect()
}

/// Mean forward return for every quantile and horizon.
pub fn mean_return_by_quantile(
    quantiles: &[u16],
    forward_returns: &BTreeMap<usize, Vec<f64>>,
) -> BTreeMap<u16, BTreeMap<usize, f64>> {
    let max_q = quantiles.iter().copied().max().unwrap_or(0);
    (1..=max_q)
        .map(|q| {
            let by_period = forward_returns
                .iter()
                .map(|(&period, values)| {
                    let selected: Vec<f64> = values
                        .iter()
                        .copied()
                        .zip(quantiles.iter().copied())
                        .filter_map(|(value, bucket)| {
                            (bucket == q && value.is_finite()).then_some(value)
                        })
                        .collect();
                    let mean = if selected.is_empty() {
                        f64::NAN
                    } else {
                        selected.iter().sum::<f64>() / selected.len() as f64
                    };
                    (period, mean)
                })
                .collect();
            (q, by_period)
        })
        .collect()
}

/// Quantile membership turnover for a fixed lag in date segments.
pub fn quantile_turnover(
    frame: &ResearchFrame,
    quantiles: &[u16],
    quantile: u16,
    lag: usize,
) -> ResearchResult<Vec<f64>> {
    if quantiles.len() != frame.index().len() {
        return Err(ResearchError::LengthMismatch {
            name: "quantiles".to_string(),
            expected: frame.index().len(),
            actual: quantiles.len(),
        });
    }
    if lag == 0 {
        return Err(ResearchError::InvalidConfig(
            "turnover lag must be > 0".to_string(),
        ));
    }
    let sets: Vec<BTreeSet<AssetId>> = frame.index().date_segments().map(|range| {
        range
            .filter(|&row| quantiles[row] == quantile)
            .map(|row| frame.index().assets()[row])
            .collect()
    });
    let mut out = vec![f64::NAN; sets.len()];
    for i in lag..sets.len() {
        if sets[i].is_empty() {
            out[i] = 0.0;
            continue;
        }
        let new_count = sets[i].difference(&sets[i - lag]).count();
        out[i] = new_count as f64 / sets[i].len() as f64;
    }
    Ok(out)
}

/// Date-wise factor rank autocorrelation with asset alignment.
pub fn rank_autocorrelation(
    frame: &ResearchFrame,
    factor_column: &str,
    lag: usize,
) -> ResearchResult<Vec<f64>> {
    let factor = frame.column(factor_column)?;
    let snapshots: Vec<BTreeMap<AssetId, f64>> = frame.index().date_segments().map(|range| {
        let rows: Vec<usize> = range.filter(|&row| factor[row].is_finite()).collect();
        let vals: Vec<f64> = rows.iter().map(|&row| factor[row]).collect();
        let ranks = fractional_ranks(&vals);
        rows.into_iter()
            .zip(ranks)
            .map(|(row, rank)| (frame.index().assets()[row], rank))
            .collect()
    });
    let mut out = vec![f64::NAN; snapshots.len()];
    for i in lag..snapshots.len() {
        let pairs: Vec<(f64, f64)> = snapshots[i]
            .iter()
            .filter_map(|(asset, &rank)| snapshots[i - lag].get(asset).map(|&old| (rank, old)))
            .collect();
        if pairs.len() >= 2 {
            let a: Vec<f64> = pairs.iter().map(|v| v.0).collect();
            let b: Vec<f64> = pairs.iter().map(|v| v.1).collect();
            out[i] = pairwise_pearson(&a, &b);
        }
    }
    Ok(out)
}

/// Alpha/beta relative to equal-weight universe return for each horizon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlphaBeta {
    pub alpha: f64,
    pub beta: f64,
    pub r_squared: f64,
}

pub fn factor_alpha_beta(
    frame: &ResearchFrame,
    factor_ret: &BTreeMap<usize, Vec<f64>>,
    forward_returns: &BTreeMap<usize, Vec<f64>>,
) -> BTreeMap<usize, AlphaBeta> {
    factor_ret
        .iter()
        .filter_map(|(&period, returns)| {
            let asset_returns = forward_returns.get(&period)?;
            let universe = frame.index().date_segments().map(|range| {
                let valid: Vec<f64> = range
                    .map(|row| asset_returns[row])
                    .filter(|v| v.is_finite())
                    .collect();
                if valid.is_empty() {
                    f64::NAN
                } else {
                    valid.iter().sum::<f64>() / valid.len() as f64
                }
            });
            let fit = simple_ols(returns, &universe).ok()?;
            Some((
                period,
                AlphaBeta {
                    alpha: fit.intercept,
                    beta: fit.coefficients.first().copied().unwrap_or(f64::NAN),
                    r_squared: fit.r_squared,
                },
            ))
        })
        .collect()
}

/// Cumulative wealth by horizon from date-level factor returns.
pub fn cumulative_factor_returns(
    factor_ret: &BTreeMap<usize, Vec<f64>>,
) -> BTreeMap<usize, Vec<f64>> {
    factor_ret
        .iter()
        .map(|(&period, values)| (period, cumulative_returns(values, 1.0)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{AssetId, PanelIndex};
    use crate::prepare::{compute_forward_returns, ForwardReturnConfig};
    use finkit::returns::ReturnKind;

    #[test]
    fn ic_detects_cross_sectional_signal() {
        let index = PanelIndex::new(
            vec![1, 1, 1, 2, 2, 2],
            vec![
                AssetId(1),
                AssetId(2),
                AssetId(3),
                AssetId(1),
                AssetId(2),
                AssetId(3),
            ],
        )
        .unwrap();
        let mut frame = ResearchFrame::new(index);
        frame
            .add_numeric("factor", "factor", vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0])
            .unwrap();
        frame
            .add_numeric("price", "market", vec![10.0, 10.0, 10.0, 11.0, 12.0, 13.0])
            .unwrap();
        let forward = compute_forward_returns(
            &frame,
            "price",
            &ForwardReturnConfig::new(vec![1], ReturnKind::Arithmetic).unwrap(),
        )
        .unwrap();
        let ic = information_coefficient(&frame, "factor", &forward).unwrap();
        assert!(ic[&1][0] > 0.99);
    }
}
