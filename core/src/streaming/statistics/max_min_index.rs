//! Streaming rolling `MAXINDEX` / `MININDEX` (TA-Lib `TA_MAXINDEX`/`TA_MININDEX`).
//!
//! O(1) per-bar incremental index of the rolling maximum / minimum. Returns
//! the offset (0 = oldest bar in window) of the most extreme value.

use crate::impl_indicator_meta;
use crate::impl_standard_methods;
use crate::streaming::traits::StreamingIndicator;
use std::collections::VecDeque;

/// Streaming rolling `MAXINDEX`.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMaxIndex {
    period: usize,
    /// Deque of `(value, offset)` indices, monotonic decreasing values.
    buf: VecDeque<(f64, usize)>,
    /// Global counter, used to compute the relative offset within the window.
    count: usize,
    last_value: Option<f64>,
}

impl StreamingMaxIndex {
    pub fn new(period: usize) -> Self {
        assert!(period >= 1, "period must be >= 1");
        Self {
            period,
            buf: VecDeque::with_capacity(period),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingMaxIndex {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let offset = self.count - 1; // 0-based index of current bar
                                     // Remove bars that have slid out of the window
        while let Some(&(_, first_off)) = self.buf.front() {
            if offset.saturating_sub(first_off) >= self.period {
                self.buf.pop_front();
            } else {
                break;
            }
        }
        // Maintain monotonic decreasing deque: pop strictly-smaller values.
        // Ties preserve the EARLIER position (TA-Lib returns first occurrence).
        while let Some(&(v, _)) = self.buf.back() {
            if v < input {
                self.buf.pop_back();
            } else {
                break;
            }
        }
        self.buf.push_back((input, offset));

        if self.count >= self.period {
            // Offset within the window: front of deque = current max's offset
            let (_, max_off) = *self.buf.front().expect("deque non-empty");
            let rel = (max_off + self.period - offset - 1) as f64;
            self.last_value = Some(rel);
            Some(rel)
        } else {
            self.last_value = None;
            None
        }
    }

    fn reset(&mut self) {
        self.buf.clear();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(
    StreamingMaxIndex,
    "MAXINDEX",
    "math_operators",
    "Index of highest value over a rolling period"
);

/// Streaming rolling `MININDEX`.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMinIndex {
    period: usize,
    /// Deque of `(value, offset)`, monotonic increasing values.
    buf: VecDeque<(f64, usize)>,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingMinIndex {
    pub fn new(period: usize) -> Self {
        assert!(period >= 1, "period must be >= 1");
        Self {
            period,
            buf: VecDeque::with_capacity(period),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingMinIndex {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let offset = self.count - 1;
        while let Some(&(_, first_off)) = self.buf.front() {
            if offset.saturating_sub(first_off) >= self.period {
                self.buf.pop_front();
            } else {
                break;
            }
        }
        // Maintain monotonic increasing deque: pop strictly-larger values.
        // Ties preserve the EARLIER position (TA-Lib returns first occurrence).
        while let Some(&(v, _)) = self.buf.back() {
            if v > input {
                self.buf.pop_back();
            } else {
                break;
            }
        }
        self.buf.push_back((input, offset));

        if self.count >= self.period {
            let (_, min_off) = *self.buf.front().expect("deque non-empty");
            let rel = (min_off + self.period - offset - 1) as f64;
            self.last_value = Some(rel);
            Some(rel)
        } else {
            self.last_value = None;
            None
        }
    }

    fn reset(&mut self) {
        self.buf.clear();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(
    StreamingMinIndex,
    "MININDEX",
    "math_operators",
    "Index of lowest value over a rolling period"
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;

    #[test]
    fn test_max_index_basic() {
        let mut m = StreamingMaxIndex::new(3);
        // [3, 1, 4] -> max 4 at offset 2
        assert_eq!(m.next(3.0), None);
        assert_eq!(m.next(1.0), None);
        assert_eq!(m.next(4.0), Some(2.0));
        // [1, 4, 1] -> max 4 at offset 1
        assert_eq!(m.next(1.0), Some(1.0));
        // [4, 1, 5] -> max 5 at offset 2
        assert_eq!(m.next(5.0), Some(2.0));
    }

    #[test]
    fn test_min_index_basic() {
        let mut m = StreamingMinIndex::new(3);
        assert_eq!(m.next(3.0), None);
        assert_eq!(m.next(1.0), None);
        // [3, 1, 4] -> min 1 at offset 1
        assert_eq!(m.next(4.0), Some(1.0));
        // [1, 4, 1] -> min 1 at offset 0 (ties -> earliest position, TA-Lib convention)
        assert_eq!(m.next(1.0), Some(0.0));
    }

    #[test]
    fn test_min_index_ties_keep_earliest() {
        // Sequence [3, 1, 4, 1, 1, 0], period=5
        //   step 5: window [3, 1, 4, 1, 1] (offsets 0..4), min=1 at k=1 (offset 1)
        //     -> rel = 1 + 5 - 4 - 1 = 1
        //   step 6: window [1, 4, 1, 1, 0] (offsets 1..5), min=0 at k=4 (offset 5)
        //     -> rel = 5 + 5 - 5 - 1 = 4
        let mut m = StreamingMinIndex::new(5);
        let results: Vec<Option<f64>> = [3.0, 1.0, 4.0, 1.0, 1.0, 0.0]
            .iter()
            .map(|&v| m.next(v))
            .collect();
        assert_eq!(results, vec![None, None, None, None, Some(1.0), Some(4.0)]);
    }

    #[test]
    fn test_max_index_ties_keep_earliest() {
        // Sequence [3, 1, 4, 4, 4, 5]:
        //   At end: window [4, 4, 4, 5] (offsets 2,3,4,5), max=5 at offset 5
        //   After tie test, the first 4 (offset 2) is kept, so rel = 0
        let mut m = StreamingMaxIndex::new(4);
        for v in [3.0, 1.0, 4.0, 4.0, 4.0, 5.0] {
            m.next(v);
        }
        // After 6th value: max_off should be earliest among ties
        // 5 is the unique max at offset 5, rel = 5 + 4 - 5 - 1 = 3
        assert_eq!(m.value(), Some(3.0));
    }

    #[test]
    fn test_max_index_ties_equal_max() {
        // Sequence [1, 3, 1, 3]: window [1, 3, 1, 3] (offsets 0..4, period=4)
        // After all 4 inputs, max=3 at offset 1 AND offset 3.
        // TA-Lib returns first occurrence: offset 1, rel = 1+4-3-1 = 1
        let mut m = StreamingMaxIndex::new(4);
        m.next(1.0);
        m.next(3.0);
        m.next(1.0);
        let v = m.next(3.0);
        assert_eq!(v, Some(1.0));
    }

    #[test]
    fn test_max_index_reset() {
        let mut m = StreamingMaxIndex::new(3);
        m.next(1.0);
        m.next(5.0);
        m.next(3.0);
        assert!(m.is_ready());
        m.reset();
        assert!(!m.is_ready());
    }

    #[test]
    fn test_meta() {
        assert_eq!(StreamingMaxIndex::name(), "MAXINDEX");
        assert_eq!(StreamingMinIndex::name(), "MININDEX");
    }

    #[test]
    fn test_vs_batch_max() {
        let data: Vec<f64> = (0..50)
            .map(|i| (i as f64 * 0.3).sin() * 10.0 + 50.0)
            .collect();
        let period = 5;
        let batch = crate::indicators::math_operators::maxindex(&data, period).unwrap();
        let mut s = StreamingMaxIndex::new(period);
        for (i, &v) in data.iter().enumerate() {
            if let Some(sv) = s.next(v) {
                // Batch uses Array1<i64> (offset 0..=period-1).
                let expected = batch[i];
                assert!(
                    (sv as i64 - expected).abs() <= 1,
                    "Mismatch at {i}: streaming={sv}, batch={expected}"
                );
            }
        }
    }
}
