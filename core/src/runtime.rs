//! Shared runtime data contracts for batch and streaming computation.
//!
//! Existing indicator implementations remain free to consume slices directly.
//! These types provide a stable, zero-copy boundary for language bindings,
//! formula execution, factor computation, and future backend dispatch.

use std::borrow::Cow;
use std::fmt;

/// Missing-value handling requested by a calculation boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NanPolicy {
    /// Preserve missing values and let each function define propagation.
    #[default]
    Preserve,
    /// Treat a missing input as an error before execution.
    Error,
    /// Forward-fill missing values from the most recent finite observation.
    ForwardFill,
}

/// How warm-up rows are represented in public results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WarmupPolicy {
    /// Keep output length aligned and fill warm-up rows with NaN.
    #[default]
    Nan,
    /// Return only stable rows after the lookback.
    Trim,
}

/// Runtime validation errors for aligned market data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    /// A required market field does not match the canonical row count.
    LengthMismatch {
        /// Field name.
        field: &'static str,
        /// Expected row count.
        expected: usize,
        /// Actual row count.
        actual: usize,
    },
    /// A non-finite value was rejected by [`NanPolicy::Error`].
    NonFinite {
        /// Field name.
        field: &'static str,
        /// Row index.
        index: usize,
    },
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch {
                field,
                expected,
                actual,
            } => write!(
                f,
                "market frame length mismatch for {field}: expected {expected}, got {actual}"
            ),
            Self::NonFinite { field, index } => {
                write!(f, "non-finite value in {field} at row {index}")
            }
        }
    }
}

impl std::error::Error for RuntimeError {}

/// Borrowed numeric series with an explicit semantic name.
#[derive(Debug, Clone, Copy)]
pub struct SeriesView<'a> {
    /// Stable field or expression name.
    pub name: &'a str,
    /// Zero-copy numeric values.
    pub values: &'a [f64],
}

impl<'a> SeriesView<'a> {
    /// Build a zero-copy named series.
    pub const fn new(name: &'a str, values: &'a [f64]) -> Self {
        Self { name, values }
    }

    /// Number of rows.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the series contains no rows.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Apply a missing-value policy without allocating when the input can be
    /// borrowed as-is.
    pub fn normalized_cow(&self, policy: NanPolicy) -> Result<Cow<'a, [f64]>, RuntimeError> {
        match policy {
            NanPolicy::Preserve => Ok(Cow::Borrowed(self.values)),
            NanPolicy::Error => {
                if let Some(index) = self.values.iter().position(|value| !value.is_finite()) {
                    return Err(RuntimeError::NonFinite {
                        field: "series",
                        index,
                    });
                }
                Ok(Cow::Borrowed(self.values))
            }
            NanPolicy::ForwardFill => {
                if self.values.iter().all(|value| value.is_finite()) {
                    return Ok(Cow::Borrowed(self.values));
                }
                let mut output = Vec::with_capacity(self.values.len());
                let mut last = f64::NAN;
                for &value in self.values {
                    if value.is_finite() {
                        last = value;
                    }
                    output.push(last);
                }
                Ok(Cow::Owned(output))
            }
        }
    }

    /// Apply a missing-value policy and always return an owned buffer.
    ///
    /// Use [`Self::normalized_cow`] when callers can consume a borrowed slice.
    pub fn normalized(&self, policy: NanPolicy) -> Result<Vec<f64>, RuntimeError> {
        Ok(self.normalized_cow(policy)?.into_owned())
    }
}

/// Zero-copy aligned OHLCV(+amount,+timestamp) market frame.
#[derive(Debug, Clone, Copy)]
pub struct MarketFrame<'a> {
    /// Opening prices.
    pub open: &'a [f64],
    /// High prices.
    pub high: &'a [f64],
    /// Low prices.
    pub low: &'a [f64],
    /// Closing prices.
    pub close: &'a [f64],
    /// Trading volume.
    pub volume: &'a [f64],
    /// Optional turnover/amount series.
    pub amount: Option<&'a [f64]>,
    /// Optional epoch timestamp series.
    pub timestamp: Option<&'a [i64]>,
}

impl<'a> MarketFrame<'a> {
    /// Construct and validate an aligned frame.
    pub fn new(
        open: &'a [f64],
        high: &'a [f64],
        low: &'a [f64],
        close: &'a [f64],
        volume: &'a [f64],
    ) -> Result<Self, RuntimeError> {
        let frame = Self {
            open,
            high,
            low,
            close,
            volume,
            amount: None,
            timestamp: None,
        };
        frame.validate()?;
        Ok(frame)
    }

    /// Attach an optional turnover/amount series.
    pub fn with_amount(mut self, amount: &'a [f64]) -> Result<Self, RuntimeError> {
        self.amount = Some(amount);
        self.validate()?;
        Ok(self)
    }

    /// Attach optional epoch timestamps.
    pub fn with_timestamp(mut self, timestamp: &'a [i64]) -> Result<Self, RuntimeError> {
        self.timestamp = Some(timestamp);
        self.validate()?;
        Ok(self)
    }

    /// Canonical row count.
    pub fn len(&self) -> usize {
        self.close.len()
    }

    /// Whether the frame contains no bars.
    pub fn is_empty(&self) -> bool {
        self.close.is_empty()
    }

    /// Validate all columns against the close-series length.
    pub fn validate(&self) -> Result<(), RuntimeError> {
        let expected = self.close.len();
        validate_len("open", self.open.len(), expected)?;
        validate_len("high", self.high.len(), expected)?;
        validate_len("low", self.low.len(), expected)?;
        validate_len("volume", self.volume.len(), expected)?;
        if let Some(amount) = self.amount {
            validate_len("amount", amount.len(), expected)?;
        }
        if let Some(timestamp) = self.timestamp {
            validate_len("timestamp", timestamp.len(), expected)?;
        }
        Ok(())
    }

    /// Validate finite numeric market fields according to a NaN policy.
    pub fn validate_nan_policy(&self, policy: NanPolicy) -> Result<(), RuntimeError> {
        if policy != NanPolicy::Error {
            return Ok(());
        }
        for (field, values) in [
            ("open", self.open),
            ("high", self.high),
            ("low", self.low),
            ("close", self.close),
            ("volume", self.volume),
        ] {
            if let Some(index) = values.iter().position(|value| !value.is_finite()) {
                return Err(RuntimeError::NonFinite { field, index });
            }
        }
        if let Some(amount) = self.amount {
            if let Some(index) = amount.iter().position(|value| !value.is_finite()) {
                return Err(RuntimeError::NonFinite {
                    field: "amount",
                    index,
                });
            }
        }
        Ok(())
    }

    /// Resolve a standard field name using common terminal aliases.
    ///
    /// Matching is allocation-free so repeated formula/runtime lookups do not
    /// create a temporary uppercase string.
    pub fn series(&self, name: &str) -> Option<SeriesView<'a>> {
        let key = name.trim();
        if key.eq_ignore_ascii_case("O") || key.eq_ignore_ascii_case("OPEN") {
            Some(SeriesView::new("open", self.open))
        } else if key.eq_ignore_ascii_case("H") || key.eq_ignore_ascii_case("HIGH") {
            Some(SeriesView::new("high", self.high))
        } else if key.eq_ignore_ascii_case("L") || key.eq_ignore_ascii_case("LOW") {
            Some(SeriesView::new("low", self.low))
        } else if key.eq_ignore_ascii_case("C") || key.eq_ignore_ascii_case("CLOSE") {
            Some(SeriesView::new("close", self.close))
        } else if key.eq_ignore_ascii_case("V")
            || key.eq_ignore_ascii_case("VOL")
            || key.eq_ignore_ascii_case("VOLUME")
        {
            Some(SeriesView::new("volume", self.volume))
        } else if key.eq_ignore_ascii_case("AMOUNT") || key.eq_ignore_ascii_case("TURNOVER") {
            self.amount.map(|values| SeriesView::new("amount", values))
        } else {
            None
        }
    }
}

/// Apply a public warm-up policy to an aligned output.
pub fn apply_warmup(values: &[f64], lookback: usize, policy: WarmupPolicy) -> Vec<f64> {
    let start = lookback.min(values.len());
    match policy {
        WarmupPolicy::Nan => {
            let mut output = values.to_vec();
            for value in &mut output[..start] {
                *value = f64::NAN;
            }
            output
        }
        WarmupPolicy::Trim => values[start..].to_vec(),
    }
}

fn validate_len(field: &'static str, actual: usize, expected: usize) -> Result<(), RuntimeError> {
    if actual == expected {
        Ok(())
    } else {
        Err(RuntimeError::LengthMismatch {
            field,
            expected,
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_frame_rejects_unaligned_columns() {
        let error = MarketFrame::new(&[1.0], &[1.0, 2.0], &[1.0], &[1.0], &[1.0]).unwrap_err();
        assert!(matches!(
            error,
            RuntimeError::LengthMismatch { field: "high", .. }
        ));
    }

    #[test]
    fn terminal_aliases_resolve_without_copying() {
        let frame = MarketFrame::new(&[1.0], &[2.0], &[0.5], &[1.5], &[10.0]).unwrap();
        assert_eq!(frame.series("C").unwrap().values, &[1.5]);
        assert_eq!(frame.series("VOL").unwrap().values, &[10.0]);
    }

    #[test]
    fn preserve_policy_borrows_finite_input() {
        let input = [1.0, 2.0, 3.0];
        let series = SeriesView::new("close", &input);
        match series.normalized_cow(NanPolicy::Preserve).unwrap() {
            std::borrow::Cow::Borrowed(values) => assert_eq!(values.as_ptr(), input.as_ptr()),
            std::borrow::Cow::Owned(_) => panic!("preserve policy unexpectedly allocated"),
        }
    }

    #[test]
    fn forward_fill_preserves_leading_nan() {
        let series = SeriesView::new("close", &[f64::NAN, 1.0, f64::NAN, 2.0]);
        let output = series.normalized(NanPolicy::ForwardFill).unwrap();
        assert!(output[0].is_nan());
        assert_eq!(output[1..], [1.0, 1.0, 2.0]);
    }

    #[test]
    fn warmup_policy_supports_aligned_and_trimmed_outputs() {
        let input = [1.0, 2.0, 3.0, 4.0];
        let aligned = apply_warmup(&input, 2, WarmupPolicy::Nan);
        assert!(aligned[0].is_nan() && aligned[1].is_nan());
        assert_eq!(&aligned[2..], &[3.0, 4.0]);
        assert_eq!(apply_warmup(&input, 2, WarmupPolicy::Trim), vec![3.0, 4.0]);
    }
}
