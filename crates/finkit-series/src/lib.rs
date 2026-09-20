//! Quant series model for the Finkit factor engine.
//!
//! # Partially adopted -- do not add public API
//!
//! Part of the unconverged `crates/finkit-*` migration track. [`QuantSeries`]
//! carries one capability `core` lacks: a symbol tag plus strict
//! alignment/monotonicity validation. That is being merged into `core` as an
//! **input-side** type (phase R2 of
//! `docs/runtime-carrier-adoption-plan-2026-09-20.md`); everything else here is
//! scheduled for removal.
//!
//! Note the warm-up semantics deliberately do **not** transfer: this crate's
//! outputs are warm-up *trimmed* (an `SMA(3)` over 10 points yields 8 values),
//! whereas `core`'s `runtime::WarmupPolicy` preserves length and fills. Core
//! wins; do not carry the trimming semantics across.

use finkit_array::FloatArray;

#[derive(Clone, Debug)]
pub struct QuantSeries {
    pub symbol: String,
    pub timestamps: Vec<i64>,
    pub values: FloatArray,
}

impl QuantSeries {
    pub fn new(symbol: impl Into<String>, timestamps: Vec<i64>, values: FloatArray) -> Self {
        Self {
            symbol: symbol.into(),
            timestamps,
            values,
        }
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn timestamps(&self) -> &[i64] {
        &self.timestamps
    }

    pub fn values(&self) -> &[f64] {
        self.values.as_slice()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Whether values and timestamps form a strictly ordered aligned series.
    pub fn is_valid(&self) -> bool {
        self.timestamps.len() == self.values.len()
            && self
                .timestamps
                .windows(2)
                .all(|window| window[0] < window[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finkit_array::FloatArray;

    #[test]
    fn validates_alignment_and_order() {
        assert!(QuantSeries::new("A", vec![1, 2], FloatArray::new(vec![1.0, 2.0])).is_valid());
        assert!(!QuantSeries::new("A", vec![2, 1], FloatArray::new(vec![1.0, 2.0])).is_valid());
        assert!(!QuantSeries::new("A", vec![1], FloatArray::new(vec![1.0, 2.0])).is_valid());
    }
}
