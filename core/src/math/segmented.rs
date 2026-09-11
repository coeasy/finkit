//! Segment-layout primitives for date/group-oriented research kernels.

use std::ops::Range;

/// Contiguous segment offsets. Offsets always start at zero and include the final length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentLayout {
    offsets: Vec<usize>,
}

impl SegmentLayout {
    /// Build from explicit offsets.
    pub fn new(offsets: Vec<usize>) -> Result<Self, String> {
        if offsets.len() < 2 || offsets[0] != 0 || offsets.windows(2).any(|w| w[0] > w[1]) {
            return Err("segment offsets must start at zero and be monotonic".to_string());
        }
        Ok(Self { offsets })
    }

    /// Build contiguous segments from a sorted key slice.
    pub fn from_sorted_keys<T: PartialEq>(keys: &[T]) -> Self {
        if keys.is_empty() {
            return Self { offsets: vec![0, 0] };
        }
        let mut offsets = vec![0];
        for i in 1..keys.len() {
            if keys[i] != keys[i - 1] {
                offsets.push(i);
            }
        }
        offsets.push(keys.len());
        Self { offsets }
    }

    /// Number of segments.
    pub fn len(&self) -> usize {
        self.offsets.len().saturating_sub(1)
    }

    /// Whether there are no non-empty logical segments.
    pub fn is_empty(&self) -> bool {
        self.total_len() == 0
    }

    /// Total covered row count.
    pub fn total_len(&self) -> usize {
        *self.offsets.last().unwrap_or(&0)
    }

    /// Row range for one segment.
    pub fn range(&self, segment: usize) -> Option<Range<usize>> {
        (segment < self.len()).then(|| self.offsets[segment]..self.offsets[segment + 1])
    }

    /// Raw offsets for zero-copy consumers.
    pub fn offsets(&self) -> &[usize] {
        &self.offsets
    }

    /// Map segments in deterministic order.
    pub fn map<T>(&self, mut f: impl FnMut(Range<usize>) -> T) -> Vec<T> {
        (0..self.len())
            .filter_map(|segment| self.range(segment))
            .map(&mut f)
            .collect()
    }

    /// Parallel map when the `rayon` feature is enabled.
    #[cfg(feature = "rayon")]
    pub fn par_map<T: Send>(&self, f: impl Fn(Range<usize>) -> T + Sync + Send) -> Vec<T> {
        use rayon::prelude::*;
        (0..self.len())
            .into_par_iter()
            .map(|segment| f(self.range(segment).expect("segment index is valid")))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_ranges_from_sorted_keys() {
        let layout = SegmentLayout::from_sorted_keys(&[1, 1, 2, 2, 2, 5]);
        assert_eq!(layout.offsets(), &[0, 2, 5, 6]);
        assert_eq!(layout.range(1), Some(2..5));
    }
}
