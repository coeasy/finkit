/// Macro to generate the `compute_bar` method for forming-bar repaint support.
///
/// Eliminates the duplicated `compute_bar` implementation across all
/// OHLCV-capable streaming indicators.
///
/// # Requirements
/// The target struct must have:
/// - `snapshot: Option<Box<Self>>` field
/// - `last_open_time: i64` field
/// - Implement `Clone`
/// - Implement `StreamingIndicator<Input, Output>` (the `next` method must be
///   callable with the type produced by `$extract`)
///
/// # Parameters
/// - `$type`: The struct name (e.g., `StreamingSma`)
/// - `$output_type`: The output type of the indicator (e.g., `f64`, `BollOutput`)
/// - `$extract`: Expression that extracts the input value from `bar`
///   (e.g., `bar.close()`, `(bar.high(), bar.low(), bar.close())`)
///
/// # Example
/// ```ignore
/// impl_compute_bar!(StreamingSma, f64, bar.close());
/// impl_compute_bar!(StreamingAtr, f64, (bar.high(), bar.low(), bar.close()));
/// impl_compute_bar!(StreamingBoll, BollOutput, bar.close());
/// ```
#[macro_export]
macro_rules! impl_compute_bar {
    ($type:ty, $output_type:ty, $extract:expr) => {
        impl $type {
            pub fn compute_bar(
                &mut self,
                bar: &dyn $crate::streaming::Ohlcv,
            ) -> Option<$output_type> {
                let t = bar.open_time();
                if t != 0 && t == self.last_open_time {
                    if let Some(snap) = self.snapshot.take() {
                        *self = *snap;
                    }
                }
                let mut snap = self.clone();
                snap.snapshot = None;
                self.snapshot = Some(Box::new(snap));
                self.last_open_time = t;
                self.next($extract)
            }
        }
    };
}

/// Macro to generate the `next_with_time` method body for forming-bar repaint support.
///
/// This macro generates a `fn next_with_time(...)` definition intended to be
/// placed inside a `StreamingIndicator` trait `impl` block. It cannot generate
/// the entire `impl` block because each indicator also defines `next`, `reset`,
/// and other trait methods inline.
///
/// # Requirements
/// The target struct must have:
/// - `snapshot: Option<Box<Self>>` field
/// - `last_open_time: i64` field
/// - Implement `Clone`
/// - The `next` method must be callable with `$input_type`
///
/// # Parameters
/// - `$input_type`: The input type for `next()` (e.g., `f64`, `(f64, f64, f64)`)
/// - `$output_type`: The output type of the indicator (e.g., `f64`, `MacdOutput`)
///
/// # Example
/// ```ignore
/// impl StreamingIndicator<f64, f64> for StreamingSma {
///     fn next(&mut self, input: f64) -> Option<f64> { /* ... */ }
///
///     impl_next_with_time!(f64, f64);
///
///     fn reset(&mut self) { /* ... */ }
///     // ...
/// }
/// ```
#[macro_export]
macro_rules! impl_next_with_time {
    ($input_type:ty, $output_type:ty) => {
        fn next_with_time(&mut self, input: $input_type, open_time: i64) -> Option<$output_type> {
            if open_time != 0 && open_time == self.last_open_time {
                if let Some(snap) = self.snapshot.take() {
                    *self = *snap;
                }
            }
            let mut snap = self.clone();
            snap.snapshot = None;
            self.snapshot = Some(Box::new(snap));
            self.last_open_time = open_time;
            self.next(input)
        }
    };
}
