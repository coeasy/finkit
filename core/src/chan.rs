//! Deterministic Chanlun (缠论) structure analysis.
//!
//! The module deliberately keeps the first implementation independent from
//! timestamps and external data-frame types.  A bar is identified by its
//! zero-based input index, which makes the result usable from Rust, Python,
//! WASM, and the visualization crate without converting date values.
//!
//! The pipeline follows the same useful decomposition as CZSC:
//!
//! `raw bars -> inclusion-free bars -> fractals -> strokes -> segments/centers`
//!
//! The exact definition of a stroke can vary between Chanlun schools.  The
//! default here is intentionally explicit and reproducible: strict three-bar
//! fractals, alternating fractals, and a configurable minimum distance between
//! stroke endpoints.  Applications that need a different school can change
//! [`ChanConfig`] without changing the result model.

use crate::error::{IndicatorError, Result};

/// Direction of a Chanlun stroke or segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChanDirection {
    /// From a bottom fractal towards a top fractal.
    Up,
    /// From a top fractal towards a bottom fractal.
    Down,
}

/// Type of a three-bar fractal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FractalKind {
    /// The middle bar is higher than both neighbours.
    Top,
    /// The middle bar is lower than both neighbours.
    Bottom,
}

/// Broad state of the latest confirmed Chanlun movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChanTrend {
    /// No confirmed stroke exists yet.
    Unknown,
    /// Latest movement is upward and outside the latest center.
    Bullish,
    /// Latest movement is downward and outside the latest center.
    Bearish,
    /// Latest endpoint remains inside the latest center.
    Range,
}

/// Common Chanlun buy/sell candidate categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChanSignalKind {
    /// Divergence-style first buy/sell candidate.
    Buy1,
    /// Center breakout/re-entry second buy/sell candidate.
    Buy2,
    /// Center pullback third buy/sell candidate.
    Buy3,
    /// Divergence-style first sell candidate.
    Sell1,
    /// Center breakout/re-entry second sell candidate.
    Sell2,
    /// Center pullback third sell candidate.
    Sell3,
}

/// Direction of a momentum divergence candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChanDivergenceKind {
    Bullish,
    Bearish,
}

/// A structural divergence candidate used as evidence for a B1/S1 signal.
#[derive(Debug, Clone, PartialEq)]
pub struct ChanDivergence {
    pub kind: ChanDivergenceKind,
    pub start_stroke: usize,
    pub end_stroke: usize,
    pub index: usize,
    pub price: f64,
    pub strength: f64,
    pub confirmed: bool,
    pub reason: String,
}

/// High-level preset for common Chanlun rule families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChanVariant {
    /// Larger stroke distance and stricter confirmation.
    Conservative,
    /// Six-bar configurable baseline.
    Standard,
    /// Shorter stroke distance and looser fractal interpretation.
    Aggressive,
}

/// Fractal detection policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FractalPolicy {
    /// Middle bar must exceed both neighbours in both high and low.
    Strict,
    /// Only the extreme high/low is required.
    Loose,
    /// Same as strict, with the final candidate kept separate.
    RightConfirmed,
}

/// Stroke endpoint policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokePolicy {
    /// Use [`ChanConfig::min_stroke_bars`].
    Configurable,
    /// Common five-bar variant.
    Fixed5,
    /// Common six-bar variant.
    Fixed6,
    /// Common seven-bar variant.
    Fixed7,
    /// Use the configured minimum bars plus price-change thresholds.
    Threshold,
}

/// Center construction policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterPolicy {
    /// One center begins with a three-stroke overlap.
    ThreeStroke,
    /// Extend the overlap while subsequent strokes remain inside it.
    Dynamic,
    /// Emit dynamic level-1 centers and one merged higher-level view.
    Hierarchical,
}

/// Numeric thresholds used by structure and signal policies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChanThresholds {
    /// Minimum absolute stroke change as a ratio of the start price.
    pub min_stroke_change_ratio: f64,
    /// Minimum fractal bar range as a ratio of its midpoint price.
    pub min_fractal_range_ratio: f64,
    /// Minimum signal strength in `[0, 1]`.
    pub signal_min_strength: f64,
    /// Relative distance required to regard a center edge as broken.
    pub center_break_ratio: f64,
}

impl Default for ChanThresholds {
    fn default() -> Self {
        Self {
            min_stroke_change_ratio: 0.0,
            min_fractal_range_ratio: 0.0,
            signal_min_strength: 0.0,
            center_break_ratio: 0.0,
        }
    }
}

impl ChanSignalKind {
    /// Whether this is a buy-side candidate.
    pub fn is_buy(self) -> bool {
        matches!(self, Self::Buy1 | Self::Buy2 | Self::Buy3)
    }
}

/// A structural signal candidate, not a guaranteed trading instruction.
#[derive(Debug, Clone, PartialEq)]
pub struct ChanSignal {
    /// Signal category.
    pub kind: ChanSignalKind,
    /// Raw-bar index where the candidate is emitted.
    pub index: usize,
    /// Endpoint price associated with the candidate.
    pub price: f64,
    /// Whether the candidate is based only on confirmed strokes.
    pub confirmed: bool,
    /// Normalized heuristic strength in `[0, 1]` when available.
    pub strength: f64,
    /// Human-readable reason for audit/debug output.
    pub reason: String,
    /// Raw indices supporting this candidate.
    pub evidence: Vec<usize>,
}

/// Lifecycle state derived from the evidence carried by a signal candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChanSignalState {
    Candidate,
    Confirmed,
}

impl ChanSignal {
    /// Returns the stable lifecycle state used by frontends and strategies.
    pub fn state(&self) -> ChanSignalState {
        if self.confirmed {
            ChanSignalState::Confirmed
        } else {
            ChanSignalState::Candidate
        }
    }
}

/// Configuration for Chanlun structure extraction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChanConfig {
    /// Minimum raw-bar index distance between the endpoints of a stroke.
    ///
    /// The default of six follows the commonly used CZSC default while still
    /// allowing small synthetic datasets to opt into a shorter distance.
    pub min_stroke_bars: usize,
    /// Maximum number of strokes retained in the result. Zero means unlimited.
    pub max_strokes: usize,
    /// Include a last, not-yet-confirmed two-bar fractal candidate.
    pub include_developing: bool,
    /// Emit conservative buy/sell candidates from confirmed structures.
    pub enable_signals: bool,
    /// High-level rule preset.
    pub variant: ChanVariant,
    /// Fractal policy.
    pub fractal_policy: FractalPolicy,
    /// Stroke policy.
    pub stroke_policy: StrokePolicy,
    /// Center policy.
    pub center_policy: CenterPolicy,
    /// Numeric extension thresholds.
    pub thresholds: ChanThresholds,
}

impl Default for ChanConfig {
    fn default() -> Self {
        Self {
            min_stroke_bars: 6,
            max_strokes: 0,
            include_developing: true,
            enable_signals: true,
            variant: ChanVariant::Standard,
            fractal_policy: FractalPolicy::Strict,
            stroke_policy: StrokePolicy::Configurable,
            center_policy: CenterPolicy::Dynamic,
            thresholds: ChanThresholds::default(),
        }
    }
}

impl ChanConfig {
    /// Apply a named preset while keeping explicit thresholds available.
    pub fn with_variant(mut self, variant: ChanVariant) -> Self {
        self.variant = variant;
        match variant {
            ChanVariant::Conservative => {
                self.stroke_policy = StrokePolicy::Fixed7;
                self.fractal_policy = FractalPolicy::Strict;
            }
            ChanVariant::Standard => {
                self.stroke_policy = StrokePolicy::Configurable;
                self.fractal_policy = FractalPolicy::Strict;
            }
            ChanVariant::Aggressive => {
                self.stroke_policy = StrokePolicy::Fixed5;
                self.fractal_policy = FractalPolicy::Loose;
            }
        }
        self
    }
}

/// A K line after inclusion relationships have been merged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChanBar {
    /// First raw-bar index represented by this merged bar.
    pub start_index: usize,
    /// Last raw-bar index represented by this merged bar.
    pub end_index: usize,
    /// Index of the raw bar contributing the merged high.
    pub high_index: usize,
    /// Index of the raw bar contributing the merged low.
    pub low_index: usize,
    /// Merged open price.
    pub open: f64,
    /// Merged high price.
    pub high: f64,
    /// Merged low price.
    pub low: f64,
    /// Merged close price.
    pub close: f64,
    /// Sum of the represented volumes.
    pub volume: f64,
}

/// A confirmed top or bottom fractal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fractal {
    /// Raw-bar index of the fractal extreme.
    pub index: usize,
    /// Fractal type.
    pub kind: FractalKind,
    /// High of the inclusion-free bar containing the fractal.
    pub high: f64,
    /// Low of the inclusion-free bar containing the fractal.
    pub low: f64,
    /// Price used to connect the fractal to a stroke.
    pub value: f64,
}

/// A stroke joining two alternating fractals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    /// Starting fractal.
    pub start: Fractal,
    /// Ending fractal.
    pub end: Fractal,
    /// Stroke direction.
    pub direction: ChanDirection,
    /// Highest price covered by the stroke endpoints.
    pub high: f64,
    /// Lowest price covered by the stroke endpoints.
    pub low: f64,
    /// Number of raw bars between the endpoints, inclusive.
    pub bars: usize,
    /// Endpoint price change (`end - start`).
    pub change: f64,
    /// Endpoint change per raw bar.
    pub slope: f64,
    /// Absolute endpoint change divided by endpoint range.
    pub strength: f64,
}

/// A conservative segment made from three alternating strokes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    /// Raw-bar index of the segment start.
    pub start_index: usize,
    /// Raw-bar index of the segment end.
    pub end_index: usize,
    /// Segment direction.
    pub direction: ChanDirection,
    /// Highest price covered by the segment.
    pub high: f64,
    /// Lowest price covered by the segment.
    pub low: f64,
    /// First stroke index in the analysis result.
    pub start_stroke: usize,
    /// Last stroke index in the analysis result.
    pub end_stroke: usize,
    /// Endpoint price change (`end - start`).
    pub change: f64,
}

/// A central trading-range overlap (中枢) made from at least three strokes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Center {
    /// Raw-bar index of the center start.
    pub start_index: usize,
    /// Raw-bar index of the center end.
    pub end_index: usize,
    /// Upper edge of the first three-stroke overlap.
    pub upper: f64,
    /// Lower edge of the first three-stroke overlap.
    pub lower: f64,
    /// Midline of the center.
    pub middle: f64,
    /// Highest price reached by all strokes in the center.
    pub high: f64,
    /// Lowest price reached by all strokes in the center.
    pub low: f64,
    /// First stroke index in the analysis result.
    pub start_stroke: usize,
    /// Last stroke index in the analysis result.
    pub end_stroke: usize,
    /// Structural level; level 1 is the source stroke overlap and higher
    /// values are recursively merged overlaps when hierarchical mode is on.
    pub level: usize,
}

/// Complete Chanlun analysis result for one OHLCV series.
#[derive(Debug, Clone, PartialEq)]
pub struct ChanAnalysis {
    /// Inclusion-free bars used by the structural stages.
    pub bars: Vec<ChanBar>,
    /// Confirmed alternating fractals.
    pub fractals: Vec<Fractal>,
    /// Strokes built from the fractals.
    pub strokes: Vec<Stroke>,
    /// Conservative three-stroke segments.
    pub segments: Vec<Segment>,
    /// Extended three-stroke overlap centers.
    pub centers: Vec<Center>,
    /// Last two-bar fractal candidate, if the current bar has not confirmed it.
    pub developing_fractal: Option<Fractal>,
    /// Broad latest-trend state.
    pub trend: ChanTrend,
    /// Conservative buy/sell candidates derived from the confirmed structures.
    pub signals: Vec<ChanSignal>,
    /// Divergence candidates separated from executable strategy signals.
    pub divergences: Vec<ChanDivergence>,
}

impl ChanAnalysis {
    /// Returns true when no structure was detected.
    pub fn is_empty(&self) -> bool {
        self.fractals.is_empty() && self.strokes.is_empty()
    }

    /// Return the most recent signal candidate, if any.
    pub fn latest_signal(&self) -> Option<&ChanSignal> {
        self.signals.last()
    }

    /// Return the most recent divergence candidate, if any.
    pub fn latest_divergence(&self) -> Option<&ChanDivergence> {
        self.divergences.last()
    }

    /// Validate ordering, bounds, and price invariants before a result is
    /// handed to a renderer or strategy.
    pub fn validate(&self) -> Result<()> {
        for pair in self.fractals.windows(2) {
            if pair[0].index >= pair[1].index {
                return Err(IndicatorError::InvalidParameter {
                    param: "fractals".to_string(),
                    reason: "fractal indices must be strictly increasing".to_string(),
                }
                .into());
            }
        }
        for stroke in &self.strokes {
            if stroke.start.index >= stroke.end.index || stroke.bars == 0 {
                return Err(IndicatorError::InvalidParameter {
                    param: "strokes".to_string(),
                    reason: "stroke endpoints must be ordered and non-empty".to_string(),
                }
                .into());
            }
        }
        for center in &self.centers {
            if center.start_index > center.end_index || center.lower > center.upper {
                return Err(IndicatorError::InvalidParameter {
                    param: "centers".to_string(),
                    reason: "center ranges must be ordered with lower <= upper".to_string(),
                }
                .into());
            }
        }
        for signal in &self.signals {
            if signal.evidence.iter().any(|index| *index > signal.index) {
                return Err(IndicatorError::InvalidParameter {
                    param: "signals".to_string(),
                    reason: "signal evidence cannot point after the signal".to_string(),
                }
                .into());
            }
        }
        Ok(())
    }
}

/// Stateful convenience wrapper for incremental data ingestion.
///
/// The wrapper keeps input ownership and recomputes a deterministic snapshot
/// on demand. `append` is the low-latency ingestion path and does not run the
/// structural pipeline; callers can choose when to refresh with
/// `snapshot_cached`. `push` remains the convenience API for one-bar/one-
/// snapshot workflows, while `extend` is preferred for backfills and replay.
#[derive(Debug, Clone)]
pub struct ChanAnalyzer {
    config: ChanConfig,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    cached: Option<ChanAnalysis>,
}

impl ChanAnalyzer {
    /// Creates an empty analyzer with the supplied configuration.
    pub fn new(config: ChanConfig) -> Self {
        Self {
            config,
            open: Vec::new(),
            high: Vec::new(),
            low: Vec::new(),
            close: Vec::new(),
            volume: Vec::new(),
            cached: None,
        }
    }

    /// Return the active rule configuration.
    pub fn config(&self) -> ChanConfig {
        self.config
    }

    /// Replace the rule configuration and invalidate the cached snapshot.
    pub fn set_config(&mut self, config: ChanConfig) {
        self.config = config;
        self.cached = None;
    }

    /// Append one bar without running the structural pipeline.
    ///
    /// This is the low-latency ingestion path for live feeds. Call
    /// [`Self::snapshot_cached`] at the desired refresh cadence instead of
    /// recalculating Chan structures for every quote/tick.
    pub fn append(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Result<()> {
        validate_bar(open, high, low, close, volume, self.close.len())?;
        self.open.push(open);
        self.high.push(high);
        self.low.push(low);
        self.close.push(close);
        self.volume.push(volume);
        self.cached = None;
        Ok(())
    }

    /// Appends one bar and returns the latest complete snapshot.
    pub fn push(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Result<ChanAnalysis> {
        self.append(open, high, low, close, volume)?;
        let analysis = self.snapshot()?;
        self.cached = Some(analysis.clone());
        Ok(analysis)
    }

    /// Append a batch of bars and analyze once.
    ///
    /// This is the preferred ingestion API for backfills and replay. It
    /// avoids running the full deterministic pipeline once per historical bar
    /// while retaining the same output contract as [`Self::push`].
    pub fn extend(
        &mut self,
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        volume: &[f64],
    ) -> Result<ChanAnalysis> {
        if open.len() != high.len()
            || open.len() != low.len()
            || open.len() != close.len()
            || open.len() != volume.len()
        {
            return Err(IndicatorError::InvalidParameter {
                param: "ohlcv".to_string(),
                reason: "batch arrays must have equal lengths".to_string(),
            }
            .into());
        }
        for index in 0..open.len() {
            validate_bar(
                open[index],
                high[index],
                low[index],
                close[index],
                volume[index],
                self.close.len() + index,
            )?;
        }
        self.open.extend_from_slice(open);
        self.high.extend_from_slice(high);
        self.low.extend_from_slice(low);
        self.close.extend_from_slice(close);
        self.volume.extend_from_slice(volume);
        self.cached = None;
        let analysis = self.snapshot()?;
        self.cached = Some(analysis.clone());
        Ok(analysis)
    }

    /// Recomputes a snapshot from all bars received so far.
    pub fn snapshot(&self) -> Result<ChanAnalysis> {
        if let Some(analysis) = &self.cached {
            return Ok(analysis.clone());
        }
        analyze(
            &self.open,
            &self.high,
            &self.low,
            &self.close,
            &self.volume,
            self.config,
        )
    }

    /// Reuse the most recent deterministic snapshot when no new bar arrived.
    /// A future local-tail implementation can replace the cache fill without
    /// changing this consumer-facing API.
    pub fn snapshot_cached(&mut self) -> Result<ChanAnalysis> {
        if let Some(analysis) = &self.cached {
            return Ok(analysis.clone());
        }
        let analysis = self.snapshot()?;
        self.cached = Some(analysis.clone());
        Ok(analysis)
    }

    /// Returns the number of raw bars received.
    pub fn len(&self) -> usize {
        self.close.len()
    }

    /// Returns true when no raw bars have been received.
    pub fn is_empty(&self) -> bool {
        self.close.is_empty()
    }
}

fn validate_bar(
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    index: usize,
) -> Result<()> {
    if [open, high, low, close, volume]
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(IndicatorError::NanPropagation {
            indicator: format!("chan at index {index}"),
        }
        .into());
    }
    if high < low {
        return Err(IndicatorError::InvalidParameter {
            param: "high/low".to_string(),
            reason: format!("high must be >= low at index {index}"),
        }
        .into());
    }
    Ok(())
}

/// Analyzes an OHLCV series and extracts Chanlun structures.
pub fn analyze(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    config: ChanConfig,
) -> Result<ChanAnalysis> {
    validate_config(config)?;
    let n = close.len();
    if open.len() != n || high.len() != n || low.len() != n || volume.len() != n {
        return Err(IndicatorError::InvalidParameter {
            param: "ohlcv".to_string(),
            reason: "open, high, low, close and volume must have equal lengths".to_string(),
        }
        .into());
    }
    if n < 3 {
        return Err(IndicatorError::InsufficientData {
            required: 3,
            actual: n,
        }
        .into());
    }
    for (index, values) in (0..n).map(|i| (i, [open[i], high[i], low[i], close[i], volume[i]])) {
        if values.iter().any(|value| !value.is_finite()) {
            return Err(IndicatorError::NanPropagation {
                indicator: format!("chan at index {index}"),
            }
            .into());
        }
        if high[index] < low[index] {
            return Err(IndicatorError::InvalidParameter {
                param: "high/low".to_string(),
                reason: format!("high must be >= low at index {index}"),
            }
            .into());
        }
    }

    let bars = merge_inclusion(open, high, low, close, volume);
    let fractals = find_fractals(&bars, config);
    let strokes = build_strokes(&fractals, config);
    let segments = build_segments(&strokes);
    let centers = build_centers(&strokes, config.center_policy);
    let developing_fractal = if config.include_developing {
        find_developing_fractal(&bars, &fractals)
    } else {
        None
    };
    let trend = infer_trend(&strokes, &centers);
    let signals = if config.enable_signals {
        build_signals(&strokes, &centers, config.thresholds)
    } else {
        Vec::new()
    };
    let divergences = build_divergences(&strokes, config.thresholds);

    let result = ChanAnalysis {
        bars,
        fractals,
        strokes,
        segments,
        centers,
        developing_fractal,
        trend,
        signals,
        divergences,
    };
    result.validate()?;
    Ok(result)
}

fn validate_config(config: ChanConfig) -> Result<()> {
    let thresholds = config.thresholds;
    if !thresholds.min_stroke_change_ratio.is_finite()
        || thresholds.min_stroke_change_ratio < 0.0
        || !thresholds.min_fractal_range_ratio.is_finite()
        || thresholds.min_fractal_range_ratio < 0.0
        || !thresholds.signal_min_strength.is_finite()
        || !(0.0..=1.0).contains(&thresholds.signal_min_strength)
        || !thresholds.center_break_ratio.is_finite()
        || thresholds.center_break_ratio < 0.0
    {
        return Err(IndicatorError::InvalidParameter {
            param: "thresholds".to_string(),
            reason:
                "thresholds must be finite, non-negative, and signal_min_strength must be in [0, 1]"
                    .to_string(),
        }
        .into());
    }
    Ok(())
}

fn merge_inclusion(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
) -> Vec<ChanBar> {
    let mut bars = Vec::with_capacity(close.len());
    for index in 0..close.len() {
        let next = ChanBar {
            start_index: index,
            end_index: index,
            high_index: index,
            low_index: index,
            open: open[index],
            high: high[index],
            low: low[index],
            close: close[index],
            volume: volume[index],
        };
        let Some(previous) = bars.last_mut() else {
            bars.push(next);
            continue;
        };

        let included = (previous.high <= next.high && previous.low >= next.low)
            || (previous.high >= next.high && previous.low <= next.low);
        if !included {
            bars.push(next);
            continue;
        }

        let upward = if next.high != previous.high {
            next.high > previous.high
        } else {
            next.close >= previous.close
        };
        previous.start_index = previous.start_index.min(next.start_index);
        previous.end_index = next.end_index;
        previous.close = next.close;
        previous.volume += next.volume;
        if upward {
            if next.high >= previous.high {
                previous.high = next.high;
                previous.high_index = next.high_index;
            }
            if next.low >= previous.low {
                previous.low = next.low;
                previous.low_index = next.low_index;
            }
        } else {
            if next.high <= previous.high {
                previous.high = next.high;
                previous.high_index = next.high_index;
            }
            if next.low <= previous.low {
                previous.low = next.low;
                previous.low_index = next.low_index;
            }
        }
    }
    bars
}

fn find_fractals(bars: &[ChanBar], config: ChanConfig) -> Vec<Fractal> {
    let mut fractals: Vec<Fractal> = Vec::new();
    for window in bars.windows(3) {
        let [left, middle, right] = window else {
            continue;
        };
        let strict_top = middle.high > left.high
            && middle.high > right.high
            && middle.low > left.low
            && middle.low > right.low;
        let strict_bottom = middle.low < left.low
            && middle.low < right.low
            && middle.high < left.high
            && middle.high < right.high;
        let top = match config.fractal_policy {
            FractalPolicy::Loose => middle.high > left.high && middle.high > right.high,
            FractalPolicy::Strict | FractalPolicy::RightConfirmed => strict_top,
        };
        let bottom = match config.fractal_policy {
            FractalPolicy::Loose => middle.low < left.low && middle.low < right.low,
            FractalPolicy::Strict | FractalPolicy::RightConfirmed => strict_bottom,
        };
        let candidate = if top
            && (middle.high - middle.low)
                / ((middle.high + middle.low).abs() * 0.5).max(f64::EPSILON)
                >= config.thresholds.min_fractal_range_ratio
        {
            Some(Fractal {
                index: middle.high_index,
                kind: FractalKind::Top,
                high: middle.high,
                low: middle.low,
                value: middle.high,
            })
        } else if bottom {
            Some(Fractal {
                index: middle.low_index,
                kind: FractalKind::Bottom,
                high: middle.high,
                low: middle.low,
                value: middle.low,
            })
        } else {
            None
        };

        if let Some(candidate) = candidate {
            if let Some(last) = fractals.last_mut() {
                if last.kind == candidate.kind {
                    let more_extreme = match candidate.kind {
                        FractalKind::Top => candidate.value > last.value,
                        FractalKind::Bottom => candidate.value < last.value,
                    };
                    if more_extreme {
                        *last = candidate;
                    }
                    continue;
                }
            }
            fractals.push(candidate);
        }
    }
    fractals
}

fn build_strokes(fractals: &[Fractal], config: ChanConfig) -> Vec<Stroke> {
    let mut selected: Vec<Fractal> = Vec::with_capacity(fractals.len());
    for &candidate in fractals {
        let Some(last) = selected.last_mut() else {
            selected.push(candidate);
            continue;
        };
        if last.kind == candidate.kind {
            let more_extreme = match candidate.kind {
                FractalKind::Top => candidate.value > last.value,
                FractalKind::Bottom => candidate.value < last.value,
            };
            if more_extreme {
                *last = candidate;
            }
        } else if candidate.index.saturating_sub(last.index) >= required_stroke_bars(config)
            && (candidate.value - last.value).abs() / last.value.abs().max(f64::EPSILON)
                >= config.thresholds.min_stroke_change_ratio
        {
            selected.push(candidate);
        }
    }

    let mut strokes = Vec::with_capacity(selected.len().saturating_sub(1));
    for pair in selected.windows(2) {
        let [start, end] = pair else { continue };
        let direction = match (start.kind, end.kind) {
            (FractalKind::Bottom, FractalKind::Top) => ChanDirection::Up,
            (FractalKind::Top, FractalKind::Bottom) => ChanDirection::Down,
            _ => continue,
        };
        strokes.push(Stroke {
            start: *start,
            end: *end,
            direction,
            high: start.high.max(end.high),
            low: start.low.min(end.low),
            bars: end.index.saturating_sub(start.index) + 1,
            change: end.value - start.value,
            slope: (end.value - start.value) / end.index.saturating_sub(start.index).max(1) as f64,
            strength: if start.high.max(end.high) > start.low.min(end.low) {
                (end.value - start.value).abs()
                    / (start.high.max(end.high) - start.low.min(end.low))
            } else {
                0.0
            },
        });
    }
    if config.max_strokes > 0 && strokes.len() > config.max_strokes {
        let start = strokes.len() - config.max_strokes;
        strokes.drain(..start);
    }
    strokes
}

fn required_stroke_bars(config: ChanConfig) -> usize {
    match config.stroke_policy {
        StrokePolicy::Configurable | StrokePolicy::Threshold => config.min_stroke_bars.max(1),
        StrokePolicy::Fixed5 => 5,
        StrokePolicy::Fixed6 => 6,
        StrokePolicy::Fixed7 => 7,
    }
}

fn build_segments(strokes: &[Stroke]) -> Vec<Segment> {
    let mut segments = Vec::with_capacity(strokes.len() / 2);
    let mut index = 0;
    while index + 2 < strokes.len() {
        let window = &strokes[index..index + 3];
        segments.push(Segment {
            start_index: window[0].start.index,
            end_index: window[2].end.index,
            direction: window[0].direction,
            high: window
                .iter()
                .map(|stroke| stroke.high)
                .fold(f64::NEG_INFINITY, f64::max),
            low: window
                .iter()
                .map(|stroke| stroke.low)
                .fold(f64::INFINITY, f64::min),
            start_stroke: index,
            end_stroke: index + 2,
            change: window[2].end.value - window[0].start.value,
        });
        index += 2;
    }
    segments
}

fn build_centers(strokes: &[Stroke], policy: CenterPolicy) -> Vec<Center> {
    let mut centers = Vec::new();
    let mut index = 0;
    while index + 2 < strokes.len() {
        let first_three = &strokes[index..index + 3];
        let upper = first_three
            .iter()
            .map(|stroke| stroke.high)
            .fold(f64::INFINITY, f64::min);
        let lower = first_three
            .iter()
            .map(|stroke| stroke.low)
            .fold(f64::NEG_INFINITY, f64::max);
        if lower > upper {
            index += 1;
            continue;
        }

        let mut end_stroke = index + 2;
        if policy != CenterPolicy::ThreeStroke {
            while end_stroke + 1 < strokes.len() {
                let next = strokes[end_stroke + 1];
                if next.high < lower || next.low > upper {
                    break;
                }
                end_stroke += 1;
            }
        }
        let range = &strokes[index..=end_stroke];
        centers.push(Center {
            start_index: range[0].start.index,
            end_index: range[range.len() - 1].end.index,
            upper,
            lower,
            middle: (upper + lower) * 0.5,
            high: range
                .iter()
                .map(|stroke| stroke.high)
                .fold(f64::NEG_INFINITY, f64::max),
            low: range
                .iter()
                .map(|stroke| stroke.low)
                .fold(f64::INFINITY, f64::min),
            start_stroke: index,
            end_stroke,
            level: 1,
        });
        index = end_stroke + 1;
    }
    if policy == CenterPolicy::Hierarchical {
        let mut previous_level = centers.clone();
        let max_levels = strokes.len().saturating_sub(2).min(16);
        for level in 2..=max_levels {
            let merged = merge_center_level(&previous_level, level);
            if merged.is_empty() {
                break;
            }
            centers.extend(merged.iter().copied());
            previous_level = merged;
        }
    }
    centers.sort_by_key(|center| (center.start_index, center.end_index, center.level));
    centers
}

/// Merge adjacent centers into the next structural level while their price
/// intervals keep a non-empty intersection. The sweep consumes each merged
/// group once, which prevents a long chain from producing duplicate parents
/// with indistinguishable evidence ranges.
fn merge_center_level(previous: &[Center], level: usize) -> Vec<Center> {
    let mut merged = Vec::new();
    let mut index = 0;
    while index + 1 < previous.len() {
        let mut end = index + 1;
        let mut upper = previous[index].upper.min(previous[end].upper);
        let mut lower = previous[index].lower.max(previous[end].lower);
        if lower > upper {
            index += 1;
            continue;
        }
        while end + 1 < previous.len() {
            let next = previous[end + 1];
            let next_upper = upper.min(next.upper);
            let next_lower = lower.max(next.lower);
            if next_lower > next_upper {
                break;
            }
            upper = next_upper;
            lower = next_lower;
            end += 1;
        }
        let group = &previous[index..=end];
        merged.push(Center {
            start_index: group[0].start_index,
            end_index: group[group.len() - 1].end_index,
            upper,
            lower,
            middle: (upper + lower) * 0.5,
            high: group
                .iter()
                .map(|center| center.high)
                .fold(f64::NEG_INFINITY, f64::max),
            low: group
                .iter()
                .map(|center| center.low)
                .fold(f64::INFINITY, f64::min),
            start_stroke: group[0].start_stroke,
            end_stroke: group[group.len() - 1].end_stroke,
            level,
        });
        index = end + 1;
    }
    merged
}

fn find_developing_fractal(bars: &[ChanBar], confirmed: &[Fractal]) -> Option<Fractal> {
    let [left, right] = bars.get(bars.len().saturating_sub(2)..)?.try_into().ok()?;
    let candidate = if left.high > right.high && left.low > right.low {
        Fractal {
            index: left.high_index,
            kind: FractalKind::Top,
            high: left.high,
            low: left.low,
            value: left.high,
        }
    } else if left.low < right.low && left.high < right.high {
        Fractal {
            index: left.low_index,
            kind: FractalKind::Bottom,
            high: left.high,
            low: left.low,
            value: left.low,
        }
    } else {
        return None;
    };
    if confirmed
        .iter()
        .any(|fractal| fractal.index == candidate.index)
    {
        None
    } else {
        Some(candidate)
    }
}

fn infer_trend(strokes: &[Stroke], centers: &[Center]) -> ChanTrend {
    let Some(last) = strokes.last() else {
        return ChanTrend::Unknown;
    };
    if let Some(center) = centers
        .iter()
        .max_by_key(|center| (center.end_index, center.level))
    {
        if (center.lower..=center.upper).contains(&last.end.value) {
            return ChanTrend::Range;
        }
    }
    match last.direction {
        ChanDirection::Up => ChanTrend::Bullish,
        ChanDirection::Down => ChanTrend::Bearish,
    }
}

fn build_signals(
    strokes: &[Stroke],
    centers: &[Center],
    thresholds: ChanThresholds,
) -> Vec<ChanSignal> {
    let mut signals = Vec::new();
    for index in 2..strokes.len() {
        let previous = strokes[index - 2];
        let current = strokes[index];
        if previous.direction == ChanDirection::Down
            && current.direction == ChanDirection::Down
            && current.end.value >= previous.end.value
            && current.strength <= previous.strength
        {
            let strength =
                (1.0 - current.strength / previous.strength.max(f64::EPSILON)).clamp(0.0, 1.0);
            if strength < thresholds.signal_min_strength {
                continue;
            }
            signals.push(ChanSignal {
                kind: ChanSignalKind::Buy1,
                index: current.end.index,
                price: current.end.value,
                confirmed: true,
                strength,
                reason: "相邻下跌笔出现低点不创新低且力度衰减".to_string(),
                evidence: vec![
                    previous.start.index,
                    previous.end.index,
                    current.start.index,
                    current.end.index,
                ],
            });
        } else if previous.direction == ChanDirection::Up
            && current.direction == ChanDirection::Up
            && current.end.value <= previous.end.value
            && current.strength <= previous.strength
        {
            let strength =
                (1.0 - current.strength / previous.strength.max(f64::EPSILON)).clamp(0.0, 1.0);
            if strength < thresholds.signal_min_strength {
                continue;
            }
            signals.push(ChanSignal {
                kind: ChanSignalKind::Sell1,
                index: current.end.index,
                price: current.end.value,
                confirmed: true,
                strength,
                reason: "相邻上涨笔出现高点不创新高且力度衰减".to_string(),
                evidence: vec![
                    previous.start.index,
                    previous.end.index,
                    current.start.index,
                    current.end.index,
                ],
            });
        }
    }

    for center in centers {
        for (index, stroke) in strokes.iter().enumerate().skip(center.end_stroke + 1) {
            let upper_break = center.upper + center.upper.abs() * thresholds.center_break_ratio;
            let lower_break = center.lower - center.lower.abs() * thresholds.center_break_ratio;
            if stroke.direction == ChanDirection::Up && stroke.end.value > upper_break {
                signals.push(ChanSignal {
                    kind: ChanSignalKind::Buy2,
                    index: stroke.end.index,
                    price: stroke.end.value,
                    confirmed: true,
                    strength: stroke.strength.min(1.0),
                    reason: format!("向上离开第{}级中枢上沿", center.level),
                    evidence: vec![center.start_index, center.end_index, stroke.end.index],
                });
                if let Some(retrace) = strokes.get(index + 1) {
                    if retrace.direction == ChanDirection::Down
                        && (center.lower..=center.upper).contains(&retrace.end.value)
                    {
                        signals.push(ChanSignal {
                            kind: ChanSignalKind::Buy3,
                            index: retrace.end.index,
                            price: retrace.end.value,
                            confirmed: true,
                            strength: retrace.strength.min(1.0),
                            reason: "向上离开中枢后的回抽重新进入中枢".to_string(),
                            evidence: vec![
                                center.start_index,
                                center.end_index,
                                stroke.end.index,
                                retrace.end.index,
                            ],
                        });
                    }
                }
                break;
            }
            if stroke.direction == ChanDirection::Down && stroke.end.value < lower_break {
                signals.push(ChanSignal {
                    kind: ChanSignalKind::Sell2,
                    index: stroke.end.index,
                    price: stroke.end.value,
                    confirmed: true,
                    strength: stroke.strength.min(1.0),
                    reason: format!("向下离开第{}级中枢下沿", center.level),
                    evidence: vec![center.start_index, center.end_index, stroke.end.index],
                });
                if let Some(retrace) = strokes.get(index + 1) {
                    if retrace.direction == ChanDirection::Up
                        && (center.lower..=center.upper).contains(&retrace.end.value)
                    {
                        signals.push(ChanSignal {
                            kind: ChanSignalKind::Sell3,
                            index: retrace.end.index,
                            price: retrace.end.value,
                            confirmed: true,
                            strength: retrace.strength.min(1.0),
                            reason: "向下离开中枢后的回抽重新进入中枢".to_string(),
                            evidence: vec![
                                center.start_index,
                                center.end_index,
                                stroke.end.index,
                                retrace.end.index,
                            ],
                        });
                    }
                }
                break;
            }
        }
    }
    signals.sort_by_key(|signal| (signal.index, signal.kind as u8));
    signals.dedup_by(|left, right| left.index == right.index && left.kind == right.kind);
    signals
}

fn build_divergences(strokes: &[Stroke], thresholds: ChanThresholds) -> Vec<ChanDivergence> {
    let mut divergences = Vec::new();
    for index in 2..strokes.len() {
        let previous = strokes[index - 2];
        let current = strokes[index];
        let (kind, price_improved, reason) = if previous.direction == ChanDirection::Down
            && current.direction == ChanDirection::Down
            && current.end.value >= previous.end.value
        {
            (
                ChanDivergenceKind::Bullish,
                current.strength < previous.strength,
                "下跌笔价格不创新低但力度减弱",
            )
        } else if previous.direction == ChanDirection::Up
            && current.direction == ChanDirection::Up
            && current.end.value <= previous.end.value
        {
            (
                ChanDivergenceKind::Bearish,
                current.strength < previous.strength,
                "上涨笔价格不创新高但力度减弱",
            )
        } else {
            continue;
        };
        if !price_improved {
            continue;
        }
        let strength =
            (1.0 - current.strength / previous.strength.max(f64::EPSILON)).clamp(0.0, 1.0);
        if strength < thresholds.signal_min_strength {
            continue;
        }
        divergences.push(ChanDivergence {
            kind,
            start_stroke: index - 2,
            end_stroke: index,
            index: current.end.index,
            price: current.end.value,
            strength,
            confirmed: true,
            reason: reason.to_string(),
        });
    }
    divergences
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ([f64; 18], [f64; 18], [f64; 18], [f64; 18], [f64; 18]) {
        let close = [
            10., 11., 14., 13., 12., 9., 8., 10., 13., 12., 11., 7., 6., 9., 12., 11., 10., 8.,
        ];
        let open = close.map(|value| value - 0.2);
        let high = close.map(|value| value + 0.5);
        let low = close.map(|value| value - 0.5);
        let volume = [1.; 18];
        (open, high, low, close, volume)
    }

    #[test]
    fn detects_fractals_and_strokes() {
        let (open, high, low, close, volume) = sample();
        let result = analyze(
            &open,
            &high,
            &low,
            &close,
            &volume,
            ChanConfig {
                min_stroke_bars: 2,
                max_strokes: 0,
                include_developing: true,
                enable_signals: true,
                ..ChanConfig::default()
            },
        )
        .expect("valid sample");
        assert!(result.fractals.len() >= 3);
        assert!(result.strokes.len() >= 2);
        assert!(result
            .strokes
            .windows(2)
            .all(|pair| pair[0].direction != pair[1].direction));
        result.validate().expect("generated structure is valid");
    }

    #[test]
    fn analyzer_reuses_cached_snapshot_until_new_data_arrives() {
        let (open, high, low, close, volume) = sample();
        let mut analyzer = ChanAnalyzer::new(ChanConfig {
            min_stroke_bars: 2,
            ..ChanConfig::default()
        });
        for index in 0..close.len() {
            let result = analyzer.push(
                open[index],
                high[index],
                low[index],
                close[index],
                volume[index],
            );
            if index >= 2 {
                result.expect("valid streaming bar");
            }
        }
        let first = analyzer.snapshot_cached().expect("cached snapshot");
        let second = analyzer.snapshot_cached().expect("same cached snapshot");
        assert_eq!(first, second);
        assert_eq!(second.bars.len(), first.bars.len());
    }

    #[test]
    fn analyzer_extend_matches_batch_analysis() {
        let (open, high, low, close, volume) = sample();
        let config = ChanConfig {
            min_stroke_bars: 2,
            ..ChanConfig::default()
        };
        let expected =
            analyze(&open, &high, &low, &close, &volume, config).expect("batch analysis");
        let mut analyzer = ChanAnalyzer::new(config);
        let actual = analyzer
            .extend(&open, &high, &low, &close, &volume)
            .expect("batch append analysis");
        assert_eq!(actual, expected);
    }

    #[test]
    fn analyzer_deferred_append_matches_batched_snapshot() {
        let (open, high, low, close, volume) = sample();
        let config = ChanConfig {
            min_stroke_bars: 2,
            ..ChanConfig::default()
        };
        let expected =
            analyze(&open, &high, &low, &close, &volume, config).expect("batch analysis");
        let mut analyzer = ChanAnalyzer::new(config);
        for index in 0..close.len() {
            analyzer
                .append(
                    open[index],
                    high[index],
                    low[index],
                    close[index],
                    volume[index],
                )
                .expect("deferred append");
        }
        assert_eq!(
            analyzer.snapshot_cached().expect("deferred snapshot"),
            expected
        );
        assert_eq!(
            analyzer
                .snapshot_cached()
                .expect("cached deferred snapshot"),
            expected
        );
    }

    #[test]
    fn rejects_invalid_signal_threshold() {
        let (_, _, _, close, volume) = sample();
        let error = analyze(
            &close,
            &close,
            &close,
            &close,
            &volume,
            ChanConfig {
                thresholds: ChanThresholds {
                    signal_min_strength: 1.1,
                    ..ChanThresholds::default()
                },
                ..ChanConfig::default()
            },
        )
        .expect_err("invalid strength must be rejected");
        assert!(matches!(
            error,
            crate::error::TaError::Indicator(IndicatorError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn merges_inclusion_without_losing_index_span() {
        let open = [10., 10.5, 11.];
        let high = [12., 11.5, 13.];
        let low = [8., 8.5, 9.];
        let close = [11., 11., 12.];
        let volume = [1., 2., 3.];
        let result = analyze(&open, &high, &low, &close, &volume, ChanConfig::default())
            .expect("valid sample");
        assert_eq!(result.bars.len(), 2);
        assert_eq!(result.bars[0].start_index, 0);
        assert_eq!(result.bars[0].end_index, 1);
        assert_eq!(result.bars[0].volume, 3.0);
    }

    #[test]
    fn rejects_mismatched_and_non_finite_input() {
        let err = analyze(
            &[1., 1.],
            &[1., 1., 1.],
            &[0., 0., 0.],
            &[1., 2., 3.],
            &[1., 1., 1.],
            ChanConfig::default(),
        )
        .expect_err("mismatched lengths must fail");
        assert!(matches!(
            err,
            crate::error::TaError::Indicator(IndicatorError::InvalidParameter { .. })
        ));
        let err = analyze(
            &[1., 1., 1.],
            &[1., 1., f64::NAN],
            &[0., 0., 0.],
            &[1., 1., 1.],
            &[1., 1., 1.],
            ChanConfig::default(),
        )
        .expect_err("NaN must fail");
        assert!(matches!(
            err,
            crate::error::TaError::Indicator(IndicatorError::NanPropagation { .. })
        ));
    }

    fn synthetic_strokes() -> Vec<Stroke> {
        let ranges = [
            (8.0, 14.0),
            (9.0, 15.0),
            (10.0, 14.0),
            (10.5, 13.5),
            (10.5, 14.0),
            (15.0, 16.0),
            (10.0, 14.0),
            (11.0, 13.5),
            (10.5, 14.0),
        ];
        ranges
            .into_iter()
            .enumerate()
            .map(|(index, (low, high))| {
                let direction = if index % 2 == 0 {
                    ChanDirection::Up
                } else {
                    ChanDirection::Down
                };
                let start_kind = if direction == ChanDirection::Up {
                    FractalKind::Bottom
                } else {
                    FractalKind::Top
                };
                Stroke {
                    start: Fractal {
                        index: index * 2,
                        kind: start_kind,
                        high,
                        low,
                        value: if start_kind == FractalKind::Top {
                            high
                        } else {
                            low
                        },
                    },
                    end: Fractal {
                        index: index * 2 + 1,
                        kind: if start_kind == FractalKind::Top {
                            FractalKind::Bottom
                        } else {
                            FractalKind::Top
                        },
                        high,
                        low,
                        value: if start_kind == FractalKind::Top {
                            low
                        } else {
                            high
                        },
                    },
                    direction,
                    high,
                    low,
                    bars: 2,
                    change: high - low,
                    slope: 1.0,
                    strength: 0.5,
                }
            })
            .collect()
    }

    #[test]
    fn center_policies_distinguish_three_stroke_dynamic_and_hierarchical() {
        let strokes = synthetic_strokes();
        let three = build_centers(&strokes, CenterPolicy::ThreeStroke);
        let dynamic = build_centers(&strokes, CenterPolicy::Dynamic);
        let hierarchical = build_centers(&strokes, CenterPolicy::Hierarchical);

        assert_eq!(three[0].end_stroke, three[0].start_stroke + 2);
        assert!(dynamic[0].end_stroke > dynamic[0].start_stroke + 2);
        assert!(hierarchical.iter().any(|center| center.level == 2));
        assert!(hierarchical
            .windows(2)
            .all(|pair| pair[0].start_index <= pair[1].start_index));
    }
}
