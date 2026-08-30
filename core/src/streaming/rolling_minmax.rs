use std::collections::VecDeque;

/// O(1) amortized rolling maximum via monotonic deque.
///
/// Maintains decreasing order; the front is always the current maximum.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RollingMax {
    deque: VecDeque<(usize, f64)>,
}

impl RollingMax {
    /// Create a new empty `RollingMax`.
    pub fn new() -> Self {
        Self {
            deque: VecDeque::new(),
        }
    }

    /// Push a new value with its index.
    ///
    /// Maintains decreasing monotonic property by removing from the back
    /// all entries whose value is `<= val`.
    #[inline]
    pub fn push(&mut self, idx: usize, val: f64) {
        while let Some(&(_, back_val)) = self.deque.back() {
            if back_val <= val {
                self.deque.pop_back();
            } else {
                break;
            }
        }
        self.deque.push_back((idx, val));
    }

    /// Remove expired entries from the front whose index `<= idx`.
    #[inline]
    pub fn pop(&mut self, idx: usize) {
        while let Some(&(front_idx, _)) = self.deque.front() {
            if front_idx <= idx {
                self.deque.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get the current maximum value, or `None` if empty.
    #[inline]
    pub fn current(&self) -> Option<f64> {
        self.deque.front().map(|&(_, v)| v)
    }

    /// Get the current maximum entry (index, value), or `None` if empty.
    #[inline]
    pub fn front(&self) -> Option<(usize, f64)> {
        self.deque.front().copied()
    }

    /// Reset to empty state.
    #[inline]
    pub fn reset(&mut self) {
        self.deque.clear();
    }
}

/// O(1) amortized rolling minimum via monotonic deque.
///
/// Maintains increasing order; the front is always the current minimum.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RollingMin {
    deque: VecDeque<(usize, f64)>,
}

impl RollingMin {
    /// Create a new empty `RollingMin`.
    pub fn new() -> Self {
        Self {
            deque: VecDeque::new(),
        }
    }

    /// Push a new value with its index.
    ///
    /// Maintains increasing monotonic property by removing from the back
    /// all entries whose value is `>= val`.
    #[inline]
    pub fn push(&mut self, idx: usize, val: f64) {
        while let Some(&(_, back_val)) = self.deque.back() {
            if back_val >= val {
                self.deque.pop_back();
            } else {
                break;
            }
        }
        self.deque.push_back((idx, val));
    }

    /// Remove expired entries from the front whose index `<= idx`.
    #[inline]
    pub fn pop(&mut self, idx: usize) {
        while let Some(&(front_idx, _)) = self.deque.front() {
            if front_idx <= idx {
                self.deque.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get the current minimum value, or `None` if empty.
    #[inline]
    pub fn current(&self) -> Option<f64> {
        self.deque.front().map(|&(_, v)| v)
    }

    /// Get the current minimum entry (index, value), or `None` if empty.
    #[inline]
    pub fn front(&self) -> Option<(usize, f64)> {
        self.deque.front().copied()
    }

    /// Reset to empty state.
    #[inline]
    pub fn reset(&mut self) {
        self.deque.clear();
    }
}
