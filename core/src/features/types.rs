//! Core types for the feature engineering module.

/// Metadata describing a single feature column.
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    /// Feature name following convention: `indicator_period_transform`
    pub name: String,
    /// Category (e.g., "momentum", "overlap", "volume", "volatility")
    pub category: String,
    /// Period parameter used (0 if not applicable)
    pub period: usize,
}

impl Feature {
    /// Create a new feature descriptor.
    pub fn new(name: impl Into<String>, category: impl Into<String>, period: usize) -> Self {
        Self {
            name: name.into(),
            category: category.into(),
            period,
        }
    }
}

/// Signal direction for crossover/threshold events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalDirection {
    /// Bullish signal (cross above / golden cross)
    Up,
    /// Bearish signal (cross below / death cross)
    Down,
}

/// A detected signal event (crossover, threshold cross, etc.)
#[derive(Debug, Clone)]
pub struct SignalEvent {
    /// Index in the data array where the signal occurred.
    pub index: usize,
    /// Direction of the signal.
    pub direction: SignalDirection,
    /// Signal strength (indicator-dependent, typically the magnitude of the cross).
    pub strength: f64,
}

/// Divergence type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivergenceType {
    /// Regular bullish divergence (price lower low, indicator higher low).
    RegularBullish,
    /// Regular bearish divergence (price higher high, indicator lower high).
    RegularBearish,
    /// Hidden bullish divergence (price higher low, indicator lower low).
    HiddenBullish,
    /// Hidden bearish divergence (price lower high, indicator higher high).
    HiddenBearish,
}

/// A detected divergence event between price and an indicator.
#[derive(Debug, Clone)]
pub struct DivergenceEvent {
    /// Start index of the divergence pattern.
    pub start_index: usize,
    /// End index of the divergence pattern.
    pub end_index: usize,
    /// Type of divergence.
    pub divergence_type: DivergenceType,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f64,
}

/// Label from triple barrier method.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarrierLabel {
    /// Label value: 1 (profit-taking), -1 (stop-loss), 0 (timeout).
    pub label: i8,
    /// Duration in bars until the barrier was hit.
    pub duration: usize,
    /// Return at exit.
    pub ret: f64,
}

/// Result of feature ranking/importance analysis.
#[derive(Debug, Clone)]
pub struct FeatureRanking {
    /// Feature names sorted by importance (descending).
    pub rankings: Vec<(String, f64)>,
}

impl FeatureRanking {
    /// Get the top-k most important features.
    pub fn top_k(&self, k: usize) -> &[(String, f64)] {
        &self.rankings[..k.min(self.rankings.len())]
    }

    /// Get features above a given importance threshold.
    pub fn above_threshold(&self, threshold: f64) -> Vec<&(String, f64)> {
        self.rankings
            .iter()
            .filter(|(_, v)| *v >= threshold)
            .collect()
    }
}
