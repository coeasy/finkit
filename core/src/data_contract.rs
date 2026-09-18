//! Canonical multi-dimensional data contracts for the V1 compute engine.
//!
//! The indicator kernels still operate on aligned slices. This module defines
//! the containers that make instrument, timeframe, cross-sectional, and
//! point-in-time fundamental dimensions explicit before a plan reaches those
//! kernels. It intentionally does not fetch data or perform resampling.

use crate::runtime::MarketFrame;
use std::collections::BTreeMap;
use std::fmt;

/// Errors raised while constructing a canonical data view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataContractError {
    /// A required identifier was empty.
    EmptyIdentifier(&'static str),
    /// A frame key already exists in a panel.
    DuplicateFrame { symbol: String, timeframe: String },
    /// The embedded OHLCV frame is not column-aligned.
    InvalidMarketFrame(crate::runtime::RuntimeError),
    /// A matrix or table column has a different number of rows.
    LengthMismatch {
        /// Logical field name.
        field: &'static str,
        /// Expected length.
        expected: usize,
        /// Actual length.
        actual: usize,
    },
    /// A matrix dimension multiplication overflowed.
    DimensionOverflow,
    /// Identifiers in a single view must be unique.
    DuplicateIdentifier { kind: &'static str, value: String },
    /// Time points must be ordered for point-in-time lookup.
    NonMonotonicTimestamps { index: usize },
}

/// Policy used when a time-indexed input is projected onto another timeline.
///
/// The engine never guesses a resampling rule. Callers must choose whether
/// only exact timestamps are valid or whether the latest value whose timestamp
/// is at or before the target row may be carried forward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalAlignment {
    /// Use a value only when the source and target timestamps are identical.
    Exact,
    /// Use the latest source value at or before the target timestamp.
    ///
    /// This is the safe closed-bar/as-of rule for higher-timeframe inputs:
    /// values from a source bar that has not closed cannot enter an earlier
    /// target row.
    AsOfClosed,
}

/// One named time-indexed numeric input that can be aligned to a formula frame.
#[derive(Debug, Clone, Copy)]
pub struct TemporalSeries<'a> {
    /// Formula variable name supplied by the caller.
    pub name: &'a str,
    /// Monotonic source timestamps, normally bar-close or publication times.
    pub timestamps: &'a [i64],
    /// Values corresponding one-to-one with [`Self::timestamps`].
    pub values: &'a [f64],
}

impl<'a> TemporalSeries<'a> {
    /// Construct a validated named temporal series.
    pub fn new(
        name: &'a str,
        timestamps: &'a [i64],
        values: &'a [f64],
    ) -> Result<Self, DataContractError> {
        if name.trim().is_empty() {
            return Err(DataContractError::EmptyIdentifier("temporal series"));
        }
        if timestamps.len() != values.len() {
            return Err(DataContractError::LengthMismatch {
                field: "temporal_values",
                expected: timestamps.len(),
                actual: values.len(),
            });
        }
        validate_monotonic_timestamps(timestamps)?;
        Ok(Self {
            name,
            timestamps,
            values,
        })
    }

    /// Project this input onto `target_timestamps` using an explicit policy.
    pub fn align_to(
        &self,
        target_timestamps: &[i64],
        policy: TemporalAlignment,
    ) -> Result<Vec<f64>, DataContractError> {
        validate_monotonic_timestamps(target_timestamps)?;
        Ok(target_timestamps
            .iter()
            .map(|&target| match policy {
                TemporalAlignment::Exact => {
                    let end = self.timestamps.partition_point(|&source| source <= target);
                    end.checked_sub(1)
                        .filter(|&index| self.timestamps[index] == target)
                        .map(|index| self.values[index])
                        .unwrap_or(f64::NAN)
                }
                TemporalAlignment::AsOfClosed => {
                    let end = self.timestamps.partition_point(|&source| source <= target);
                    end.checked_sub(1)
                        .map(|index| self.values[index])
                        .unwrap_or(f64::NAN)
                }
            })
            .collect())
    }
}

fn validate_monotonic_timestamps(timestamps: &[i64]) -> Result<(), DataContractError> {
    for (index, pair) in timestamps.windows(2).enumerate() {
        if pair[1] < pair[0] {
            return Err(DataContractError::NonMonotonicTimestamps { index: index + 1 });
        }
    }
    Ok(())
}

impl fmt::Display for DataContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier(kind) => write!(f, "{kind} identifier must not be empty"),
            Self::DuplicateFrame { symbol, timeframe } => {
                write!(f, "duplicate market frame: {symbol}@{timeframe}")
            }
            Self::InvalidMarketFrame(error) => write!(f, "invalid market frame: {error}"),
            Self::LengthMismatch {
                field,
                expected,
                actual,
            } => write!(
                f,
                "{field} length mismatch: expected {expected}, got {actual}"
            ),
            Self::DimensionOverflow => write!(f, "data dimensions overflow usize"),
            Self::DuplicateIdentifier { kind, value } => {
                write!(f, "duplicate {kind} identifier: {value}")
            }
            Self::NonMonotonicTimestamps { index } => {
                write!(f, "timestamps are not monotonic at index {index}")
            }
        }
    }
}

impl std::error::Error for DataContractError {}

/// An explicit instrument/timeframe address for one aligned market frame.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameKey {
    /// Instrument identifier, for example `AAPL` or `600000.SS`.
    pub symbol: String,
    /// Canonical timeframe label, for example `1m`, `1d`, or `60s`.
    pub timeframe: String,
}

impl FrameKey {
    /// Construct a validated frame key.
    pub fn new(
        symbol: impl Into<String>,
        timeframe: impl Into<String>,
    ) -> Result<Self, DataContractError> {
        let symbol = symbol.into();
        let timeframe = timeframe.into();
        if symbol.trim().is_empty() {
            return Err(DataContractError::EmptyIdentifier("symbol"));
        }
        if timeframe.trim().is_empty() {
            return Err(DataContractError::EmptyIdentifier("timeframe"));
        }
        Ok(Self { symbol, timeframe })
    }
}

/// A borrowed collection of aligned frames across instruments and timeframes.
///
/// Frames are not implicitly resampled or joined. A compiled plan must select
/// a [`FrameKey`] and declare its alignment policy explicitly. This prevents a
/// formula from silently mixing symbols or using a higher-timeframe value
/// before that bar has closed.
#[derive(Debug, Default)]
pub struct MarketPanel<'a> {
    frames: BTreeMap<FrameKey, MarketFrame<'a>>,
}

impl<'a> MarketPanel<'a> {
    /// Create an empty multi-dimensional market panel.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert one validated instrument/timeframe frame.
    pub fn insert(
        &mut self,
        key: FrameKey,
        frame: MarketFrame<'a>,
    ) -> Result<(), DataContractError> {
        frame
            .validate()
            .map_err(DataContractError::InvalidMarketFrame)?;
        if self.frames.contains_key(&key) {
            return Err(DataContractError::DuplicateFrame {
                symbol: key.symbol,
                timeframe: key.timeframe,
            });
        }
        self.frames.insert(key, frame);
        Ok(())
    }

    /// Look up one instrument/timeframe frame without copying numeric data.
    pub fn get(&self, key: &FrameKey) -> Option<MarketFrame<'a>> {
        self.frames.get(key).copied()
    }

    /// Number of instrument/timeframe frames.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Whether the panel contains no frames.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Iterate over keys and zero-copy frame views in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&FrameKey, &MarketFrame<'a>)> {
        self.frames.iter()
    }

    /// Return unique symbols in deterministic order.
    pub fn symbols(&self) -> Vec<&str> {
        self.frames
            .keys()
            .map(|key| key.symbol.as_str())
            .fold(Vec::new(), |mut symbols, symbol| {
                if symbols.last().copied() != Some(symbol) {
                    symbols.push(symbol);
                }
                symbols
            })
    }

    /// Return unique timeframes in deterministic order.
    pub fn timeframes(&self) -> Vec<&str> {
        let mut timeframes: Vec<&str> = self
            .frames
            .keys()
            .map(|key| key.timeframe.as_str())
            .collect();
        timeframes.sort_unstable();
        timeframes.dedup();
        timeframes
    }
}

/// Row-major cross-sectional values aligned by timestamp and symbol.
///
/// `values[row * symbols.len() + column]` is the value for one symbol at one
/// timestamp. This is the canonical input for rank, z-score, winsorization,
/// neutralization, and other cross-sectional factor operations.
#[derive(Debug, Clone, Copy)]
pub struct CrossSectionView<'a> {
    /// Ordered timestamps, one row per observation date/time.
    pub timestamps: &'a [i64],
    /// Ordered symbols, one column per instrument.
    pub symbols: &'a [&'a str],
    /// Row-major numeric values.
    pub values: &'a [f64],
}

impl<'a> CrossSectionView<'a> {
    /// Construct a validated row-major cross-sectional view.
    pub fn new(
        timestamps: &'a [i64],
        symbols: &'a [&'a str],
        values: &'a [f64],
    ) -> Result<Self, DataContractError> {
        validate_monotonic_timestamps(timestamps)?;
        for (index, &symbol) in symbols.iter().enumerate() {
            if symbol.trim().is_empty() {
                return Err(DataContractError::EmptyIdentifier("symbol"));
            }
            if symbols[..index].contains(&symbol) {
                return Err(DataContractError::DuplicateIdentifier {
                    kind: "symbol",
                    value: symbol.to_string(),
                });
            }
        }
        let expected = timestamps
            .len()
            .checked_mul(symbols.len())
            .ok_or(DataContractError::DimensionOverflow)?;
        if values.len() != expected {
            return Err(DataContractError::LengthMismatch {
                field: "values",
                expected,
                actual: values.len(),
            });
        }
        Ok(Self {
            timestamps,
            symbols,
            values,
        })
    }

    /// Number of timestamp rows.
    pub const fn row_count(&self) -> usize {
        self.timestamps.len()
    }

    /// Number of instrument columns.
    pub const fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    /// Read one value by row and symbol column.
    pub fn value(&self, row: usize, column: usize) -> Option<f64> {
        if row >= self.row_count() || column >= self.symbol_count() {
            return None;
        }
        Some(self.values[row * self.symbol_count() + column])
    }

    /// Read one complete timestamp row without copying.
    pub fn row(&self, row: usize) -> Option<&'a [f64]> {
        if row >= self.row_count() {
            return None;
        }
        let start = row * self.symbol_count();
        Some(&self.values[start..start + self.symbol_count()])
    }
}

/// Point-in-time fundamental observations with as-of lookup semantics.
///
/// This is a data contract, not a fundamentals provider. Timestamps represent
/// publication/availability time, so a consumer can request the latest value
/// known at a market timestamp without silently using a future revision.
#[derive(Debug, Clone, Copy)]
pub struct FundamentalSeries<'a> {
    /// Fundamental field name, for example `book_value` or `earnings_ttm`.
    pub name: &'a str,
    /// Non-decreasing publication timestamps.
    pub timestamps: &'a [i64],
    /// Values corresponding to `timestamps`.
    pub values: &'a [f64],
}

impl<'a> FundamentalSeries<'a> {
    /// Construct a validated point-in-time fundamental series.
    pub fn new(
        name: &'a str,
        timestamps: &'a [i64],
        values: &'a [f64],
    ) -> Result<Self, DataContractError> {
        if name.trim().is_empty() {
            return Err(DataContractError::EmptyIdentifier("fundamental"));
        }
        if timestamps.len() != values.len() {
            return Err(DataContractError::LengthMismatch {
                field: "fundamental_values",
                expected: timestamps.len(),
                actual: values.len(),
            });
        }
        validate_monotonic_timestamps(timestamps)?;
        Ok(Self {
            name,
            timestamps,
            values,
        })
    }

    /// Return the most recently available value at or before `timestamp`.
    pub fn as_of(&self, timestamp: i64) -> Option<f64> {
        let end = self.timestamps.partition_point(|&point| point <= timestamp);
        end.checked_sub(1).map(|index| self.values[index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame<'a>(close: &'a [f64]) -> MarketFrame<'a> {
        MarketFrame::new(close, close, close, close, close).unwrap()
    }

    #[test]
    fn panel_keeps_symbol_and_timeframe_dimensions_explicit() {
        let daily = [1.0, 2.0];
        let minute = [1.0, 1.5];
        let mut panel = MarketPanel::new();
        panel
            .insert(FrameKey::new("AAA", "1d").unwrap(), frame(&daily))
            .unwrap();
        panel
            .insert(FrameKey::new("AAA", "1m").unwrap(), frame(&minute))
            .unwrap();
        assert_eq!(panel.len(), 2);
        assert_eq!(panel.symbols(), vec!["AAA"]);
        assert_eq!(panel.timeframes(), vec!["1d", "1m"]);
    }

    #[test]
    fn panel_rejects_duplicate_frame_key() {
        let values = [1.0];
        let key = FrameKey::new("AAA", "1d").unwrap();
        let mut panel = MarketPanel::new();
        panel.insert(key.clone(), frame(&values)).unwrap();
        assert!(matches!(
            panel.insert(key, frame(&values)),
            Err(DataContractError::DuplicateFrame { .. })
        ));
    }

    #[test]
    fn cross_section_reads_row_major_values() {
        let timestamps = [10, 20];
        let symbols = ["AAA", "BBB"];
        let values = [1.0, 2.0, 3.0, 4.0];
        let view = CrossSectionView::new(&timestamps, &symbols, &values).unwrap();
        assert_eq!(view.value(1, 0), Some(3.0));
        assert_eq!(view.row(0), Some(&values[..2]));
        assert_eq!(view.value(2, 0), None);
    }

    #[test]
    fn cross_section_rejects_shape_and_order_errors() {
        let timestamps = [20, 10];
        let symbols = ["AAA"];
        assert!(matches!(
            CrossSectionView::new(&timestamps, &symbols, &[1.0, 2.0]),
            Err(DataContractError::NonMonotonicTimestamps { .. })
        ));

        let timestamps = [10];
        assert!(matches!(
            CrossSectionView::new(&timestamps, &symbols, &[]),
            Err(DataContractError::LengthMismatch {
                field: "values",
                ..
            })
        ));
    }

    #[test]
    fn fundamental_as_of_never_reads_a_future_revision() {
        let timestamps = [10, 20, 20];
        let values = [1.0, 2.0, 2.5];
        let series = FundamentalSeries::new("earnings_ttm", &timestamps, &values).unwrap();
        assert_eq!(series.as_of(9), None);
        assert_eq!(series.as_of(19), Some(1.0));
        assert_eq!(series.as_of(20), Some(2.5));
    }

    #[test]
    fn temporal_series_supports_exact_and_closed_bar_alignment() {
        let source = TemporalSeries::new("daily_close", &[10, 20, 30], &[1.0, 2.0, 3.0]).unwrap();
        let exact = source
            .align_to(&[10, 15, 30], TemporalAlignment::Exact)
            .unwrap();
        assert_eq!(exact[0], 1.0);
        assert!(exact[1].is_nan());
        assert_eq!(exact[2], 3.0);

        let as_of = source
            .align_to(&[9, 15, 20, 29, 30], TemporalAlignment::AsOfClosed)
            .unwrap();
        assert!(as_of[0].is_nan());
        assert_eq!(&as_of[1..], &[1.0, 2.0, 2.0, 3.0]);
    }

    #[test]
    fn temporal_series_rejects_unordered_target_timeline() {
        let source = TemporalSeries::new("x", &[10, 20], &[1.0, 2.0]).unwrap();
        assert!(matches!(
            source.align_to(&[20, 10], TemporalAlignment::AsOfClosed),
            Err(DataContractError::NonMonotonicTimestamps { index: 1 })
        ));
    }

    #[test]
    fn temporal_exact_alignment_uses_latest_same_timestamp_revision() {
        let source = TemporalSeries::new("revised", &[10, 10, 20], &[1.0, 1.5, 2.0]).unwrap();
        assert_eq!(
            source.align_to(&[10], TemporalAlignment::Exact).unwrap(),
            &[1.5]
        );
    }
}
