//! Declarative macros for streaming indicator boilerplate reduction.
//!
//! These macros eliminate the repetitive code that every streaming indicator
//! must implement: metadata and standard trait methods. (The repaint-support
//! macros were removed: they had zero call sites — every indicator hand-writes
//! `compute_bar` — and uninvoked `macro_rules!` are invisible to the
//! dead-code gate. `scripts/check_unused_macros.py` now guards against a
//! recurrence.)

/// Generate the [`IndicatorMeta`](crate::streaming::IndicatorMeta) implementation.
///
/// The indicator struct must have a `period: usize` field (used by `warm_up_period`).
///
/// # Example
///
/// ```ignore
/// impl_indicator_meta!(StreamingSma, "SMA", "overlap", "Simple Moving Average");
/// ```
#[macro_export]
macro_rules! impl_indicator_meta {
    ($type:ty, $name:expr, $category:expr, $desc:expr) => {
        impl $crate::streaming::IndicatorMeta for $type {
            #[inline]
            fn name() -> &'static str {
                $name
            }
            #[inline]
            fn category() -> &'static str {
                debug_assert!(
                    $crate::streaming::registry::VALID_CATEGORIES.contains(&$category),
                    "invalid streaming indicator category slug: {}",
                    $category
                );
                $category
            }
            #[inline]
            fn description() -> &'static str {
                $desc
            }
            #[inline]
            fn warm_up_period(&self) -> usize {
                self.period
            }
        }
    };
}

/// Generate the standard `count()` and `value()` trait methods.
///
/// Every streaming indicator stores `count: usize` and `last_value: Option<T>`
/// and returns them from the trait. This macro eliminates that repetition.
///
/// # Example
///
/// ```ignore
/// impl StreamingIndicator for StreamingFoo {
///     fn next(&mut self, input: f64) -> Option<f64> { /* ... */ }
///     fn reset(&mut self) { /* ... */ }
///     fn is_ready(&self) -> bool { /* ... */ }
///     impl_standard_methods!();
/// }
/// ```
#[macro_export]
macro_rules! impl_standard_methods {
    () => {
        #[inline]
        fn count(&self) -> usize {
            self.count
        }
        #[inline]
        fn value(&self) -> Option<f64> {
            self.last_value
        }
    };
    (output = $output_type:ty) => {
        #[inline]
        fn count(&self) -> usize {
            self.count
        }
        #[inline]
        fn value(&self) -> Option<$output_type> {
            self.last_value
        }
    };
}
// ===========================================================================
// Test macros
// ===========================================================================

/// Generate a standard `test_streaming_<name>_meta()` test function.
///
/// # Example
///
/// ```ignore
/// #[test]
/// fn test_streaming_sma_meta() {
///     test_streaming_meta!(StreamingSma, 10, "SMA", "overlap", 10);
/// }
/// ```
#[macro_export]
macro_rules! test_streaming_meta {
    ($type:ty, $period:expr, $name:expr, $category:expr, $warmup:expr) => {
        let ind = <$type>::new($period);
        assert_eq!(<$type>::name(), $name);
        assert_eq!(<$type>::category(), $category);
        assert_eq!(ind.warm_up_period(), $warmup);
    };
}

/// Generate a standard `test_streaming_<name>_reset()` test function.
///
/// Feeds `n` sequential `i` values (as `f64`) to the indicator, then
/// verifies `reset()` returns it to the initial state.
///
/// # Example
///
/// ```ignore
/// #[test]
/// fn test_streaming_sma_reset() {
///     test_streaming_reset!(StreamingSma, 3, 10, |ind: &mut StreamingSma, i| { ind.next(i); });
/// }
/// ```
#[macro_export]
macro_rules! test_streaming_reset {
    ($type:ty, $period:expr, $n:expr, $feed:expr) => {
        let mut ind = <$type>::new($period);
        for i in 0..$n {
            let i_f = i as f64;
            $feed(&mut ind, i_f);
        }
        assert!(ind.is_ready());
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.count(), 0);
    };
}

/// Generate a standard `test_streaming_vs_batch_convergence()` test.
///
/// Generates 100 sinusoidal data points, computes batch results, and
/// compares them point-by-point against the streaming indicator.
///
/// # Example
///
/// ```ignore
/// #[test]
/// fn test_streaming_vs_batch_convergence() {
///     test_streaming_vs_batch!(StreamingSma, 14, |data, period| {
///         crate::math::moving_avg::sma(data, period).unwrap()
///     });
/// }
/// ```
#[macro_export]
macro_rules! test_streaming_vs_batch {
    ($type:ty, $period:expr, $batch_fn:expr) => {
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let batch_result = $batch_fn(&data, $period);

        let mut streaming = <$type>::new($period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch_result[i].is_nan()) {
                assert!(
                    (s - batch_result[i]).abs() < 1e-10,
                    "Mismatch at index {i}: streaming={s}, batch={}",
                    batch_result[i]
                );
            }
        }
    };
}
