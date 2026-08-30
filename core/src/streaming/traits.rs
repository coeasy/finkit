/// Trait for accessing OHLCV bar data.
///
/// Any type implementing this trait can be used as input to streaming indicators.
///
/// # Example
///
/// ```
/// use finkit::streaming::{Ohlcv, OhlcvBar};
///
/// let bar = OhlcvBar::new(100.0, 110.0, 95.0, 105.0, 1000.0);
/// assert_eq!(bar.close(), 105.0);
/// ```
pub trait Ohlcv {
    fn open(&self) -> f64;
    fn high(&self) -> f64;
    fn low(&self) -> f64;
    fn close(&self) -> f64;
    fn volume(&self) -> f64;

    /// Typical price: (H + L + C) / 3
    #[inline]
    fn typical_price(&self) -> f64 {
        (self.high() + self.low() + self.close()) / 3.0
    }

    /// Median price: (H + L) / 2
    #[inline]
    fn median_price(&self) -> f64 {
        (self.high() + self.low()) / 2.0
    }

    /// Weighted close: (H + L + 2*C) / 4
    #[inline]
    fn weighted_close(&self) -> f64 {
        (self.high() + self.low() + 2.0 * self.close()) / 4.0
    }

    /// True range relative to a previous close
    #[inline]
    fn true_range(&self, prev_close: f64) -> f64 {
        let hl = self.high() - self.low();
        let hpc = (self.high() - prev_close).abs();
        let lpc = (self.low() - prev_close).abs();
        hl.max(hpc).max(lpc)
    }

    /// Bar open timestamp (epoch millis). Used for forming-bar repaint detection.
    /// Returns 0 by default, meaning repaint is disabled.
    fn open_time(&self) -> i64 {
        0
    }
}

/// O(1) incremental indicator update interface.
///
/// Streaming indicators maintain internal state and accept one bar at a time,
/// producing an updated output without re-scanning the full history.
///
/// Returns `None` during the warm-up period before the indicator has converged,
/// and `Some(value)` once enough data has been received.
///
/// # Type Parameters
///
/// - `Input`: The bar type (usually anything implementing [`Ohlcv`], or `f64`).
/// - `Output`: The computed value(s) per bar.
///
/// # Example
///
/// ```
/// use finkit::streaming::{StreamingIndicator, OhlcvBar};
/// use finkit::streaming::indicators::StreamingSma;
///
/// let mut sma = StreamingSma::new(3);
/// assert_eq!(sma.next(1.0), None);
/// assert_eq!(sma.next(2.0), None);
/// assert_eq!(sma.next(3.0), Some(2.0));
/// assert_eq!(sma.next(4.0), Some(3.0));
/// // Cached last value
/// assert_eq!(sma.value(), Some(3.0));
/// ```
pub trait StreamingIndicator<Input = f64, Output = f64> {
    /// Feed a new data point and return the updated indicator value.
    ///
    /// Returns `None` during warm-up (before convergence), `Some(output)` once ready.
    fn next(&mut self, input: Input) -> Option<Output>;

    /// Feed a new data point with a timestamp for forming-bar repaint support.
    ///
    /// If `open_time` matches the previous call's timestamp, the indicator
    /// rolls back to the pre-bar state and recomputes with the new input,
    /// effectively replacing the forming bar.
    ///
    /// Default implementation ignores the timestamp and delegates to `next()`.
    fn next_with_time(&mut self, input: Input, _open_time: i64) -> Option<Output> {
        self.next(input)
    }

    /// Reset the indicator to its initial state.
    fn reset(&mut self);

    /// Returns `true` once the indicator has received enough data to produce valid output.
    fn is_ready(&self) -> bool;

    /// Number of data points consumed so far.
    fn count(&self) -> usize;

    /// Returns the last computed value without advancing state, or `None` if not yet converged.
    fn value(&self) -> Option<Output>;
}

/// Machine-readable indicator metadata for registry and discovery.
///
/// # Example
///
/// ```
/// use finkit::streaming::indicators::StreamingSma;
/// use finkit::streaming::IndicatorMeta;
///
/// assert_eq!(StreamingSma::name(), "SMA");
/// assert_eq!(StreamingSma::category(), "overlap");
/// ```
pub trait IndicatorMeta {
    /// Short canonical name (e.g. "SMA", "RSI", "MACD").
    fn name() -> &'static str;

    /// Category slug (e.g. "overlap", "momentum", "volume", "volatility").
    fn category() -> &'static str;

    /// Human-readable description.
    fn description() -> &'static str;

    /// Minimum warm-up period before valid output.
    fn warm_up_period(&self) -> usize;
}
