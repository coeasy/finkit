use crate::data::AssetId;
use finkit::returns::{return_between, ReturnKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Copy)]
struct PendingPrice {
    date_index: usize,
    price: f64,
}

/// A forward return that became fully observable on the latest appended session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaturedForwardReturn {
    pub origin_date_index: usize,
    pub asset: AssetId,
    pub period: usize,
    pub value: f64,
}

/// Incremental forward-return maturity tracker.
#[derive(Debug, Clone)]
pub struct IncrementalForwardReturnEngine {
    periods: Vec<usize>,
    kind: ReturnKind,
    next_date_index: usize,
    history: BTreeMap<AssetId, VecDeque<PendingPrice>>,
}

impl IncrementalForwardReturnEngine {
    pub fn new(mut periods: Vec<usize>, kind: ReturnKind) -> Self {
        periods.retain(|period| *period > 0);
        periods.sort_unstable();
        periods.dedup();
        Self {
            periods,
            kind,
            next_date_index: 0,
            history: BTreeMap::new(),
        }
    }

    /// Append one complete/partial cross-sectional session. Only fully matured horizons are emitted.
    pub fn append_session(&mut self, prices: &[(AssetId, f64)]) -> Vec<MaturedForwardReturn> {
        let date_index = self.next_date_index;
        self.next_date_index += 1;
        let max_period = self.periods.iter().copied().max().unwrap_or(0);
        let mut matured = Vec::new();
        for &(asset, price) in prices {
            let queue = self.history.entry(asset).or_default();
            for pending in queue.iter() {
                let age = date_index.saturating_sub(pending.date_index);
                if self.periods.binary_search(&age).is_ok() {
                    matured.push(MaturedForwardReturn {
                        origin_date_index: pending.date_index,
                        asset,
                        period: age,
                        value: return_between(pending.price, price, self.kind),
                    });
                }
            }
            queue.push_back(PendingPrice { date_index, price });
            while queue
                .front()
                .is_some_and(|item| date_index.saturating_sub(item.date_index) >= max_period)
            {
                queue.pop_front();
            }
        }
        matured
    }

    pub fn pending_assets(&self) -> usize {
        self.history.len()
    }
    pub fn date_count(&self) -> usize {
        self.next_date_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizons_emit_only_when_mature() {
        let mut engine = IncrementalForwardReturnEngine::new(vec![2], ReturnKind::Arithmetic);
        assert!(engine.append_session(&[(AssetId(1), 10.0)]).is_empty());
        assert!(engine.append_session(&[(AssetId(1), 11.0)]).is_empty());
        let matured = engine.append_session(&[(AssetId(1), 12.0)]);
        assert_eq!(matured.len(), 1);
        assert_eq!(matured[0].origin_date_index, 0);
        assert!((matured[0].value - 0.2).abs() < 1e-12);
    }
}
