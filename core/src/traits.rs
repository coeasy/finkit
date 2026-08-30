//! Top-level trait abstractions for the alpha_ta streaming indicator framework.
//!
//! This module defines three public traits that together form a uniform,
//! object-safe interface over alpha_ta's 150+ indicators:
//!
//! - [`Ohlcv`] — the canonical bar-data input shape (6 fields).
//! - [`StreamingIndicator`] — O(1) incremental update interface.
//! - [`BatchIndicator`] — vectorized batch compute interface.
//!
//! In addition, the module provides:
//!
//! - [`OhlcvBar`] — a 6-field owned bar (the default [`Ohlcv`] implementation).
//! - [`OhlcvArrayAdapter`] — a zero-copy adapter that maps a 6-element
//!   `&[f64; 6]` array to the [`Ohlcv`] interface.
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────┐    ┌──────────────────────────┐    ┌──────────────┐
//! │  &dyn Ohlcv    │───▶│ StreamingIndicator       │───▶│ Self::Output │
//! │  (per bar)     │    │ .update(bar) -> Option<O> │    │  (per bar)   │
//! └────────────────┘    └──────────────────────────┘    └──────────────┘
//! ```
//!
//! # Example
//!
//! ```
//! use alpha_ta_core::traits::{Ohlcv, OhlcvBar, StreamingIndicator};
//!
//! let bar = OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 1_000.0, 1_700_000_000_000);
//! assert_eq!(bar.close(), 11.0);
//! assert_eq!(bar.timestamp(), 1_700_000_000_000);
//! ```

// B1: `no_std`-portable `Vec` (declared in `lib.rs` under `no_std`).
use alloc::vec::Vec;

#[cfg(all(feature = "std", feature = "indicators-all"))]
use crate::streaming as streaming_mod;
/// Bring the inner streaming traits into scope so we can call their methods.
#[cfg(all(feature = "std", feature = "indicators-all"))]
use crate::streaming::{IndicatorMeta as InnerIndicatorMeta, StreamingIndicator as InnerStreamingIndicator};

// ---------------------------------------------------------------------------
// Ohlcv — canonical bar-data input shape (6 fields).
// ---------------------------------------------------------------------------

/// Standardized OHLCV bar-data input.
///
/// Any type implementing this trait can be fed into a [`StreamingIndicator`]
/// or used by a [`BatchIndicator`].
///
/// # Stability
///
/// This is a *minimal* contract — implementors only need to expose the six
/// accessors. Helper derivations (typical / median / weighted close) live in
/// helper methods and default to the canonical formulas when the trait
/// provides them.
///
/// # Example
///
/// ```
/// use alpha_ta_core::traits::{Ohlcv, OhlcvBar};
///
/// let bar = OhlcvBar::new(100.0, 110.0, 95.0, 105.0, 500.0, 1_700_000_000);
/// assert_eq!(bar.open(), 100.0);
/// assert_eq!(bar.high(), 110.0);
/// assert_eq!(bar.low(), 95.0);
/// assert_eq!(bar.close(), 105.0);
/// assert_eq!(bar.volume(), 500.0);
/// assert_eq!(bar.timestamp(), 1_700_000_000);
/// ```
pub trait Ohlcv {
    /// Opening price of the bar.
    fn open(&self) -> f64;
    /// High price of the bar.
    fn high(&self) -> f64;
    /// Low price of the bar.
    fn low(&self) -> f64;
    /// Closing price of the bar.
    fn close(&self) -> f64;
    /// Trading volume of the bar.
    fn volume(&self) -> f64;
    /// Bar opening timestamp in epoch milliseconds.
    ///
    /// A value of `0` means "timestamp not provided" — repaint support is
    /// disabled for such bars.
    fn timestamp(&self) -> i64;
}

// ---------------------------------------------------------------------------
// OhlcvBar — the default 6-field owned bar implementation.
// ---------------------------------------------------------------------------

/// A concrete, owned 6-field OHLCV bar.
///
/// This is the default [`Ohlcv`] implementation and is the canonical bar
/// shape consumed by the [`StreamingIndicator`] and [`BatchIndicator`]
/// adapters.
///
/// # Example
///
/// ```
/// use alpha_ta_core::traits::{Ohlcv, OhlcvBar};
///
/// let bar = OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 1_000.0, 0);
/// assert_eq!(bar.volume(), 1_000.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OhlcvBar {
    /// Opening price.
    pub open: f64,
    /// High price.
    pub high: f64,
    /// Low price.
    pub low: f64,
    /// Closing price.
    pub close: f64,
    /// Volume.
    pub volume: f64,
    /// Bar opening timestamp (epoch millis). `0` disables repaint detection.
    pub timestamp: i64,
}

impl OhlcvBar {
    /// Build a new bar with `timestamp = 0` (repaint disabled).
    #[inline]
    pub fn new(open: f64, high: f64, low: f64, close: f64, volume: f64, timestamp: i64) -> Self {
        Self {
            open,
            high,
            low,
            close,
            volume,
            timestamp,
        }
    }

    /// Build a new bar with explicit timestamp.
    #[inline]
    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = timestamp;
        self
    }
}

impl Ohlcv for OhlcvBar {
    #[inline]
    fn open(&self) -> f64 {
        self.open
    }
    #[inline]
    fn high(&self) -> f64 {
        self.high
    }
    #[inline]
    fn low(&self) -> f64 {
        self.low
    }
    #[inline]
    fn close(&self) -> f64 {
        self.close
    }
    #[inline]
    fn volume(&self) -> f64 {
        self.volume
    }
    #[inline]
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
}

// ---------------------------------------------------------------------------
// Free-function adapters — implement Ohlcv for stock shapes.
// ---------------------------------------------------------------------------

/// Adapt a `(f64, f64, f64, f64, f64, i64)` 6-tuple to the [`Ohlcv`] interface.
///
/// Tuple layout: `(open, high, low, close, volume, timestamp)`.
impl Ohlcv for (f64, f64, f64, f64, f64, i64) {
    #[inline]
    fn open(&self) -> f64 {
        self.0
    }
    #[inline]
    fn high(&self) -> f64 {
        self.1
    }
    #[inline]
    fn low(&self) -> f64 {
        self.2
    }
    #[inline]
    fn close(&self) -> f64 {
        self.3
    }
    #[inline]
    fn volume(&self) -> f64 {
        self.4
    }
    #[inline]
    fn timestamp(&self) -> i64 {
        self.5
    }
}

/// Adapt a `&[f64; 6]` array to the [`Ohlcv`] interface.
///
/// Array layout: `[open, high, low, close, volume, timestamp]`.
pub struct OhlcvArrayAdapter {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    timestamp: i64,
}

impl OhlcvArrayAdapter {
    /// Build an adapter from a `&[f64; 6]`.
    #[inline]
    pub fn from_array(arr: &[f64; 6]) -> Self {
        Self {
            open: arr[0],
            high: arr[1],
            low: arr[2],
            close: arr[3],
            volume: arr[4],
            timestamp: arr[5] as i64,
        }
    }
}

impl Ohlcv for OhlcvArrayAdapter {
    #[inline]
    fn open(&self) -> f64 {
        self.open
    }
    #[inline]
    fn high(&self) -> f64 {
        self.high
    }
    #[inline]
    fn low(&self) -> f64 {
        self.low
    }
    #[inline]
    fn close(&self) -> f64 {
        self.close
    }
    #[inline]
    fn volume(&self) -> f64 {
        self.volume
    }
    #[inline]
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
}

// ---------------------------------------------------------------------------
// StreamingIndicator — O(1) incremental update interface.
// ---------------------------------------------------------------------------

/// Streaming (incremental) indicator that updates in O(1) per bar.
///
/// The trait is the *uniform* contract over alpha_ta's 150+ indicators.  It is
/// independent of the existing
/// `crate::streaming::StreamingIndicator` (which uses a generic `Input` /
/// `Output` shape); adapters wrap each concrete type.
///
/// # Associated types
///
/// - [`StreamingIndicator::Config`] — constructor argument bundle; must be
///   `Clone` so that the same config can be reused.
/// - [`StreamingIndicator::Output`] — per-bar output value (a scalar or a
///   small struct such as `MacdOutput`).
///
/// # Convergence
///
/// [`StreamingIndicator::convergence`] returns the number of bars the
/// indicator needs to produce a *numerically stable* output.  It is the
/// `warm_up_period` reported by alpha_ta's batch layer and is a conservative
/// upper bound (the actual `is_ready()` may flip earlier).
///
/// # Repaint
///
/// [`StreamingIndicator::repaint`] feeds a bar that *replaces* the most
/// recent bar in the stream (same `timestamp`, new price).  Indicators that
/// support forming-bar repaint roll back their state and recompute; the
/// default implementation simply delegates to [`StreamingIndicator::update`].
///
/// # Example
///
/// ```
/// use alpha_ta_core::traits::{Ohlcv, OhlcvBar, StreamingIndicator};
/// use alpha_ta_core::streaming::indicators::StreamingSma;
/// use alpha_ta_core::streaming::{IndicatorMeta, StreamingIndicator as InnerStreamingIndicator};
///
/// // Adapt the existing concrete type to the trait wrapper:
/// struct SmaAdapter(StreamingSma);
///
/// impl StreamingIndicator for SmaAdapter {
///     type Config = usize;
///     type Output = f64;
///     fn new(period: usize) -> Self { Self(StreamingSma::new(period)) }
///     fn update(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
///         // The outer `Ohlcv` and the streaming-layer `Ohlcv` both expose
///         // the same accessor methods, so the inner `next` accepts the
///         // close price directly.
///         self.0.next(bar.close())
///     }
///     fn repaint(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
///         self.update(bar)
///     }
///     fn convergence(&self) -> usize { self.0.warm_up_period() }
///     fn reset(&mut self) { self.0.reset(); }
/// }
/// ```
pub trait StreamingIndicator: Send {
    /// Constructor configuration bundle.
    type Config: Clone;
    /// Per-bar output value type.
    type Output: Clone;

    /// Build a fresh indicator with the given configuration.
    fn new(config: Self::Config) -> Self;

    /// Update with a new bar.
    ///
    /// Returns `None` while the indicator is still warming up (before
    /// [`StreamingIndicator::convergence`] bars have been consumed), and
    /// `Some(value)` once the output has stabilized.
    fn update(&mut self, bar: &dyn Ohlcv) -> Option<Self::Output>;

    /// Re-feed a bar that *replaces* the most recent bar in the stream
    /// (same `timestamp`, new price).
    ///
    /// The default implementation delegates to [`StreamingIndicator::update`].
    /// Indicators that maintain forming-bar snapshots override this to roll
    /// back to the pre-bar state and recompute.
    fn repaint(&mut self, bar: &dyn Ohlcv) -> Option<Self::Output> {
        self.update(bar)
    }

    /// Number of bars the indicator needs before producing a stable output.
    fn convergence(&self) -> usize;

    /// Reset the indicator to its initial state.
    fn reset(&mut self);
}

// ---------------------------------------------------------------------------
// BatchIndicator — vectorized batch compute interface.
// ---------------------------------------------------------------------------

/// Batch (vectorized) indicator that consumes the full OHLCV history and
/// produces a same-length output vector.
///
/// The default per-indicator implementations live in
/// `crate::indicators::*`; this trait is the *uniform* entry point so
/// downstream code can stay generic across indicators.
///
/// # Example
///
/// ```
/// use alpha_ta_core::traits::{BatchIndicator, Ohlcv};
/// use alpha_ta_core::indicators::overlap::sma;
///
/// struct SmaBatch { period: usize }
///
/// impl BatchIndicator for SmaBatch {
///     type Output = f64;
///     fn calculate(&self, data: &[&dyn Ohlcv]) -> Vec<f64> {
///         let closes: Vec<f64> = data.iter().map(|b| b.close()).collect();
///         sma(&closes, self.period).unwrap().to_vec()
///     }
/// }
/// ```
pub trait BatchIndicator {
    /// Per-bar output value type.
    type Output;

    /// Compute the indicator over the full bar history.
    ///
    /// Implementations should align with their streaming counterpart: after
    /// `convergence` bars the streaming and batch outputs should agree to
    /// within 1e-12.
    fn calculate(&self, data: &[&dyn Ohlcv]) -> Vec<Self::Output>;
}

// ---------------------------------------------------------------------------
// Bridge — adapt the top-level `Ohlcv` to the inner streaming `Ohlcv`.
// ---------------------------------------------------------------------------

/// Adapter that bridges a `&dyn` top-level [`Ohlcv`] (with `timestamp`) to the
/// inner `crate::streaming::Ohlcv` (with `open_time`).
///
/// This lets the `update` / `repaint` adapters feed any `&dyn Ohlcv` to
/// existing `compute_bar` / `next` methods that expect
/// `&dyn crate::streaming::Ohlcv`.
#[cfg(all(feature = "std", feature = "indicators-all"))]
struct OhlcvCompat<'a> {
    bar: &'a dyn Ohlcv,
}

#[cfg(all(feature = "std", feature = "indicators-all"))]
impl<'a> streaming_mod::Ohlcv for OhlcvCompat<'a> {
    #[inline]
    fn open(&self) -> f64 {
        Ohlcv::open(self.bar)
    }
    #[inline]
    fn high(&self) -> f64 {
        Ohlcv::high(self.bar)
    }
    #[inline]
    fn low(&self) -> f64 {
        Ohlcv::low(self.bar)
    }
    #[inline]
    fn close(&self) -> f64 {
        Ohlcv::close(self.bar)
    }
    #[inline]
    fn volume(&self) -> f64 {
        Ohlcv::volume(self.bar)
    }
    #[inline]
    fn open_time(&self) -> i64 {
        Ohlcv::timestamp(self.bar)
    }
}

// ---------------------------------------------------------------------------
// StreamingIndicator adapters for the existing concrete streaming types.
//
// These adapters do NOT modify the existing streaming implementations; they
// only expose them through the top-level `StreamingIndicator` trait.
// ---------------------------------------------------------------------------

/// Adapter for `StreamingSma`.
#[cfg(all(feature = "std", feature = "indicators-all"))]
pub struct SmaAdapter(pub streaming_mod::indicators::StreamingSma);

#[cfg(all(feature = "std", feature = "indicators-all"))]
impl StreamingIndicator for SmaAdapter {
    type Config = usize;
    type Output = f64;

    fn new(period: usize) -> Self {
        Self(streaming_mod::indicators::StreamingSma::new(period))
    }

    fn update(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        let compat = OhlcvCompat { bar };
        self.0.compute_bar(&compat)
    }

    fn repaint(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        // `compute_bar` already handles the same-timestamp case internally.
        self.update(bar)
    }

    fn convergence(&self) -> usize {
        InnerIndicatorMeta::warm_up_period(&self.0)
    }

    fn reset(&mut self) {
        InnerStreamingIndicator::reset(&mut self.0);
    }
}

/// Adapter for `StreamingEma`.
#[cfg(all(feature = "std", feature = "indicators-all"))]
pub struct EmaAdapter(pub streaming_mod::indicators::StreamingEma);

#[cfg(all(feature = "std", feature = "indicators-all"))]
impl StreamingIndicator for EmaAdapter {
    type Config = usize;
    type Output = f64;

    fn new(period: usize) -> Self {
        Self(streaming_mod::indicators::StreamingEma::new(period))
    }

    fn update(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        let compat = OhlcvCompat { bar };
        self.0.compute_bar(&compat)
    }

    fn repaint(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.update(bar)
    }

    fn convergence(&self) -> usize {
        // Spec contract: EMA(n) -> 3*(n+1) conservative convergence.
        3 * (InnerIndicatorMeta::warm_up_period(&self.0) + 1)
    }

    fn reset(&mut self) {
        InnerStreamingIndicator::reset(&mut self.0);
    }
}

/// Adapter for `StreamingRsi`.
#[cfg(all(feature = "std", feature = "indicators-all"))]
pub struct RsiAdapter(pub streaming_mod::indicators::StreamingRsi);

#[cfg(all(feature = "std", feature = "indicators-all"))]
impl StreamingIndicator for RsiAdapter {
    type Config = usize;
    type Output = f64;

    fn new(period: usize) -> Self {
        Self(streaming_mod::indicators::StreamingRsi::new(period))
    }

    fn update(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        let compat = OhlcvCompat { bar };
        self.0.compute_bar(&compat)
    }

    fn repaint(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.update(bar)
    }

    fn convergence(&self) -> usize {
        // RSI needs the first comparison, then `period` averaged samples.
        InnerIndicatorMeta::warm_up_period(&self.0) + 1
    }

    fn reset(&mut self) {
        InnerStreamingIndicator::reset(&mut self.0);
    }
}

/// Adapter for `StreamingMacd`.
///
/// Stores the original `(fast, slow, signal)` config so the spec-mandated
/// `convergence()` formula (`max(slow, signal) + slow`) can be reported
/// without reaching into the private fields of the inner type.
#[cfg(all(feature = "std", feature = "indicators-all"))]
pub struct MacdAdapter {
    inner: streaming_mod::indicators::StreamingMacd,
    config: (usize, usize, usize),
}

#[cfg(all(feature = "std", feature = "indicators-all"))]
impl StreamingIndicator for MacdAdapter {
    /// `(fast_period, slow_period, signal_period)` configuration tuple.
    type Config = (usize, usize, usize);
    /// Re-use the existing MACD output struct.
    type Output = streaming_mod::momentum::macd::MacdOutput;

    fn new(config: Self::Config) -> Self {
        let (fast, slow, signal) = config;
        Self {
            inner: streaming_mod::indicators::StreamingMacd::new(fast, slow, signal),
            config,
        }
    }

    fn update(&mut self, bar: &dyn Ohlcv) -> Option<Self::Output> {
        let compat = OhlcvCompat { bar };
        self.inner.compute_bar(&compat)
    }

    fn repaint(&mut self, bar: &dyn Ohlcv) -> Option<Self::Output> {
        self.update(bar)
    }

    fn convergence(&self) -> usize {
        // Spec contract: max(slow, signal) + slow conservative.
        let (_fast, slow, signal) = self.config;
        slow + signal.max(slow)
    }

    fn reset(&mut self) {
        InnerStreamingIndicator::reset(&mut self.inner);
    }
}

/// Adapter for `StreamingBoll`.
#[cfg(all(feature = "std", feature = "indicators-all"))]
pub struct BollAdapter(pub streaming_mod::indicators::StreamingBoll);

#[cfg(all(feature = "std", feature = "indicators-all"))]
impl StreamingIndicator for BollAdapter {
    /// `(period, nb_dev_up, nb_dev_dn)` configuration tuple.
    type Config = (usize, f64, f64);
    /// Re-use the existing BBANDS output struct.
    type Output = streaming_mod::volatility::boll::BollOutput;

    fn new(config: Self::Config) -> Self {
        let (period, up, dn) = config;
        Self(streaming_mod::indicators::StreamingBoll::new(period, up, dn))
    }

    fn update(&mut self, bar: &dyn Ohlcv) -> Option<Self::Output> {
        let compat = OhlcvCompat { bar };
        self.0.compute_bar(&compat)
    }

    fn repaint(&mut self, bar: &dyn Ohlcv) -> Option<Self::Output> {
        self.update(bar)
    }

    fn convergence(&self) -> usize {
        InnerIndicatorMeta::warm_up_period(&self.0)
    }

    fn reset(&mut self) {
        InnerStreamingIndicator::reset(&mut self.0);
    }
}

/// Adapter for `StreamingAtr`.
#[cfg(all(feature = "std", feature = "indicators-all"))]
pub struct AtrAdapter(pub streaming_mod::indicators::StreamingAtr);

#[cfg(all(feature = "std", feature = "indicators-all"))]
impl StreamingIndicator for AtrAdapter {
    type Config = usize;
    type Output = f64;

    fn new(period: usize) -> Self {
        Self(streaming_mod::indicators::StreamingAtr::new(period))
    }

    fn update(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        let compat = OhlcvCompat { bar };
        self.0.compute_bar(&compat)
    }

    fn repaint(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.update(bar)
    }

    fn convergence(&self) -> usize {
        InnerIndicatorMeta::warm_up_period(&self.0)
    }

    fn reset(&mut self) {
        InnerStreamingIndicator::reset(&mut self.0);
    }
}

/// Adapter for `StreamingKdj`.
#[cfg(all(feature = "std", feature = "indicators-all"))]
pub struct KdjAdapter(pub streaming_mod::indicators::StreamingKdj);

#[cfg(all(feature = "std", feature = "indicators-all"))]
impl StreamingIndicator for KdjAdapter {
    /// `(n, m1, m2)` configuration tuple.
    type Config = (usize, usize, usize);
    /// Re-use the existing KDJ output struct.
    type Output = streaming_mod::momentum::kdj::KdjOutput;

    fn new(config: Self::Config) -> Self {
        let (n, m1, m2) = config;
        Self(streaming_mod::indicators::StreamingKdj::new(n, m1, m2))
    }

    fn update(&mut self, bar: &dyn Ohlcv) -> Option<Self::Output> {
        // StreamingKdj's `next` takes (high, low, close) — extract from bar.
        InnerStreamingIndicator::next(
            &mut self.0,
            (Ohlcv::high(bar), Ohlcv::low(bar), Ohlcv::close(bar)),
        )
    }

    fn repaint(&mut self, bar: &dyn Ohlcv) -> Option<Self::Output> {
        self.update(bar)
    }

    fn convergence(&self) -> usize {
        InnerIndicatorMeta::warm_up_period(&self.0)
    }

    fn reset(&mut self) {
        InnerStreamingIndicator::reset(&mut self.0);
    }
}

/// Adapter for `StreamingStoch`.
#[cfg(all(feature = "std", feature = "indicators-all"))]
pub struct StochAdapter(pub streaming_mod::indicators::StreamingStoch);

#[cfg(all(feature = "std", feature = "indicators-all"))]
impl StreamingIndicator for StochAdapter {
    /// `(k_period, k_slow, d_period)` configuration tuple.
    type Config = (usize, usize, usize);
    /// Re-use the existing Stoch output struct.
    type Output = streaming_mod::momentum::stoch::StochOutput;

    fn new(config: Self::Config) -> Self {
        let (k_period, k_slow, d_period) = config;
        Self(streaming_mod::indicators::StreamingStoch::new(
            k_period, k_slow, d_period,
        ))
    }

    fn update(&mut self, bar: &dyn Ohlcv) -> Option<Self::Output> {
        InnerStreamingIndicator::next(
            &mut self.0,
            (Ohlcv::high(bar), Ohlcv::low(bar), Ohlcv::close(bar)),
        )
    }

    fn repaint(&mut self, bar: &dyn Ohlcv) -> Option<Self::Output> {
        self.update(bar)
    }

    fn convergence(&self) -> usize {
        InnerIndicatorMeta::warm_up_period(&self.0)
    }

    fn reset(&mut self) {
        InnerStreamingIndicator::reset(&mut self.0);
    }
}

/// Adapter for `StreamingObv`.
#[cfg(all(feature = "std", feature = "indicators-all"))]
pub struct ObvAdapter(pub streaming_mod::indicators::StreamingObv);

#[cfg(all(feature = "std", feature = "indicators-all"))]
impl StreamingIndicator for ObvAdapter {
    type Config = ();
    type Output = f64;

    fn new(_config: Self::Config) -> Self {
        Self(streaming_mod::indicators::StreamingObv::new())
    }

    fn update(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        let compat = OhlcvCompat { bar };
        InnerStreamingIndicator::next(&mut self.0, &compat)
    }

    fn repaint(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.update(bar)
    }

    fn convergence(&self) -> usize {
        1
    }

    fn reset(&mut self) {
        InnerStreamingIndicator::reset(&mut self.0);
    }
}

/// Adapter for `StreamingAd`.
#[cfg(all(feature = "std", feature = "indicators-all"))]
pub struct AdAdapter(pub streaming_mod::indicators::StreamingAd);

#[cfg(all(feature = "std", feature = "indicators-all"))]
impl StreamingIndicator for AdAdapter {
    type Config = ();
    type Output = f64;

    fn new(_config: Self::Config) -> Self {
        Self(streaming_mod::indicators::StreamingAd::new())
    }

    fn update(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        let compat = OhlcvCompat { bar };
        InnerStreamingIndicator::next(&mut self.0, &compat)
    }

    fn repaint(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.update(bar)
    }

    fn convergence(&self) -> usize {
        1
    }

    fn reset(&mut self) {
        InnerStreamingIndicator::reset(&mut self.0);
    }
}

/// Adapter for `StreamingMfi`.
#[cfg(all(feature = "std", feature = "indicators-all"))]
pub struct MfiAdapter(pub streaming_mod::indicators::StreamingMfi);

#[cfg(all(feature = "std", feature = "indicators-all"))]
impl StreamingIndicator for MfiAdapter {
    type Config = usize;
    type Output = f64;

    fn new(period: usize) -> Self {
        Self(streaming_mod::indicators::StreamingMfi::new(period))
    }

    fn update(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        let compat = OhlcvCompat { bar };
        self.0.next(&compat)
    }

    fn repaint(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.update(bar)
    }

    fn convergence(&self) -> usize {
        // MFI requires period+1 bars to compare typical prices.
        InnerIndicatorMeta::warm_up_period(&self.0) + 1
    }

    fn reset(&mut self) {
        self.0.reset();
    }
}

/// Adapter for `StreamingVwap`.
#[cfg(all(feature = "std", feature = "indicators-all"))]
pub struct VwapAdapter(pub streaming_mod::indicators::StreamingVwap);

#[cfg(all(feature = "std", feature = "indicators-all"))]
impl StreamingIndicator for VwapAdapter {
    type Config = ();
    type Output = f64;

    fn new(_config: Self::Config) -> Self {
        Self(streaming_mod::indicators::StreamingVwap::new())
    }

    fn update(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        let compat = OhlcvCompat { bar };
        InnerStreamingIndicator::next(&mut self.0, &compat)
    }

    fn repaint(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.update(bar)
    }

    fn convergence(&self) -> usize {
        1
    }

    fn reset(&mut self) {
        InnerStreamingIndicator::reset(&mut self.0);
    }
}

// ---------------------------------------------------------------------------
// BatchIndicator: uniform batch compute over the Ohlcv history.
// ---------------------------------------------------------------------------

/// SMA batch indicator — a uniform-batch facade over `crate::math::moving_avg::sma`.
#[cfg(all(feature = "std", feature = "indicators-all"))]
pub struct SmaBatch {
    /// Lookback period.
    pub period: usize,
}

#[cfg(all(feature = "std", feature = "indicators-all"))]
impl BatchIndicator for SmaBatch {
    type Output = f64;

    fn calculate(&self, data: &[&dyn Ohlcv]) -> Vec<Self::Output> {
        let closes: Vec<f64> = data.iter().map(|b| b.close()).collect();
        crate::math::moving_avg::sma(&closes, self.period)
            .map(|arr| arr.to_vec())
            .unwrap_or_default()
    }
}

/// EMA batch indicator — uniform-batch facade over `crate::math::moving_avg::ema`.
#[cfg(all(feature = "std", feature = "indicators-all"))]
pub struct EmaBatch {
    /// Lookback period.
    pub period: usize,
}

#[cfg(all(feature = "std", feature = "indicators-all"))]
impl BatchIndicator for EmaBatch {
    type Output = f64;

    fn calculate(&self, data: &[&dyn Ohlcv]) -> Vec<Self::Output> {
        let closes: Vec<f64> = data.iter().map(|b| b.close()).collect();
        crate::math::moving_avg::ema(&closes, self.period)
            .map(|arr| arr.to_vec())
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "std", feature = "indicators-all"))]
mod tests {
    use super::*;

    fn make_bars(n: usize) -> Vec<OhlcvBar> {
        (0..n)
            .map(|i| {
                let base = 100.0 + (i as f64 * 0.1).sin() * 5.0;
                OhlcvBar::new(base, base + 1.0, base - 1.0, base, 1_000.0, i as i64)
            })
            .collect()
    }

    #[test]
    fn test_ohlcv_bar_implements_ohlcv() {
        let bar = OhlcvBar::new(10.0, 11.0, 9.5, 10.5, 1_000.0, 1_700_000_000);
        let o: &dyn Ohlcv = &bar;
        assert_eq!(o.open(), 10.0);
        assert_eq!(o.high(), 11.0);
        assert_eq!(o.low(), 9.5);
        assert_eq!(o.close(), 10.5);
        assert_eq!(o.volume(), 1_000.0);
        assert_eq!(o.timestamp(), 1_700_000_000);
    }

    #[test]
    fn test_ohlcv_tuple_implements_ohlcv() {
        let tup: (f64, f64, f64, f64, f64, i64) = (1.0, 2.0, 0.5, 1.5, 100.0, 42);
        let o: &dyn Ohlcv = &tup;
        assert_eq!(o.open(), 1.0);
        assert_eq!(o.high(), 2.0);
        assert_eq!(o.low(), 0.5);
        assert_eq!(o.close(), 1.5);
        assert_eq!(o.volume(), 100.0);
        assert_eq!(o.timestamp(), 42);
    }

    #[test]
    fn test_ohlcv_array_adapter() {
        let arr = [1.0, 2.0, 0.5, 1.5, 100.0, 7.0];
        let adapter = OhlcvArrayAdapter::from_array(&arr);
        let o: &dyn Ohlcv = &adapter;
        assert_eq!(o.open(), 1.0);
        assert_eq!(o.close(), 1.5);
        assert_eq!(o.timestamp(), 7);
    }

    #[test]
    fn test_sma_convergence_returns_n() {
        let adapter = SmaAdapter::new(20);
        assert_eq!(adapter.convergence(), 20);
    }

    #[test]
    fn test_ema_convergence_returns_3_times_n_plus_1() {
        let n = 14;
        let adapter = EmaAdapter::new(n);
        // The new spec contract: 3*(n+1) conservative convergence for EMA.
        assert_eq!(adapter.convergence(), 3 * (n + 1));
    }

    #[test]
    fn test_sma_adapter_implements_streaming_indicator() {
        let mut adapter = SmaAdapter::new(3);
        let bars = make_bars(10);
        let mut outputs = Vec::new();
        for bar in &bars {
            let o: &dyn Ohlcv = bar;
            outputs.push(adapter.update(o));
        }
        // First 2 outputs should be None (warm-up), then Some(...)
        assert!(outputs[0].is_none());
        assert!(outputs[1].is_none());
        assert!(outputs[2].is_some());
    }

    #[test]
    fn test_sma_streaming_vs_batch_after_convergence() {
        let period = 14;
        let n = 100;
        let bars = make_bars(n);

        // Batch reference
        let batch = SmaBatch { period };
        let dyn_refs: Vec<&dyn Ohlcv> = bars.iter().map(|b| b as &dyn Ohlcv).collect();
        let batch_out = batch.calculate(&dyn_refs);

        // Streaming
        let mut adapter = SmaAdapter::new(period);
        let mut stream_out = Vec::with_capacity(n);
        for bar in &bars {
            let o: &dyn Ohlcv = bar;
            stream_out.push(adapter.update(o));
        }

        // Compare after convergence (bar index >= period)
        for i in period..n {
            let s = stream_out[i].expect("streaming should be ready after convergence");
            let b = batch_out[i];
            assert!(
                (s - b).abs() < 1e-12,
                "Mismatch at index {i}: streaming={s}, batch={b}"
            );
        }
    }

    #[test]
    fn test_ema_streaming_vs_batch_after_convergence() {
        let period = 10;
        let n = 200;
        let bars = make_bars(n);

        let batch = EmaBatch { period };
        let dyn_refs: Vec<&dyn Ohlcv> = bars.iter().map(|b| b as &dyn Ohlcv).collect();
        let batch_out = batch.calculate(&dyn_refs);

        let mut adapter = EmaAdapter::new(period);
        let mut stream_out = Vec::with_capacity(n);
        for bar in &bars {
            let o: &dyn Ohlcv = bar;
            stream_out.push(adapter.update(o));
        }

        // Compare after EMA's convergence (period)
        for i in period..n {
            let s = stream_out[i].expect("streaming should be ready after convergence");
            let b = batch_out[i];
            if !b.is_nan() {
                assert!(
                    (s - b).abs() < 1e-9,
                    "Mismatch at index {i}: streaming={s}, batch={b}"
                );
            }
        }
    }

    #[test]
    fn test_ohlcv_bar_to_inner_ohlcv_bridge() {
        // The new OhlcvBar should be passable (via OhlcvCompat bridge) to
        // existing streaming compute_bar methods.
        use streaming_mod::indicators::StreamingSma;
        let mut sma = StreamingSma::new(3);
        let bar1 = OhlcvBar::new(0.0, 0.0, 0.0, 1.0, 0.0, 1);
        let bar2 = OhlcvBar::new(0.0, 0.0, 0.0, 2.0, 0.0, 2);
        let bar3 = OhlcvBar::new(0.0, 0.0, 0.0, 3.0, 0.0, 3);
        let v1 = sma.compute_bar(&OhlcvCompat { bar: &bar1 });
        let v2 = sma.compute_bar(&OhlcvCompat { bar: &bar2 });
        let v3 = sma.compute_bar(&OhlcvCompat { bar: &bar3 });
        assert!(v1.is_none());
        assert!(v2.is_none());
        assert!(v3.is_some());
    }

    #[test]
    fn test_rsi_convergence_after_period_plus_1() {
        let adapter = RsiAdapter::new(14);
        // RSI needs period+1 bars (the first comparison then period averaged).
        assert!(adapter.convergence() > 14);
    }

    #[test]
    fn test_boll_adapter_basic() {
        let mut adapter = BollAdapter::new((5, 2.0, 2.0));
        let bars = make_bars(10);
        for bar in &bars {
            let o: &dyn Ohlcv = bar;
            let out = adapter.update(o);
            if let Some(b) = out {
                assert!(b.upper >= b.middle);
                assert!(b.middle >= b.lower);
            }
        }
    }

    #[test]
    fn test_macd_adapter_config() {
        let adapter = MacdAdapter::new((12, 26, 9));
        // 26 + max(26, 9) = 52
        assert_eq!(adapter.convergence(), 52);
    }

    #[test]
    fn test_atr_adapter_basic() {
        let mut adapter = AtrAdapter::new(5);
        let bars = make_bars(20);
        for bar in &bars {
            let o: &dyn Ohlcv = bar;
            if let Some(v) = adapter.update(o) {
                assert!(v >= 0.0);
            }
        }
    }

    #[test]
    fn test_obv_adapter_convergence() {
        let adapter = ObvAdapter::new(());
        assert_eq!(adapter.convergence(), 1);
    }

    #[test]
    fn test_vwap_adapter_convergence() {
        let adapter = VwapAdapter::new(());
        assert_eq!(adapter.convergence(), 1);
    }

    #[test]
    fn test_kdj_adapter_basic() {
        let mut adapter = KdjAdapter::new((9, 3, 3));
        let bars = make_bars(50);
        for bar in &bars {
            let o: &dyn Ohlcv = bar;
            if let Some(out) = adapter.update(o) {
                // K and D are in [0, 100] by construction
                assert!((0.0..=100.0).contains(&out.k));
                assert!((0.0..=100.0).contains(&out.d));
            }
        }
    }

    #[test]
    fn test_stoch_adapter_basic() {
        let mut adapter = StochAdapter::new((14, 3, 3));
        let bars = make_bars(50);
        for bar in &bars {
            let o: &dyn Ohlcv = bar;
            if let Some(out) = adapter.update(o) {
                assert!((0.0..=100.0).contains(&out.k));
                assert!((0.0..=100.0).contains(&out.d));
            }
        }
    }

    #[test]
    fn test_mfi_adapter_basic() {
        let mut adapter = MfiAdapter::new(14);
        let bars = make_bars(50);
        for bar in &bars {
            let o: &dyn Ohlcv = bar;
            if let Some(v) = adapter.update(o) {
                assert!((0.0..=100.0).contains(&v));
            }
        }
    }

    #[test]
    fn test_ad_adapter_basic() {
        let mut adapter = AdAdapter::new(());
        let bars = make_bars(20);
        for bar in &bars {
            let o: &dyn Ohlcv = bar;
            let v = adapter.update(o);
            assert!(v.is_some());
        }
    }

    #[test]
    fn test_repaint_handling() {
        // Two bars with the same timestamp should not double-count.
        let mut adapter = SmaAdapter::new(3);
        let bar_a = OhlcvBar::new(0.0, 0.0, 0.0, 1.0, 0.0, 1);
        let bar_b = OhlcvBar::new(0.0, 0.0, 0.0, 2.0, 0.0, 2);
        let bar_c_first = OhlcvBar::new(0.0, 0.0, 0.0, 100.0, 0.0, 3);
        let bar_c_repaint_a = OhlcvBar::new(0.0, 0.0, 0.0, 200.0, 0.0, 3);
        let bar_c_final = OhlcvBar::new(0.0, 0.0, 0.0, 3.0, 0.0, 3);

        let a: &dyn Ohlcv = &bar_a;
        let b: &dyn Ohlcv = &bar_b;
        let c1: &dyn Ohlcv = &bar_c_first;
        let c2: &dyn Ohlcv = &bar_c_repaint_a;
        let c3: &dyn Ohlcv = &bar_c_final;

        adapter.update(a);
        adapter.update(b);
        adapter.update(c1);
        adapter.update(c2); // repaint — should roll back the c1 state
        let final_val = adapter.repaint(c3); // final repaint to 3.0

        // Compare against a clean run that only saw the final value
        let mut clean = SmaAdapter::new(3);
        clean.update(a);
        clean.update(b);
        let clean_val = clean.update(c3);

        assert_eq!(final_val, clean_val);
    }
}
