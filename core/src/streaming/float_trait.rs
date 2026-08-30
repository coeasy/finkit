//! Generic floating-point trait for precision-agnostic streaming indicators.
//!
//! The [`Float`] trait abstracts over `f32` and `f64`, enabling core streaming
//! indicators to operate in either precision mode.
//!
//! # Example
//!
//! ```
//! use alpha_ta_core::streaming::float_trait::{Float, GenericSma};
//!
//! // f64 (default)
//! let mut sma64 = GenericSma::<f64>::new(3);
//! sma64.next(1.0_f64);
//! sma64.next(2.0_f64);
//! assert_eq!(sma64.next(3.0_f64), Some(2.0_f64));
//!
//! // f32
//! let mut sma32 = GenericSma::<f32>::new(3);
//! sma32.next(1.0_f32);
//! sma32.next(2.0_f32);
//! let val = sma32.next(3.0_f32).unwrap();
//! assert!((val - 2.0_f32).abs() < 1e-6);
//! ```

use std::fmt::Debug;
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// Trait abstracting over `f32` and `f64` for generic indicator computation.
pub trait Float:
    Copy
    + Clone
    + Debug
    + PartialOrd
    + Default
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
    + AddAssign
    + SubAssign
    + Send
    + Sync
    + 'static
{
    fn zero() -> Self;
    fn one() -> Self;
    fn two() -> Self;
    fn hundred() -> Self;
    fn nan() -> Self;
    fn neg_infinity() -> Self;
    fn infinity() -> Self;
    fn from_usize(v: usize) -> Self;
    fn from_f64(v: f64) -> Self;
    fn abs(self) -> Self;
    fn max(self, other: Self) -> Self;
    fn min(self, other: Self) -> Self;
    fn sqrt(self) -> Self;
    fn is_nan(self) -> bool;
    fn to_f64(self) -> f64;
    fn epsilon() -> Self;
}

impl Float for f64 {
    #[inline(always)]
    fn zero() -> Self { 0.0 }
    #[inline(always)]
    fn one() -> Self { 1.0 }
    #[inline(always)]
    fn two() -> Self { 2.0 }
    #[inline(always)]
    fn hundred() -> Self { 100.0 }
    #[inline(always)]
    fn nan() -> Self { f64::NAN }
    #[inline(always)]
    fn neg_infinity() -> Self { f64::NEG_INFINITY }
    #[inline(always)]
    fn infinity() -> Self { f64::INFINITY }
    #[inline(always)]
    fn from_usize(v: usize) -> Self { v as f64 }
    #[inline(always)]
    fn from_f64(v: f64) -> Self { v }
    #[inline(always)]
    fn abs(self) -> Self { f64::abs(self) }
    #[inline(always)]
    fn max(self, other: Self) -> Self { f64::max(self, other) }
    #[inline(always)]
    fn min(self, other: Self) -> Self { f64::min(self, other) }
    #[inline(always)]
    fn sqrt(self) -> Self { f64::sqrt(self) }
    #[inline(always)]
    fn is_nan(self) -> bool { f64::is_nan(self) }
    #[inline(always)]
    fn to_f64(self) -> f64 { self }
    #[inline(always)]
    fn epsilon() -> Self { 1e-15 }
}

impl Float for f32 {
    #[inline(always)]
    fn zero() -> Self { 0.0 }
    #[inline(always)]
    fn one() -> Self { 1.0 }
    #[inline(always)]
    fn two() -> Self { 2.0 }
    #[inline(always)]
    fn hundred() -> Self { 100.0 }
    #[inline(always)]
    fn nan() -> Self { f32::NAN }
    #[inline(always)]
    fn neg_infinity() -> Self { f32::NEG_INFINITY }
    #[inline(always)]
    fn infinity() -> Self { f32::INFINITY }
    #[inline(always)]
    fn from_usize(v: usize) -> Self { v as f32 }
    #[inline(always)]
    fn from_f64(v: f64) -> Self { v as f32 }
    #[inline(always)]
    fn abs(self) -> Self { f32::abs(self) }
    #[inline(always)]
    fn max(self, other: Self) -> Self { f32::max(self, other) }
    #[inline(always)]
    fn min(self, other: Self) -> Self { f32::min(self, other) }
    #[inline(always)]
    fn sqrt(self) -> Self { f32::sqrt(self) }
    #[inline(always)]
    fn is_nan(self) -> bool { f32::is_nan(self) }
    #[inline(always)]
    fn to_f64(self) -> f64 { self as f64 }
    #[inline(always)]
    fn epsilon() -> Self { 1e-6 }
}

// ---------------------------------------------------------------------------
// Generic SMA
// ---------------------------------------------------------------------------

/// Generic Simple Moving Average supporting both f32 and f64.
#[derive(Clone, Debug)]
pub struct GenericSma<F: Float> {
    period: usize,
    buffer: Vec<F>,
    head: usize,
    len: usize,
    sum: F,
    inv_period: F,
    count: usize,
    last_value: Option<F>,
}

impl<F: Float> GenericSma<F> {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: vec![F::zero(); period],
            head: 0,
            len: 0,
            sum: F::zero(),
            inv_period: F::one() / F::from_usize(period),
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    pub fn next(&mut self, input: F) -> Option<F> {
        self.count += 1;
        self.sum += input;

        if self.len == self.period {
            self.sum -= self.buffer[self.head];
        } else {
            self.len += 1;
        }

        self.buffer[self.head] = input;
        self.head += 1;
        if self.head == self.period {
            self.head = 0;
        }

        let result = if self.len == self.period {
            Some(self.sum * self.inv_period)
        } else {
            None
        };
        self.last_value = result;
        result
    }

    pub fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.sum = F::zero();
        self.count = 0;
        self.last_value = None;
    }

    pub fn is_ready(&self) -> bool {
        self.len >= self.period
    }

    pub fn value(&self) -> Option<F> {
        self.last_value
    }
}

// ---------------------------------------------------------------------------
// Generic EMA
// ---------------------------------------------------------------------------

/// Generic Exponential Moving Average supporting both f32 and f64.
#[derive(Clone, Debug)]
pub struct GenericEma<F: Float> {
    period: usize,
    multiplier: F,
    decay: F,
    inv_period: F,
    ema_value: F,
    count: usize,
    sum: F,
    last_value: Option<F>,
}

impl<F: Float> GenericEma<F> {
    pub fn new(period: usize) -> Self {
        let fp = F::from_usize(period);
        let multiplier = F::two() / (fp + F::one());
        Self {
            period,
            multiplier,
            decay: F::one() - multiplier,
            inv_period: F::one() / fp,
            ema_value: F::nan(),
            count: 0,
            sum: F::zero(),
            last_value: None,
        }
    }

    #[inline]
    pub fn next(&mut self, input: F) -> Option<F> {
        self.count += 1;

        if self.count < self.period {
            self.sum += input;
            self.last_value = None;
            return None;
        }

        if self.count == self.period {
            self.sum += input;
            self.ema_value = self.sum * self.inv_period;
        } else {
            self.ema_value = input * self.multiplier + self.ema_value * self.decay;
        }

        let result = Some(self.ema_value);
        self.last_value = result;
        result
    }

    pub fn reset(&mut self) {
        self.ema_value = F::nan();
        self.count = 0;
        self.sum = F::zero();
        self.last_value = None;
    }

    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    pub fn value(&self) -> Option<F> {
        self.last_value
    }
}

// ---------------------------------------------------------------------------
// Generic RSI
// ---------------------------------------------------------------------------

/// Generic Relative Strength Index supporting both f32 and f64.
#[derive(Clone, Debug)]
pub struct GenericRsi<F: Float> {
    period: usize,
    inv_period: F,
    decay: F,
    avg_gain: F,
    avg_loss: F,
    sum_gain: F,
    sum_loss: F,
    prev_input: F,
    count: usize,
    last_value: Option<F>,
}

impl<F: Float> GenericRsi<F> {
    pub fn new(period: usize) -> Self {
        let inv_period = F::one() / F::from_usize(period);
        Self {
            period,
            inv_period,
            decay: (F::from_usize(period) - F::one()) * inv_period,
            avg_gain: F::zero(),
            avg_loss: F::zero(),
            sum_gain: F::zero(),
            sum_loss: F::zero(),
            prev_input: F::nan(),
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    pub fn next(&mut self, input: F) -> Option<F> {
        self.count += 1;

        if self.count == 1 {
            self.prev_input = input;
            self.last_value = None;
            return None;
        }

        let change = input - self.prev_input;
        self.prev_input = input;
        let gain = Float::max(change, F::zero());
        let loss = Float::max(-change, F::zero());

        if self.count <= self.period + 1 {
            self.sum_gain += gain;
            self.sum_loss += loss;

            if self.count == self.period + 1 {
                self.avg_gain = self.sum_gain * self.inv_period;
                self.avg_loss = self.sum_loss * self.inv_period;
            } else {
                self.last_value = None;
                return None;
            }
        } else {
            self.avg_gain = self.avg_gain * self.decay + gain * self.inv_period;
            self.avg_loss = self.avg_loss * self.decay + loss * self.inv_period;
        }

        let result = if self.avg_loss.abs() < F::epsilon() {
            Some(F::hundred())
        } else {
            let rs = self.avg_gain / self.avg_loss;
            Some(F::hundred() - (F::hundred() / (F::one() + rs)))
        };
        self.last_value = result;
        result
    }

    pub fn reset(&mut self) {
        self.avg_gain = F::zero();
        self.avg_loss = F::zero();
        self.sum_gain = F::zero();
        self.sum_loss = F::zero();
        self.prev_input = F::nan();
        self.count = 0;
        self.last_value = None;
    }

    pub fn is_ready(&self) -> bool {
        self.count > self.period
    }

    pub fn value(&self) -> Option<F> {
        self.last_value
    }
}

// ---------------------------------------------------------------------------
// Generic MACD
// ---------------------------------------------------------------------------

/// Generic MACD output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GenericMacdOutput<F: Float> {
    pub macd: F,
    pub signal: F,
    pub histogram: F,
}

/// Generic MACD supporting both f32 and f64.
#[derive(Clone, Debug)]
pub struct GenericMacd<F: Float> {
    fast_ema: GenericEma<F>,
    slow_ema: GenericEma<F>,
    signal_ema: GenericEma<F>,
    count: usize,
    last_value: Option<GenericMacdOutput<F>>,
}

impl<F: Float> GenericMacd<F> {
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self {
            fast_ema: GenericEma::new(fast_period),
            slow_ema: GenericEma::new(slow_period),
            signal_ema: GenericEma::new(signal_period),
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    pub fn next(&mut self, input: F) -> Option<GenericMacdOutput<F>> {
        self.count += 1;

        let fast = self.fast_ema.next(input);
        let slow = self.slow_ema.next(input);
        let (Some(fast), Some(slow)) = (fast, slow) else {
            self.last_value = None;
            return None;
        };

        let macd = fast - slow;
        let Some(signal) = self.signal_ema.next(macd) else {
            self.last_value = None;
            return None;
        };
        let histogram = macd - signal;

        let result = Some(GenericMacdOutput {
            macd,
            signal,
            histogram,
        });
        self.last_value = result;
        result
    }

    pub fn reset(&mut self) {
        self.fast_ema.reset();
        self.slow_ema.reset();
        self.signal_ema.reset();
        self.count = 0;
        self.last_value = None;
    }

    pub fn is_ready(&self) -> bool {
        self.slow_ema.is_ready() && self.fast_ema.is_ready() && self.signal_ema.is_ready()
    }

    pub fn value(&self) -> Option<GenericMacdOutput<F>> {
        self.last_value
    }
}

// ---------------------------------------------------------------------------
// Generic BOLL
// ---------------------------------------------------------------------------

/// Generic Bollinger Bands output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GenericBollOutput<F: Float> {
    pub upper: F,
    pub middle: F,
    pub lower: F,
}

/// Generic Bollinger Bands supporting both f32 and f64.
#[derive(Clone, Debug)]
pub struct GenericBoll<F: Float> {
    period: usize,
    nb_dev_up: F,
    nb_dev_dn: F,
    buffer: Vec<F>,
    head: usize,
    len: usize,
    sum: F,
    sum_sq: F,
    count: usize,
    inv_n: F,
    inv_n_minus_1: F,
    last_value: Option<GenericBollOutput<F>>,
}

impl<F: Float> GenericBoll<F> {
    pub fn new(period: usize, nb_dev_up: F, nb_dev_dn: F) -> Self {
        let fp = F::from_usize(period);
        Self {
            period,
            nb_dev_up,
            nb_dev_dn,
            buffer: vec![F::zero(); period],
            head: 0,
            len: 0,
            sum: F::zero(),
            sum_sq: F::zero(),
            count: 0,
            inv_n: F::one() / fp,
            inv_n_minus_1: F::one() / (fp - F::one()),
            last_value: None,
        }
    }

    #[inline]
    pub fn next(&mut self, input: F) -> Option<GenericBollOutput<F>> {
        self.count += 1;
        self.sum += input;
        self.sum_sq += input * input;

        if self.len == self.period {
            let old = self.buffer[self.head];
            self.sum -= old;
            self.sum_sq -= old * old;
        } else {
            self.len += 1;
        }

        self.buffer[self.head] = input;
        self.head += 1;
        if self.head == self.period {
            self.head = 0;
        }

        if self.len < self.period {
            self.last_value = None;
            return None;
        }

        let mean = self.sum * self.inv_n;
        let variance = (self.sum_sq - self.sum * mean) * self.inv_n_minus_1;
        let std_dev = Float::max(variance, F::zero()).sqrt();

        let result = Some(GenericBollOutput {
            middle: mean,
            upper: mean + std_dev * self.nb_dev_up,
            lower: mean - std_dev * self.nb_dev_dn,
        });
        self.last_value = result;
        result
    }

    pub fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.sum = F::zero();
        self.sum_sq = F::zero();
        self.count = 0;
        self.last_value = None;
    }

    pub fn is_ready(&self) -> bool {
        self.len >= self.period
    }

    pub fn value(&self) -> Option<GenericBollOutput<F>> {
        self.last_value
    }
}

// ---------------------------------------------------------------------------
// Generic ATR
// ---------------------------------------------------------------------------

/// Generic Average True Range supporting both f32 and f64.
#[derive(Clone, Debug)]
pub struct GenericAtr<F: Float> {
    ema: GenericEma<F>,
    prev_close: F,
    count: usize,
    last_value: Option<F>,
}

impl<F: Float> GenericAtr<F> {
    pub fn new(period: usize) -> Self {
        Self {
            ema: GenericEma::new(period),
            prev_close: F::nan(),
            count: 0,
            last_value: None,
        }
    }

    /// Input: (high, low, close)
    #[inline]
    pub fn next(&mut self, input: (F, F, F)) -> Option<F> {
        let (high, low, close) = input;
        self.count += 1;

        let tr = if self.count == 1 {
            high - low
        } else {
            let hl = high - low;
            let hpc = (high - self.prev_close).abs();
            let lpc = (low - self.prev_close).abs();
            Float::max(hl, Float::max(hpc, lpc))
        };
        self.prev_close = close;

        let result = self.ema.next(tr);
        self.last_value = result;
        result
    }

    pub fn reset(&mut self) {
        self.ema.reset();
        self.prev_close = F::nan();
        self.count = 0;
        self.last_value = None;
    }

    pub fn is_ready(&self) -> bool {
        self.ema.is_ready()
    }

    pub fn value(&self) -> Option<F> {
        self.last_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_float_trait_f64_basics() {
        assert_eq!(f64::zero(), 0.0);
        assert_eq!(f64::one(), 1.0);
        assert_eq!(f64::from_usize(42), 42.0);
        assert!(f64::nan().is_nan());
    }

    #[test]
    fn test_float_trait_f32_basics() {
        assert_eq!(f32::zero(), 0.0_f32);
        assert_eq!(f32::one(), 1.0_f32);
        assert_eq!(f32::from_usize(42), 42.0_f32);
        assert!(f32::nan().is_nan());
    }

    #[test]
    fn test_generic_sma_f64() {
        let mut sma = GenericSma::<f64>::new(3);
        assert_eq!(sma.next(1.0), None);
        assert_eq!(sma.next(2.0), None);
        assert!((sma.next(3.0).unwrap() - 2.0).abs() < 1e-10);
        assert!((sma.next(4.0).unwrap() - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_generic_sma_f32() {
        let mut sma = GenericSma::<f32>::new(3);
        assert_eq!(sma.next(1.0_f32), None);
        assert_eq!(sma.next(2.0_f32), None);
        let val = sma.next(3.0_f32).unwrap();
        assert!((val - 2.0_f32).abs() < 1e-4);
    }

    #[test]
    fn test_generic_ema_f64() {
        let mut ema = GenericEma::<f64>::new(3);
        assert_eq!(ema.next(2.0), None);
        assert_eq!(ema.next(4.0), None);
        let v3 = ema.next(6.0).unwrap();
        assert!((v3 - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_generic_ema_f32() {
        let mut ema = GenericEma::<f32>::new(3);
        assert_eq!(ema.next(2.0_f32), None);
        assert_eq!(ema.next(4.0_f32), None);
        let v3 = ema.next(6.0_f32).unwrap();
        assert!((v3 - 4.0_f32).abs() < 1e-4);
    }

    #[test]
    fn test_generic_rsi_f64() {
        let mut rsi = GenericRsi::<f64>::new(5);
        for i in 0..7 {
            rsi.next(10.0 + i as f64);
        }
        let val = rsi.next(20.0).unwrap();
        assert!(val > 50.0 && val <= 100.0);
    }

    #[test]
    fn test_generic_rsi_f32() {
        let mut rsi = GenericRsi::<f32>::new(5);
        for i in 0..7 {
            rsi.next(10.0_f32 + i as f32);
        }
        let val = rsi.next(20.0_f32).unwrap();
        assert!(val > 50.0_f32 && val <= 100.0_f32);
    }

    #[test]
    fn test_generic_macd_f64() {
        let mut macd = GenericMacd::<f64>::new(3, 5, 3);
        let mut ready = false;
        for i in 1..=10 {
            if let Some(out) = macd.next(i as f64) {
                assert!(!out.macd.is_nan());
                ready = true;
            }
        }
        assert!(ready);
    }

    #[test]
    fn test_generic_macd_f32() {
        let mut macd = GenericMacd::<f32>::new(3, 5, 3);
        let mut ready = false;
        for i in 1..=10 {
            if let Some(out) = macd.next(i as f32) {
                assert!(!out.macd.is_nan());
                ready = true;
            }
        }
        assert!(ready);
    }

    #[test]
    fn test_generic_boll_f32() {
        let mut boll = GenericBoll::<f32>::new(5, 2.0_f32, 2.0_f32);
        for i in 1..=5 {
            let out = boll.next(i as f32);
            if i == 5 {
                let out = out.unwrap();
                assert!(out.upper > out.middle);
                assert!(out.lower < out.middle);
            }
        }
    }

    #[test]
    fn test_generic_atr_f32() {
        let mut atr = GenericAtr::<f32>::new(3);
        assert_eq!(atr.next((12.0_f32, 10.0_f32, 11.0_f32)), None);
        assert_eq!(atr.next((13.0_f32, 11.0_f32, 12.0_f32)), None);
        let val = atr.next((14.0_f32, 12.0_f32, 13.0_f32)).unwrap();
        assert!(val > 0.0_f32);
    }

    #[test]
    fn test_generic_sma_f64_matches_concrete() {
        use crate::streaming::indicators::StreamingSma;
        use crate::streaming::StreamingIndicator;

        let mut concrete = StreamingSma::new(5);
        let mut generic = GenericSma::<f64>::new(5);

        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        for &v in &data {
            let a = concrete.next(v);
            let b = generic.next(v);
            assert_eq!(a, b);
        }
    }
}
