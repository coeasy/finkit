//! Streaming Math Transform Indicators
//!
//! Provides O(1) incremental streaming versions for all 15 TA-Lib Math Transform functions.
//! All of these functions are *stateless element-wise operations* -- each `next(value)` immediately
//! returns `Some(transformed_value)`, with no warm-up accumulation needed.
//!
//! # Domain Error Handling
//! For functions with restricted domains (`acos`/`asin` input must be in `[-1, 1]`, `ln`/`log10`/`sqrt`
//! input must be non-negative), out-of-domain inputs return `None` (streaming interface convention).
//! Callers should decide whether to filter based on business requirements.
//!
//! # Function List
//! - Trigonometric: `acos` / `asin` / `atan` / `cos` / `sin` / `tan`
//! - Hyperbolic: `cosh` / `sinh` / `tanh`
//! - Exponential/Logarithmic: `exp` / `ln` / `log10`
//! - Rounding: `ceil` / `floor`
//! - Power: `sqrt`

use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

// =====================================================================
// Trigonometric functions (no domain constraints / partial domain constraints)
// =====================================================================

/// Streaming Arc Cosine (acos)
///
/// Input must be in `[-1, 1]`; returns `None` outside domain.
#[derive(Clone, Default)]
pub struct StreamingAcos {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingAcos {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingAcos {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        if !input.is_finite() || input < -1.0 || input > 1.0 {
            self.last_value = None;
            return None;
        }
        let v = input.acos();
        self.last_value = Some(v);
        Some(v)
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        true
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingAcos {
    fn name() -> &'static str {
        "ACOS"
    }
    fn category() -> &'static str {
        "math_transform"
    }
    fn description() -> &'static str {
        "Vector Arc Cosine"
    }
    fn warm_up_period(&self) -> usize {
        0
    }
}

/// Streaming Arc Sine (asin)
///
/// Input must be in `[-1, 1]`; returns `None` outside domain.
#[derive(Clone, Default)]
pub struct StreamingAsin {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingAsin {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingAsin {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        if !input.is_finite() || input < -1.0 || input > 1.0 {
            self.last_value = None;
            return None;
        }
        let v = input.asin();
        self.last_value = Some(v);
        Some(v)
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        true
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingAsin {
    fn name() -> &'static str {
        "ASIN"
    }
    fn category() -> &'static str {
        "math_transform"
    }
    fn description() -> &'static str {
        "Vector Arc Sine"
    }
    fn warm_up_period(&self) -> usize {
        0
    }
}

/// Streaming Arc Tangent (atan)
#[derive(Clone, Default)]
pub struct StreamingAtan {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingAtan {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingAtan {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let v = input.atan();
        self.last_value = Some(v);
        Some(v)
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        true
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingAtan {
    fn name() -> &'static str {
        "ATAN"
    }
    fn category() -> &'static str {
        "math_transform"
    }
    fn description() -> &'static str {
        "Vector Arc Tangent"
    }
    fn warm_up_period(&self) -> usize {
        0
    }
}

/// Streaming Cosine (cos)
#[derive(Clone, Default)]
pub struct StreamingCos {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingCos {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingCos {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let v = input.cos();
        self.last_value = Some(v);
        Some(v)
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        true
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingCos {
    fn name() -> &'static str {
        "COS"
    }
    fn category() -> &'static str {
        "math_transform"
    }
    fn description() -> &'static str {
        "Vector Cosine"
    }
    fn warm_up_period(&self) -> usize {
        0
    }
}

/// Streaming Hyperbolic Cosine (cosh)
#[derive(Clone, Default)]
pub struct StreamingCosh {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingCosh {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingCosh {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let v = input.cosh();
        self.last_value = Some(v);
        Some(v)
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        true
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingCosh {
    fn name() -> &'static str {
        "COSH"
    }
    fn category() -> &'static str {
        "math_transform"
    }
    fn description() -> &'static str {
        "Vector Hyperbolic Cosine"
    }
    fn warm_up_period(&self) -> usize {
        0
    }
}

/// Streaming Sine (sin)
#[derive(Clone, Default)]
pub struct StreamingSin {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingSin {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingSin {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let v = input.sin();
        self.last_value = Some(v);
        Some(v)
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        true
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingSin {
    fn name() -> &'static str {
        "SIN"
    }
    fn category() -> &'static str {
        "math_transform"
    }
    fn description() -> &'static str {
        "Vector Sine"
    }
    fn warm_up_period(&self) -> usize {
        0
    }
}

/// Streaming Hyperbolic Sine (sinh)
#[derive(Clone, Default)]
pub struct StreamingSinh {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingSinh {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingSinh {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let v = input.sinh();
        self.last_value = Some(v);
        Some(v)
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        true
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingSinh {
    fn name() -> &'static str {
        "SINH"
    }
    fn category() -> &'static str {
        "math_transform"
    }
    fn description() -> &'static str {
        "Vector Hyperbolic Sine"
    }
    fn warm_up_period(&self) -> usize {
        0
    }
}

/// Streaming Tangent (tan)
#[derive(Clone, Default)]
pub struct StreamingTan {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingTan {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingTan {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let v = input.tan();
        self.last_value = Some(v);
        Some(v)
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        true
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingTan {
    fn name() -> &'static str {
        "TAN"
    }
    fn category() -> &'static str {
        "math_transform"
    }
    fn description() -> &'static str {
        "Vector Tangent"
    }
    fn warm_up_period(&self) -> usize {
        0
    }
}

/// Streaming Hyperbolic Tangent (tanh)
#[derive(Clone, Default)]
pub struct StreamingTanh {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingTanh {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingTanh {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let v = input.tanh();
        self.last_value = Some(v);
        Some(v)
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        true
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingTanh {
    fn name() -> &'static str {
        "TANH"
    }
    fn category() -> &'static str {
        "math_transform"
    }
    fn description() -> &'static str {
        "Vector Hyperbolic Tangent"
    }
    fn warm_up_period(&self) -> usize {
        0
    }
}

// =====================================================================
// Exponential and Logarithmic
// =====================================================================

/// Streaming Exponential (exp)
#[derive(Clone, Default)]
pub struct StreamingExp {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingExp {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingExp {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let v = input.exp();
        self.last_value = Some(v);
        Some(v)
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        true
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingExp {
    fn name() -> &'static str {
        "EXP"
    }
    fn category() -> &'static str {
        "math_transform"
    }
    fn description() -> &'static str {
        "Vector Exponential"
    }
    fn warm_up_period(&self) -> usize {
        0
    }
}

/// Streaming Natural Logarithm (ln)
///
/// Input must be > 0; non-positive or non-finite inputs return `None`.
#[derive(Clone, Default)]
pub struct StreamingLn {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingLn {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingLn {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        if !input.is_finite() || input <= 0.0 {
            self.last_value = None;
            return None;
        }
        let v = input.ln();
        self.last_value = Some(v);
        Some(v)
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        true
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingLn {
    fn name() -> &'static str {
        "LN"
    }
    fn category() -> &'static str {
        "math_transform"
    }
    fn description() -> &'static str {
        "Vector Natural Logarithm"
    }
    fn warm_up_period(&self) -> usize {
        0
    }
}

/// Streaming Base-10 Logarithm (log10)
///
/// Input must be > 0; non-positive or non-finite inputs return `None`.
#[derive(Clone, Default)]
pub struct StreamingLog10 {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingLog10 {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingLog10 {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        if !input.is_finite() || input <= 0.0 {
            self.last_value = None;
            return None;
        }
        let v = input.log10();
        self.last_value = Some(v);
        Some(v)
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        true
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingLog10 {
    fn name() -> &'static str {
        "LOG10"
    }
    fn category() -> &'static str {
        "math_transform"
    }
    fn description() -> &'static str {
        "Vector Base-10 Logarithm"
    }
    fn warm_up_period(&self) -> usize {
        0
    }
}

// =====================================================================
// Rounding and Power functions
// =====================================================================

/// Streaming Ceiling (ceil)
#[derive(Clone, Default)]
pub struct StreamingCeil {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingCeil {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingCeil {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let v = input.ceil();
        self.last_value = Some(v);
        Some(v)
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        true
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingCeil {
    fn name() -> &'static str {
        "CEIL"
    }
    fn category() -> &'static str {
        "math_transform"
    }
    fn description() -> &'static str {
        "Vector Ceiling"
    }
    fn warm_up_period(&self) -> usize {
        0
    }
}

/// Streaming Floor (floor)
#[derive(Clone, Default)]
pub struct StreamingFloor {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingFloor {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingFloor {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let v = input.floor();
        self.last_value = Some(v);
        Some(v)
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        true
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingFloor {
    fn name() -> &'static str {
        "FLOOR"
    }
    fn category() -> &'static str {
        "math_transform"
    }
    fn description() -> &'static str {
        "Vector Floor"
    }
    fn warm_up_period(&self) -> usize {
        0
    }
}

/// Streaming Square Root (sqrt)
///
/// Input must be >= 0; negative or non-finite inputs return `None`.
#[derive(Clone, Default)]
pub struct StreamingSqrt {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingSqrt {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingSqrt {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        if !input.is_finite() || input < 0.0 {
            self.last_value = None;
            return None;
        }
        let v = input.sqrt();
        self.last_value = Some(v);
        Some(v)
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        true
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingSqrt {
    fn name() -> &'static str {
        "SQRT"
    }
    fn category() -> &'static str {
        "math_transform"
    }
    fn description() -> &'static str {
        "Vector Square Root"
    }
    fn warm_up_period(&self) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-10;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    // ---------- ACOS ----------
    #[test]
    fn test_streaming_acos_basic() {
        let mut ind = StreamingAcos::new();
        assert!(ind.is_ready()); // no warm-up
        let v = ind.next(1.0).unwrap();
        assert!(close(v, 0.0));
        let v = ind.next(0.0).unwrap();
        assert!(close(v, std::f64::consts::FRAC_PI_2));
        let v = ind.next(-1.0).unwrap();
        assert!(close(v, std::f64::consts::PI));
        assert_eq!(ind.count(), 3);
        assert_eq!(ind.value(), Some(std::f64::consts::PI));
    }

    #[test]
    fn test_streaming_acos_domain_error() {
        let mut ind = StreamingAcos::new();
        assert_eq!(ind.next(2.0), None);
        assert_eq!(ind.next(-2.0), None);
    }

    #[test]
    fn test_streaming_acos_meta() {
        assert_eq!(StreamingAcos::name(), "ACOS");
        assert_eq!(StreamingAcos::category(), "math_transform");
    }

    #[test]
    fn test_streaming_acos_reset() {
        let mut ind = StreamingAcos::new();
        ind.next(0.5);
        assert_eq!(ind.count(), 1);
        ind.reset();
        assert_eq!(ind.count(), 0);
        assert_eq!(ind.value(), None);
    }

    // ---------- ASIN ----------
    #[test]
    fn test_streaming_asin_basic() {
        let mut ind = StreamingAsin::new();
        assert!(ind.is_ready());
        let v = ind.next(0.0).unwrap();
        assert!(close(v, 0.0));
        let v = ind.next(1.0).unwrap();
        assert!(close(v, std::f64::consts::FRAC_PI_2));
    }

    #[test]
    fn test_streaming_asin_domain_error() {
        let mut ind = StreamingAsin::new();
        assert_eq!(ind.next(2.0), None);
    }

    #[test]
    fn test_streaming_asin_meta() {
        assert_eq!(StreamingAsin::name(), "ASIN");
        assert_eq!(StreamingAsin::category(), "math_transform");
    }

    #[test]
    fn test_streaming_asin_reset() {
        let mut ind = StreamingAsin::new();
        ind.next(0.5);
        ind.reset();
        assert_eq!(ind.value(), None);
    }

    // ---------- ATAN ----------
    #[test]
    fn test_streaming_atan_basic() {
        let mut ind = StreamingAtan::new();
        assert!(ind.is_ready());
        let v = ind.next(0.0).unwrap();
        assert!(close(v, 0.0));
        let v = ind.next(1.0).unwrap();
        assert!(close(v, std::f64::consts::FRAC_PI_4));
    }

    #[test]
    fn test_streaming_atan_meta() {
        assert_eq!(StreamingAtan::name(), "ATAN");
        assert_eq!(StreamingAtan::category(), "math_transform");
    }

    #[test]
    fn test_streaming_atan_reset() {
        let mut ind = StreamingAtan::new();
        ind.next(1.0);
        ind.reset();
        assert_eq!(ind.count(), 0);
    }

    // ---------- COS ----------
    #[test]
    fn test_streaming_cos_basic() {
        let mut ind = StreamingCos::new();
        assert!(ind.is_ready());
        let v = ind.next(0.0).unwrap();
        assert!(close(v, 1.0));
    }

    #[test]
    fn test_streaming_cos_meta() {
        assert_eq!(StreamingCos::name(), "COS");
        assert_eq!(StreamingCos::category(), "math_transform");
    }

    #[test]
    fn test_streaming_cos_reset() {
        let mut ind = StreamingCos::new();
        ind.next(0.5);
        ind.reset();
        assert_eq!(ind.value(), None);
    }

    // ---------- COSH ----------
    #[test]
    fn test_streaming_cosh_basic() {
        let mut ind = StreamingCosh::new();
        assert!(ind.is_ready());
        let v = ind.next(0.0).unwrap();
        assert!(close(v, 1.0));
    }

    #[test]
    fn test_streaming_cosh_meta() {
        assert_eq!(StreamingCosh::name(), "COSH");
    }

    #[test]
    fn test_streaming_cosh_reset() {
        let mut ind = StreamingCosh::new();
        ind.next(1.0);
        ind.reset();
        assert_eq!(ind.count(), 0);
    }

    // ---------- EXP ----------
    #[test]
    fn test_streaming_exp_basic() {
        let mut ind = StreamingExp::new();
        assert!(ind.is_ready());
        let v = ind.next(0.0).unwrap();
        assert!(close(v, 1.0));
        let v = ind.next(1.0).unwrap();
        assert!(close(v, std::f64::consts::E));
    }

    #[test]
    fn test_streaming_exp_meta() {
        assert_eq!(StreamingExp::name(), "EXP");
    }

    #[test]
    fn test_streaming_exp_reset() {
        let mut ind = StreamingExp::new();
        ind.next(1.0);
        ind.reset();
        assert_eq!(ind.value(), None);
    }

    // ---------- FLOOR ----------
    #[test]
    fn test_streaming_floor_basic() {
        let mut ind = StreamingFloor::new();
        assert!(ind.is_ready());
        let v = ind.next(1.7).unwrap();
        assert!(close(v, 1.0));
        let v = ind.next(-1.2).unwrap();
        assert!(close(v, -2.0));
    }

    #[test]
    fn test_streaming_floor_meta() {
        assert_eq!(StreamingFloor::name(), "FLOOR");
    }

    #[test]
    fn test_streaming_floor_reset() {
        let mut ind = StreamingFloor::new();
        ind.next(1.5);
        ind.reset();
        assert_eq!(ind.count(), 0);
    }

    // ---------- LN ----------
    #[test]
    fn test_streaming_ln_basic() {
        let mut ind = StreamingLn::new();
        assert!(ind.is_ready());
        let v = ind.next(1.0).unwrap();
        assert!(close(v, 0.0));
        let v = ind.next(std::f64::consts::E).unwrap();
        assert!(close(v, 1.0));
    }

    #[test]
    fn test_streaming_ln_domain_error() {
        let mut ind = StreamingLn::new();
        assert_eq!(ind.next(0.0), None);
        assert_eq!(ind.next(-1.0), None);
    }

    #[test]
    fn test_streaming_ln_meta() {
        assert_eq!(StreamingLn::name(), "LN");
    }

    #[test]
    fn test_streaming_ln_reset() {
        let mut ind = StreamingLn::new();
        ind.next(2.0);
        ind.reset();
        assert_eq!(ind.value(), None);
    }

    // ---------- LOG10 ----------
    #[test]
    fn test_streaming_log10_basic() {
        let mut ind = StreamingLog10::new();
        assert!(ind.is_ready());
        let v = ind.next(1.0).unwrap();
        assert!(close(v, 0.0));
        let v = ind.next(10.0).unwrap();
        assert!(close(v, 1.0));
        let v = ind.next(100.0).unwrap();
        assert!(close(v, 2.0));
    }

    #[test]
    fn test_streaming_log10_domain_error() {
        let mut ind = StreamingLog10::new();
        assert_eq!(ind.next(0.0), None);
        assert_eq!(ind.next(-10.0), None);
    }

    #[test]
    fn test_streaming_log10_meta() {
        assert_eq!(StreamingLog10::name(), "LOG10");
    }

    #[test]
    fn test_streaming_log10_reset() {
        let mut ind = StreamingLog10::new();
        ind.next(10.0);
        ind.reset();
        assert_eq!(ind.count(), 0);
    }

    // ---------- SIN ----------
    #[test]
    fn test_streaming_sin_basic() {
        let mut ind = StreamingSin::new();
        assert!(ind.is_ready());
        let v = ind.next(0.0).unwrap();
        assert!(close(v, 0.0));
    }

    #[test]
    fn test_streaming_sin_meta() {
        assert_eq!(StreamingSin::name(), "SIN");
    }

    #[test]
    fn test_streaming_sin_reset() {
        let mut ind = StreamingSin::new();
        ind.next(0.5);
        ind.reset();
        assert_eq!(ind.value(), None);
    }

    // ---------- SINH ----------
    #[test]
    fn test_streaming_sinh_basic() {
        let mut ind = StreamingSinh::new();
        assert!(ind.is_ready());
        let v = ind.next(0.0).unwrap();
        assert!(close(v, 0.0));
    }

    #[test]
    fn test_streaming_sinh_meta() {
        assert_eq!(StreamingSinh::name(), "SINH");
    }

    #[test]
    fn test_streaming_sinh_reset() {
        let mut ind = StreamingSinh::new();
        ind.next(1.0);
        ind.reset();
        assert_eq!(ind.count(), 0);
    }

    // ---------- SQRT ----------
    #[test]
    fn test_streaming_sqrt_basic() {
        let mut ind = StreamingSqrt::new();
        assert!(ind.is_ready());
        let v = ind.next(0.0).unwrap();
        assert!(close(v, 0.0));
        let v = ind.next(4.0).unwrap();
        assert!(close(v, 2.0));
        let v = ind.next(9.0).unwrap();
        assert!(close(v, 3.0));
    }

    #[test]
    fn test_streaming_sqrt_domain_error() {
        let mut ind = StreamingSqrt::new();
        assert_eq!(ind.next(-1.0), None);
    }

    #[test]
    fn test_streaming_sqrt_meta() {
        assert_eq!(StreamingSqrt::name(), "SQRT");
    }

    #[test]
    fn test_streaming_sqrt_reset() {
        let mut ind = StreamingSqrt::new();
        ind.next(16.0);
        ind.reset();
        assert_eq!(ind.value(), None);
    }

    // ---------- TAN ----------
    #[test]
    fn test_streaming_tan_basic() {
        let mut ind = StreamingTan::new();
        assert!(ind.is_ready());
        let v = ind.next(0.0).unwrap();
        assert!(close(v, 0.0));
    }

    #[test]
    fn test_streaming_tan_meta() {
        assert_eq!(StreamingTan::name(), "TAN");
    }

    #[test]
    fn test_streaming_tan_reset() {
        let mut ind = StreamingTan::new();
        ind.next(0.5);
        ind.reset();
        assert_eq!(ind.count(), 0);
    }

    // ---------- TANH ----------
    #[test]
    fn test_streaming_tanh_basic() {
        let mut ind = StreamingTanh::new();
        assert!(ind.is_ready());
        let v = ind.next(0.0).unwrap();
        assert!(close(v, 0.0));
    }

    #[test]
    fn test_streaming_tanh_meta() {
        assert_eq!(StreamingTanh::name(), "TANH");
    }

    #[test]
    fn test_streaming_tanh_reset() {
        let mut ind = StreamingTanh::new();
        ind.next(1.0);
        ind.reset();
        assert_eq!(ind.value(), None);
    }

    // ---------- CEIL ----------
    #[test]
    fn test_streaming_ceil_basic() {
        let mut ind = StreamingCeil::new();
        assert!(ind.is_ready());
        let v = ind.next(1.2).unwrap();
        assert!(close(v, 2.0));
        let v = ind.next(-1.7).unwrap();
        assert!(close(v, -1.0));
    }

    #[test]
    fn test_streaming_ceil_meta() {
        assert_eq!(StreamingCeil::name(), "CEIL");
    }

    #[test]
    fn test_streaming_ceil_reset() {
        let mut ind = StreamingCeil::new();
        ind.next(1.5);
        ind.reset();
        assert_eq!(ind.count(), 0);
    }

    // ---------- Cross-validation with batch implementation ----------
    fn assert_streaming_matches_batch(
        _data: &[f64],
        streaming: Vec<f64>,
        batch: &ndarray::Array1<f64>,
        name: &str,
    ) {
        assert_eq!(streaming.len(), batch.len(), "{name}: length mismatch");
        for (i, (s, b)) in streaming.iter().zip(batch.iter()).enumerate() {
            assert!(
                (s - b).abs() < 1e-10,
                "{name} mismatch at {i}: streaming={s} batch={b}"
            );
        }
    }

    #[test]
    fn test_streaming_vs_batch_cos() {
        let data: Vec<f64> = vec![
            0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 0.15, 0.25, 0.35, 0.45, 0.55, 0.65,
            0.75, 0.85, 0.95, 0.05,
        ];
        let mut ind = StreamingCos::new();
        let streaming: Vec<f64> = data.iter().map(|&v| ind.next(v).unwrap()).collect();
        let batch = crate::indicators::math_transform::cos(&data).unwrap();
        assert_streaming_matches_batch(&data, streaming, &batch, "cos");
    }

    #[test]
    fn test_streaming_vs_batch_sin() {
        let data: Vec<f64> = vec![
            0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 0.15, 0.25, 0.35, 0.45, 0.55, 0.65,
            0.75, 0.85, 0.95, 0.05,
        ];
        let mut ind = StreamingSin::new();
        let streaming: Vec<f64> = data.iter().map(|&v| ind.next(v).unwrap()).collect();
        let batch = crate::indicators::math_transform::sin(&data).unwrap();
        assert_streaming_matches_batch(&data, streaming, &batch, "sin");
    }

    #[test]
    fn test_streaming_vs_batch_tan() {
        let data: Vec<f64> = vec![
            0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 0.15, 0.25, 0.35, 0.45, 0.55, 0.65,
            0.75, 0.85, 0.95, 0.05,
        ];
        let mut ind = StreamingTan::new();
        let streaming: Vec<f64> = data.iter().map(|&v| ind.next(v).unwrap()).collect();
        let batch = crate::indicators::math_transform::tan(&data).unwrap();
        assert_streaming_matches_batch(&data, streaming, &batch, "tan");
    }

    #[test]
    fn test_streaming_vs_batch_exp() {
        let data: Vec<f64> = vec![
            0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 0.15, 0.25, 0.35, 0.45, 0.55, 0.65,
            0.75, 0.85, 0.95, 0.05,
        ];
        let mut ind = StreamingExp::new();
        let streaming: Vec<f64> = data.iter().map(|&v| ind.next(v).unwrap()).collect();
        let batch = crate::indicators::math_transform::exp(&data).unwrap();
        assert_streaming_matches_batch(&data, streaming, &batch, "exp");
    }
}
