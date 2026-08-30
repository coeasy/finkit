//! Declarative macros for streaming indicator boilerplate reduction.
//!
//! These macros eliminate the repetitive code that every streaming indicator
//! must implement: metadata, standard trait methods, and repaint support.

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
            fn name() -> &'static str { $name }
            #[inline]
            fn category() -> &'static str { $category }
            #[inline]
            fn description() -> &'static str { $desc }
            #[inline]
            fn warm_up_period(&self) -> usize { self.period }
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
        fn count(&self) -> usize { self.count }
        #[inline]
        fn value(&self) -> Option<f64> { self.last_value }
    };
    (output = $output_type:ty) => {
        #[inline]
        fn count(&self) -> usize { self.count }
        #[inline]
        fn value(&self) -> Option<$output_type> { self.last_value }
    };
}

/// Generate `compute_bar` and `next_with_time` for repaint support.
///
/// Generates both methods inside a `StreamingIndicator` trait `impl` block.
/// The struct must have `snapshot: Option<SnapshotState>` and `last_open_time: i64`.
///
/// # Syntax variants
///
/// ## 1. Simple fields (direct copy, same name in struct and SnapshotState)
///
/// ```ignore
/// impl_repaint!(f64, f64, sum: f64, head: usize, len: usize);
/// ```
///
/// ## 2. Custom extract expression (for `bar.close()` vs `price_source.extract(bar)`)
///
/// ```ignore
/// impl_repaint!(f64, f64, extract = self.price_source.extract(bar),
///     sum: f64, head: usize, len: usize);
/// ```
///
/// ## 3. Fields with custom save/restore expressions
///
/// ```ignore
/// // head_val is saved as `self.buffer[self.head]` and restored as `self.buffer[snap.head] = snap.head_val`
/// impl_repaint!(f64, f64,
///     sum: f64,
///     head: usize,
///     len: usize,
///     head_val => |s| s.buffer[s.head] => |s, v| s.buffer[s.head] = v
/// );
/// ```
#[macro_export]
macro_rules! impl_repaint {
    // ---- Variant 1: Simple direct-copy fields ----
    ($input_type:ty, $output_type:ty, $($field:ident : $ftype:ty),+ $(,)?) => {
        fn compute_bar(
            &mut self,
            bar: &dyn $crate::streaming::Ohlcv,
        ) -> Option<$output_type> {
            let t = bar.open_time();
            if t != 0 && t == self.last_open_time {
                if let Some(snap) = self.snapshot.take() {
                    self.count = snap.count;
                    self.last_value = snap.last_value;
                    self.last_open_time = snap.last_open_time;
                    $(self.$field = snap.$field;)+
                }
            }
            self.snapshot = Some(SnapshotState {
                count: self.count,
                last_value: self.last_value,
                last_open_time: self.last_open_time,
                $($field: self.$field,)+
            });
            self.last_open_time = t;
            self.next(bar.close())
        }

        fn next_with_time(
            &mut self,
            input: $input_type,
            open_time: i64,
        ) -> Option<$output_type> {
            if open_time != 0 && open_time == self.last_open_time {
                if let Some(snap) = self.snapshot.take() {
                    self.count = snap.count;
                    self.last_value = snap.last_value;
                    self.last_open_time = snap.last_open_time;
                    $(self.$field = snap.$field;)+
                }
            }
            self.snapshot = Some(SnapshotState {
                count: self.count,
                last_value: self.last_value,
                last_open_time: self.last_open_time,
                $($field: self.$field,)+
            });
            self.last_open_time = open_time;
            self.next(input)
        }
    };

    // ---- Variant 2: Custom extract expression ----
    ($input_type:ty, $output_type:ty, extract = $extract:expr,
     $($field:ident : $ftype:ty),+ $(,)?) => {
        fn compute_bar(
            &mut self,
            bar: &dyn $crate::streaming::Ohlcv,
        ) -> Option<$output_type> {
            let t = bar.open_time();
            if t != 0 && t == self.last_open_time {
                if let Some(snap) = self.snapshot.take() {
                    self.count = snap.count;
                    self.last_value = snap.last_value;
                    self.last_open_time = snap.last_open_time;
                    $(self.$field = snap.$field;)+
                }
            }
            self.snapshot = Some(SnapshotState {
                count: self.count,
                last_value: self.last_value,
                last_open_time: self.last_open_time,
                $($field: self.$field,)+
            });
            self.last_open_time = t;
            self.next($extract)
        }

        fn next_with_time(
            &mut self,
            input: $input_type,
            open_time: i64,
        ) -> Option<$output_type> {
            if open_time != 0 && open_time == self.last_open_time {
                if let Some(snap) = self.snapshot.take() {
                    self.count = snap.count;
                    self.last_value = snap.last_value;
                    self.last_open_time = snap.last_open_time;
                    $(self.$field = snap.$field;)+
                }
            }
            self.snapshot = Some(SnapshotState {
                count: self.count,
                last_value: self.last_value,
                last_open_time: self.last_open_time,
                $($field: self.$field,)+
            });
            self.last_open_time = open_time;
            self.next(input)
        }
    };

    // ---- Variant 3: Fields with custom save/restore closures ----
    ($input_type:ty, $output_type:ty,
     $($field:ident : $ftype:ty => $save:expr => $restore:expr),+ $(,)?) => {
        fn compute_bar(
            &mut self,
            bar: &dyn $crate::streaming::Ohlcv,
        ) -> Option<$output_type> {
            let t = bar.open_time();
            if t != 0 && t == self.last_open_time {
                if let Some(snap) = self.snapshot.take() {
                    self.count = snap.count;
                    self.last_value = snap.last_value;
                    self.last_open_time = snap.last_open_time;
                    $($restore(self, snap.$field);)+
                }
            }
            self.snapshot = Some(SnapshotState {
                count: self.count,
                last_value: self.last_value,
                last_open_time: self.last_open_time,
                $($field: $save(self),)+
            });
            self.last_open_time = t;
            self.next(bar.close())
        }

        fn next_with_time(
            &mut self,
            input: $input_type,
            open_time: i64,
        ) -> Option<$output_type> {
            if open_time != 0 && open_time == self.last_open_time {
                if let Some(snap) = self.snapshot.take() {
                    self.count = snap.count;
                    self.last_value = snap.last_value;
                    self.last_open_time = snap.last_open_time;
                    $($restore(self, snap.$field);)+
                }
            }
            self.snapshot = Some(SnapshotState {
                count: self.count,
                last_value: self.last_value,
                last_open_time: self.last_open_time,
                $($field: $save(self),)+
            });
            self.last_open_time = open_time;
            self.next(input)
        }
    };
}

/// Convenience: generate both `IndicatorMeta` + `count()` + `value()` at once.
///
/// The indicator struct must have a `period: usize` field.
///
/// # Example
///
/// ```ignore
/// impl StreamingIndicator for StreamingFoo {
///     fn next(&mut self, input: f64) -> Option<f64> { /* ... */ }
///     fn reset(&mut self) { /* ... */ }
///     fn is_ready(&self) -> bool { self.count >= self.period }
///     impl_indicator_meta_and_methods!(StreamingFoo, "FOO", "overlap", "Foo Indicator");
/// }
/// ```
#[macro_export]
macro_rules! impl_indicator_meta_and_methods {
    ($type:ty, $name:expr, $category:expr, $desc:expr) => {
        $crate::impl_indicator_meta!($type, $name, $category, $desc);
        $crate::impl_standard_methods!();
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
