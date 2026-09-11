use crate::data::{AssetId, ResearchFrame};
use crate::error::ResearchResult;
use finkit::returns::{return_between, ReturnKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One event-aligned return path around a factor/event row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventPath {
    pub event_row: usize,
    pub relative_returns: Vec<f64>,
    pub event_offset: usize,
}

/// Build common-start event paths for each requested event row.
pub fn common_start_returns(
    frame: &ResearchFrame,
    price_column: &str,
    event_rows: &[usize],
    before: usize,
    after: usize,
) -> ResearchResult<Vec<EventPath>> {
    let prices = frame.column(price_column)?;
    let mut by_asset: BTreeMap<AssetId, Vec<usize>> = BTreeMap::new();
    for (row, &asset) in frame.index().assets().iter().enumerate() {
        by_asset.entry(asset).or_default().push(row);
    }
    let local_position: BTreeMap<usize, usize> = by_asset.values().flat_map(|rows| rows.iter().enumerate().map(|(local, &row)| (row, local))).collect();
    let mut paths = Vec::new();
    for &event_row in event_rows {
        if event_row >= frame.index().len() { continue; }
        let asset = frame.index().assets()[event_row];
        let rows = &by_asset[&asset];
        let local = local_position[&event_row];
        let start = local.saturating_sub(before);
        let end = (local + after).min(rows.len().saturating_sub(1));
        let base = prices[event_row];
        let relative_returns = rows[start..=end]
            .iter()
            .map(|&row| return_between(base, prices[row], ReturnKind::Arithmetic))
            .collect();
        paths.push(EventPath {
            event_row,
            relative_returns,
            event_offset: local - start,
        });
    }
    Ok(paths)
}

/// Mean event path after aligning all paths at their event timestamp.
pub fn average_event_path(paths: &[EventPath], before: usize, after: usize) -> Vec<f64> {
    let width = before + after + 1;
    let mut sums = vec![0.0; width];
    let mut counts = vec![0usize; width];
    for path in paths {
        for (local, &value) in path.relative_returns.iter().enumerate() {
            if !value.is_finite() { continue; }
            let aligned = before as isize + local as isize - path.event_offset as isize;
            if aligned >= 0 && (aligned as usize) < width {
                sums[aligned as usize] += value;
                counts[aligned as usize] += 1;
            }
        }
    }
    sums.into_iter().zip(counts).map(|(sum, count)| if count == 0 { f64::NAN } else { sum / count as f64 }).collect()
}
