use crate::data::{AssetId, GroupId, ResearchFrame};
use crate::error::{ResearchError, ResearchResult};
use finkit::math::rank::percentile_rank;
use finkit::returns::{return_between, ReturnKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Forward-return computation configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardReturnConfig {
    pub periods: Vec<usize>,
    pub kind: ReturnKind,
}

impl ForwardReturnConfig {
    pub fn new(periods: Vec<usize>, kind: ReturnKind) -> ResearchResult<Self> {
        if periods.is_empty() || periods.iter().any(|&period| period == 0) {
            return Err(ResearchError::InvalidConfig(
                "forward-return periods must be non-empty and > 0".to_string(),
            ));
        }
        Ok(Self { periods, kind })
    }
}

/// Compute forward returns by asset, so no return can cross an instrument boundary.
pub fn compute_forward_returns(
    frame: &ResearchFrame,
    price_column: &str,
    config: &ForwardReturnConfig,
) -> ResearchResult<BTreeMap<usize, Vec<f64>>> {
    let prices = frame.column(price_column)?;
    let mut by_asset: BTreeMap<AssetId, Vec<usize>> = BTreeMap::new();
    for (row, &asset) in frame.index().assets().iter().enumerate() {
        by_asset.entry(asset).or_default().push(row);
    }
    let mut result: BTreeMap<usize, Vec<f64>> = config
        .periods
        .iter()
        .copied()
        .map(|period| (period, vec![f64::NAN; frame.index().len()]))
        .collect();
    for rows in by_asset.values() {
        for &period in &config.periods {
            let out = result.get_mut(&period).expect("period initialized");
            for local in 0..rows.len().saturating_sub(period) {
                let start = rows[local];
                let end = rows[local + period];
                out[start] = return_between(prices[start], prices[end], config.kind);
            }
        }
    }
    Ok(result)
}

/// Quantile assignment configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantizeConfig {
    pub quantiles: u16,
    pub by_group: Option<String>,
    pub zero_aware: bool,
}

impl Default for QuantizeConfig {
    fn default() -> Self {
        Self {
            quantiles: 5,
            by_group: None,
            zero_aware: false,
        }
    }
}

fn assign_bucket_rows(rows: &[(usize, f64)], quantiles: u16, out: &mut [u16]) {
    if rows.is_empty() || quantiles == 0 {
        return;
    }
    let values: Vec<f64> = rows.iter().map(|(_, value)| *value).collect();
    let pct = percentile_rank(&values);
    for ((row, _), rank) in rows.iter().zip(pct) {
        if rank.is_finite() {
            out[*row] = ((rank * quantiles as f64).floor() as u16 + 1).min(quantiles);
        }
    }
}

fn assign_zero_aware(rows: &[(usize, f64)], quantiles: u16, out: &mut [u16]) {
    if quantiles < 2 {
        assign_bucket_rows(rows, quantiles, out);
        return;
    }
    let lower = quantiles / 2;
    let upper = quantiles - lower;
    let negative: Vec<(usize, f64)> = rows.iter().copied().filter(|(_, v)| *v < 0.0).collect();
    let positive: Vec<(usize, f64)> = rows.iter().copied().filter(|(_, v)| *v > 0.0).collect();
    let zero: Vec<usize> = rows
        .iter()
        .filter(|(_, v)| *v == 0.0)
        .map(|(i, _)| *i)
        .collect();
    assign_bucket_rows(&negative, lower, out);
    if !positive.is_empty() {
        let mut tmp = vec![0u16; out.len()];
        assign_bucket_rows(&positive, upper, &mut tmp);
        for (row, _) in positive {
            if tmp[row] > 0 {
                out[row] = lower + tmp[row];
            }
        }
    }
    let middle = lower.max(1);
    for row in zero {
        out[row] = middle;
    }
}

/// Assign 1-based factor quantiles independently for each timestamp.
pub fn quantize_factor(
    frame: &ResearchFrame,
    factor_column: &str,
    config: &QuantizeConfig,
) -> ResearchResult<Vec<u16>> {
    if config.quantiles == 0 {
        return Err(ResearchError::InvalidConfig(
            "quantiles must be > 0".to_string(),
        ));
    }
    let factor = frame.column(factor_column)?;
    let group_values = match &config.by_group {
        Some(name) => Some(frame.group(name)?),
        None => None,
    };
    let mut out = vec![0u16; factor.len()];
    for segment in 0..frame.index().date_segments().len() {
        let range = frame
            .index()
            .date_segments()
            .range(segment)
            .expect("valid date segment");
        if let Some(groups) = group_values {
            let mut grouped: BTreeMap<GroupId, Vec<(usize, f64)>> = BTreeMap::new();
            for row in range {
                if factor[row].is_finite() {
                    grouped
                        .entry(groups[row])
                        .or_default()
                        .push((row, factor[row]));
                }
            }
            for rows in grouped.values() {
                if config.zero_aware {
                    assign_zero_aware(rows, config.quantiles, &mut out);
                } else {
                    assign_bucket_rows(rows, config.quantiles, &mut out);
                }
            }
        } else {
            let rows: Vec<(usize, f64)> = range
                .filter(|&row| factor[row].is_finite())
                .map(|row| (row, factor[row]))
                .collect();
            if config.zero_aware {
                assign_zero_aware(&rows, config.quantiles, &mut out);
            } else {
                assign_bucket_rows(&rows, config.quantiles, &mut out);
            }
        }
    }
    Ok(out)
}

/// Structured input/cleaning diagnostics used by every report surface.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DataQualityReport {
    pub input_rows: usize,
    pub finite_factor_rows: usize,
    pub missing_factor: usize,
    pub duplicate_keys: usize,
    pub date_count: usize,
    pub min_cross_section: usize,
    pub max_cross_section: usize,
    pub total_loss_ratio: f64,
}

pub fn data_quality(
    frame: &ResearchFrame,
    factor_column: &str,
) -> ResearchResult<DataQualityReport> {
    let factor = frame.column(factor_column)?;
    let input_rows = factor.len();
    let finite_factor_rows = factor.iter().filter(|v| v.is_finite()).count();
    let mut duplicate_keys = 0usize;
    let mut previous: Option<(i64, AssetId)> = None;
    for (&ts, &asset) in frame
        .index()
        .timestamps()
        .iter()
        .zip(frame.index().assets())
    {
        let key = (ts, asset);
        if previous == Some(key) {
            duplicate_keys += 1;
        }
        previous = Some(key);
    }
    let sizes: Vec<usize> = (0..frame.index().date_segments().len())
        .map(|segment| {
            let range = frame.index().date_segments().range(segment).unwrap();
            range.end - range.start
        })
        .collect();
    Ok(DataQualityReport {
        input_rows,
        finite_factor_rows,
        missing_factor: input_rows.saturating_sub(finite_factor_rows),
        duplicate_keys,
        date_count: sizes.len(),
        min_cross_section: sizes.iter().copied().min().unwrap_or(0),
        max_cross_section: sizes.iter().copied().max().unwrap_or(0),
        total_loss_ratio: if input_rows == 0 {
            0.0
        } else {
            1.0 - finite_factor_rows as f64 / input_rows as f64
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{AssetId, PanelIndex};

    fn sample_frame() -> ResearchFrame {
        let index = PanelIndex::new(
            vec![1, 1, 2, 2, 3, 3],
            vec![
                AssetId(1),
                AssetId(2),
                AssetId(1),
                AssetId(2),
                AssetId(1),
                AssetId(2),
            ],
        )
        .unwrap();
        let mut frame = ResearchFrame::new(index);
        frame
            .add_numeric("price", "market", vec![10.0, 20.0, 11.0, 18.0, 12.0, 21.0])
            .unwrap();
        frame
            .add_numeric("factor", "factor", vec![1.0, -1.0, 2.0, -2.0, 3.0, -3.0])
            .unwrap();
        frame
    }

    #[test]
    fn forward_returns_do_not_cross_assets() {
        let frame = sample_frame();
        let cfg = ForwardReturnConfig::new(vec![1], ReturnKind::Arithmetic).unwrap();
        let ret = compute_forward_returns(&frame, "price", &cfg).unwrap();
        assert!((ret[&1][0] - 0.1).abs() < 1e-12);
        assert!((ret[&1][1] + 0.1).abs() < 1e-12);
    }

    #[test]
    fn quantiles_are_date_local() {
        let frame = sample_frame();
        let q = quantize_factor(
            &frame,
            "factor",
            &QuantizeConfig {
                quantiles: 2,
                by_group: None,
                zero_aware: false,
            },
        )
        .unwrap();
        assert_eq!(&q[..2], &[2, 1]);
    }
}
