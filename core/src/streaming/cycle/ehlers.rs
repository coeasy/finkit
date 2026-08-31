//! Streaming implementations of Ehlers Digital Signal Processing filters.

use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

// ============================================================================
// StreamingSuperSmoother —2-pole
// ============================================================================

/// Streaming 2-pole Super Smoother Filter (Ehlers).
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingSuperSmoother {
    c1: f64,
    c2: f64,
    c3: f64,
    prev1: f64,
    prev2: f64,
    prev_input: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingSuperSmoother {
    pub fn new(period: usize) -> Self {
        let p = period.max(2) as f64;
        let a1 = (-std::f64::consts::SQRT_2 * std::f64::consts::PI / p).exp();
        let b1 = 2.0 * a1 * (std::f64::consts::SQRT_2 * std::f64::consts::PI / p).cos();
        let c2 = b1;
        let c3 = -a1 * a1;
        let c1 = 1.0 - c2 - c3;
        Self {
            c1,
            c2,
            c3,
            prev1: 0.0,
            prev2: 0.0,
            prev_input: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl Default for StreamingSuperSmoother {
    fn default() -> Self {
        Self::new(10)
    }
}

impl StreamingIndicator for StreamingSuperSmoother {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let val = if self.count <= 2 {
            input
        } else {
            self.c1 * (input + self.prev_input) / 2.0 + self.c2 * self.prev1 + self.c3 * self.prev2
        };
        self.prev_input = input;
        self.prev2 = self.prev1;
        self.prev1 = val;
        let result = Some(val);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.prev1 = 0.0;
        self.prev2 = 0.0;
        self.prev_input = 0.0;
        self.count = 0;
        self.last_value = None;
    }
    fn is_ready(&self) -> bool {
        self.count >= 3
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingSuperSmoother {
    fn name() -> &'static str {
        "SUPER_SMOOTHER"
    }
    fn category() -> &'static str {
        "cycle"
    }
    fn description() -> &'static str {
        "Ehlers 2-pole Super Smoother Filter"
    }
    fn warm_up_period(&self) -> usize {
        3
    }
}

// ============================================================================
// StreamingSuperSmoother3Pole —3-pole
// ============================================================================

/// Streaming 3-pole Super Smoother Filter (Ehlers).
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingSuperSmoother3Pole {
    coef1: f64,
    coef2: f64,
    coef3: f64,
    coef4: f64,
    prev1: f64,
    prev2: f64,
    prev3: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingSuperSmoother3Pole {
    pub fn new(period: usize) -> Self {
        let p = period.max(2) as f64;
        let a1 = (-std::f64::consts::PI / p).exp();
        let b1 = 2.0 * a1 * (std::f64::consts::PI * 1.738 / p).cos();
        let c1 = a1 * a1;
        let coef2 = b1 + c1;
        let coef3 = -(c1 + b1 * c1);
        let coef4 = c1 * c1;
        let coef1 = 1.0 - coef2 - coef3 - coef4;
        Self {
            coef1,
            coef2,
            coef3,
            coef4,
            prev1: 0.0,
            prev2: 0.0,
            prev3: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl Default for StreamingSuperSmoother3Pole {
    fn default() -> Self {
        Self::new(10)
    }
}

impl StreamingIndicator for StreamingSuperSmoother3Pole {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let val = if self.count <= 3 {
            input
        } else {
            self.coef1 * input
                + self.coef2 * self.prev1
                + self.coef3 * self.prev2
                + self.coef4 * self.prev3
        };
        self.prev3 = self.prev2;
        self.prev2 = self.prev1;
        self.prev1 = val;
        let result = Some(val);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.prev1 = 0.0;
        self.prev2 = 0.0;
        self.prev3 = 0.0;
        self.count = 0;
        self.last_value = None;
    }
    fn is_ready(&self) -> bool {
        self.count >= 4
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingSuperSmoother3Pole {
    fn name() -> &'static str {
        "SUPER_SMOOTHER_3POLE"
    }
    fn category() -> &'static str {
        "cycle"
    }
    fn description() -> &'static str {
        "Ehlers 3-pole Super Smoother Filter"
    }
    fn warm_up_period(&self) -> usize {
        4
    }
}

// ============================================================================
// StreamingRoofingFilter
// ============================================================================

/// Streaming Roofing Filter (Ehlers): highpass + super smoother.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingRoofingFilter {
    // High-pass coefficients
    hp_coef: f64,
    alpha_hp: f64,
    // Super smoother coefficients
    c1: f64,
    c2: f64,
    c3: f64,
    // State
    prev_input: f64,
    hp_prev1: f64,
    hp_prev2: f64,
    ss_prev1: f64,
    ss_prev2: f64,
    hp_prev_input: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingRoofingFilter {
    pub fn new(hp_period: usize, lp_period: usize) -> Self {
        let hp_p = hp_period.max(2) as f64;
        let lp_p = lp_period.max(2) as f64;

        let alpha_hp = (0.707 * 2.0 * std::f64::consts::PI / hp_p).cos();
        let hp_coef = (1.0 + alpha_hp) / 2.0;

        let a1 = (-std::f64::consts::SQRT_2 * std::f64::consts::PI / lp_p).exp();
        let b1 = 2.0 * a1 * (std::f64::consts::SQRT_2 * std::f64::consts::PI / lp_p).cos();
        let c2 = b1;
        let c3 = -a1 * a1;
        let c1 = 1.0 - c2 - c3;

        Self {
            hp_coef,
            alpha_hp,
            c1,
            c2,
            c3,
            prev_input: 0.0,
            hp_prev1: 0.0,
            hp_prev2: 0.0,
            ss_prev1: 0.0,
            ss_prev2: 0.0,
            hp_prev_input: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl Default for StreamingRoofingFilter {
    fn default() -> Self {
        Self::new(48, 10)
    }
}

impl StreamingIndicator for StreamingRoofingFilter {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;

        let hp = if self.count == 1 {
            input
        } else if self.count == 2 {
            self.hp_coef * (input - self.prev_input)
        } else {
            self.hp_coef * (input - self.prev_input) + (2.0 * self.alpha_hp - 1.0) * self.hp_prev1
                - (self.alpha_hp * self.alpha_hp - 2.0 * self.alpha_hp + 1.0) * self.hp_prev2
        };

        let ss = if self.count <= 2 {
            hp
        } else {
            self.c1 * (hp + self.hp_prev_input) / 2.0
                + self.c2 * self.ss_prev1
                + self.c3 * self.ss_prev2
        };

        self.prev_input = input;
        self.hp_prev2 = self.hp_prev1;
        self.hp_prev1 = hp;
        self.hp_prev_input = hp;
        self.ss_prev2 = self.ss_prev1;
        self.ss_prev1 = ss;

        let result = Some(ss);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.prev_input = 0.0;
        self.hp_prev1 = 0.0;
        self.hp_prev2 = 0.0;
        self.ss_prev1 = 0.0;
        self.ss_prev2 = 0.0;
        self.hp_prev_input = 0.0;
        self.count = 0;
        self.last_value = None;
    }
    fn is_ready(&self) -> bool {
        self.count >= 3
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingRoofingFilter {
    fn name() -> &'static str {
        "ROOFING_FILTER"
    }
    fn category() -> &'static str {
        "cycle"
    }
    fn description() -> &'static str {
        "Ehlers Roofing Filter (HP + Super Smoother)"
    }
    fn warm_up_period(&self) -> usize {
        3
    }
}

// ============================================================================
// StreamingDecycler
// ============================================================================

/// Streaming Decycler (Ehlers): removes cycle, keeps trend.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingDecycler {
    hp_coef: f64,
    alpha_hp: f64,
    prev_input: f64,
    hp_prev1: f64,
    hp_prev2: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingDecycler {
    pub fn new(hp_period: usize) -> Self {
        let hp_p = hp_period.max(2) as f64;
        let alpha_hp = (0.707 * 2.0 * std::f64::consts::PI / hp_p).cos();
        let hp_coef = (1.0 + alpha_hp) / 2.0;
        Self {
            hp_coef,
            alpha_hp,
            prev_input: 0.0,
            hp_prev1: 0.0,
            hp_prev2: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl Default for StreamingDecycler {
    fn default() -> Self {
        Self::new(20)
    }
}

impl StreamingIndicator for StreamingDecycler {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;

        let hp = if self.count == 1 {
            0.0
        } else if self.count == 2 {
            self.hp_coef * (input - self.prev_input)
        } else {
            self.hp_coef * (input - self.prev_input) + (2.0 * self.alpha_hp - 1.0) * self.hp_prev1
                - (self.alpha_hp * self.alpha_hp - 2.0 * self.alpha_hp + 1.0) * self.hp_prev2
        };

        self.prev_input = input;
        self.hp_prev2 = self.hp_prev1;
        self.hp_prev1 = hp;

        let val = input - hp;
        let result = Some(val);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.prev_input = 0.0;
        self.hp_prev1 = 0.0;
        self.hp_prev2 = 0.0;
        self.count = 0;
        self.last_value = None;
    }
    fn is_ready(&self) -> bool {
        self.count >= 1
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingDecycler {
    fn name() -> &'static str {
        "DECYCLER"
    }
    fn category() -> &'static str {
        "cycle"
    }
    fn description() -> &'static str {
        "Ehlers Decycler (trend extraction)"
    }
    fn warm_up_period(&self) -> usize {
        1
    }
}

// ============================================================================
// StreamingBandpass
// ============================================================================

/// Streaming Bandpass Filter (Ehlers).
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingBandpass {
    alpha: f64,
    beta: f64,
    /// input[i-1]
    input_prev1: f64,
    /// input[i-2]
    input_prev2: f64,
    /// output[i-1]
    out_prev1: f64,
    /// output[i-2]
    out_prev2: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingBandpass {
    pub fn new(period: usize, bandwidth: f64) -> Self {
        let p = period.max(2) as f64;
        let bw = bandwidth.clamp(0.01, 1.0);
        let beta = (2.0 * std::f64::consts::PI / p).cos();
        let gamma = (2.0 * std::f64::consts::PI * bw / p).cos();
        let delta = 1.0 / gamma;
        let alpha = delta - (delta * delta - 1.0).sqrt();
        Self {
            alpha,
            beta,
            input_prev1: 0.0,
            input_prev2: 0.0,
            out_prev1: 0.0,
            out_prev2: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl Default for StreamingBandpass {
    fn default() -> Self {
        Self::new(20, 0.3)
    }
}

impl StreamingIndicator for StreamingBandpass {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let val = if self.count <= 2 {
            0.0
        } else {
            0.5 * (1.0 - self.alpha) * (input - self.input_prev2)
                + self.beta * (1.0 + self.alpha) * self.out_prev1
                - self.alpha * self.out_prev2
        };
        self.input_prev2 = self.input_prev1;
        self.input_prev1 = input;
        self.out_prev2 = self.out_prev1;
        self.out_prev1 = val;

        let result = Some(val);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.input_prev1 = 0.0;
        self.input_prev2 = 0.0;
        self.out_prev1 = 0.0;
        self.out_prev2 = 0.0;
        self.count = 0;
        self.last_value = None;
    }
    fn is_ready(&self) -> bool {
        self.count >= 3
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingBandpass {
    fn name() -> &'static str {
        "BANDPASS"
    }
    fn category() -> &'static str {
        "cycle"
    }
    fn description() -> &'static str {
        "Ehlers Bandpass Filter"
    }
    fn warm_up_period(&self) -> usize {
        3
    }
}

// ============================================================================
// StreamingInstantaneousTrendline
// ============================================================================

/// Streaming Instantaneous Trendline / ITrend (Ehlers).
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingInstantaneousTrendline {
    alpha: f64,
    c0: f64,
    c1: f64,
    c2: f64,
    c3: f64,
    c4: f64,
    prev_input1: f64,
    prev_input2: f64,
    prev_out1: f64,
    prev_out2: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingInstantaneousTrendline {
    pub fn new(alpha: f64) -> Self {
        let a = alpha.clamp(0.001, 1.0);
        Self {
            alpha: a,
            c0: a - a * a / 4.0,
            c1: 0.5 * a * a,
            c2: a - 0.75 * a * a,
            c3: 2.0 * (1.0 - a),
            c4: (1.0 - a) * (1.0 - a),
            prev_input1: 0.0,
            prev_input2: 0.0,
            prev_out1: 0.0,
            prev_out2: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl Default for StreamingInstantaneousTrendline {
    fn default() -> Self {
        Self::new(0.07)
    }
}

impl StreamingIndicator for StreamingInstantaneousTrendline {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let val = match self.count {
            1 => input,
            2 => (input + self.prev_input1) / 2.0,
            _ => {
                self.c0 * input + self.c1 * self.prev_input1 - self.c2 * self.prev_input2
                    + self.c3 * self.prev_out1
                    - self.c4 * self.prev_out2
            }
        };
        self.prev_input2 = self.prev_input1;
        self.prev_input1 = input;
        self.prev_out2 = self.prev_out1;
        self.prev_out1 = val;

        let result = Some(val);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.prev_input1 = 0.0;
        self.prev_input2 = 0.0;
        self.prev_out1 = 0.0;
        self.prev_out2 = 0.0;
        self.count = 0;
        self.last_value = None;
    }
    fn is_ready(&self) -> bool {
        self.count >= 3
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingInstantaneousTrendline {
    fn name() -> &'static str {
        "INSTANTANEOUS_TRENDLINE"
    }
    fn category() -> &'static str {
        "cycle"
    }
    fn description() -> &'static str {
        "Ehlers Instantaneous Trendline (ITrend)"
    }
    fn warm_up_period(&self) -> usize {
        3
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_data(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| (i as f64 * 0.1).sin() * 10.0 + 50.0)
            .collect()
    }

    #[test]
    fn test_streaming_super_smoother_basic() {
        let mut ss = StreamingSuperSmoother::new(10);
        let data = test_data(100);
        let mut last = None;
        for &v in &data {
            last = ss.next(v);
        }
        assert!(last.is_some());
        assert!(ss.is_ready());
    }

    #[test]
    fn test_streaming_super_smoother_reset() {
        let mut ss = StreamingSuperSmoother::new(10);
        for i in 0..20 {
            ss.next(i as f64);
        }
        assert!(ss.is_ready());
        ss.reset();
        assert!(!ss.is_ready());
        assert_eq!(ss.count(), 0);
    }

    #[test]
    fn test_streaming_super_smoother_meta() {
        assert_eq!(StreamingSuperSmoother::name(), "SUPER_SMOOTHER");
        assert_eq!(StreamingSuperSmoother::category(), "cycle");
    }

    #[test]
    fn test_streaming_super_smoother_3pole_basic() {
        let mut ss = StreamingSuperSmoother3Pole::new(10);
        let data = test_data(100);
        let mut last = None;
        for &v in &data {
            last = ss.next(v);
        }
        assert!(last.is_some());
    }

    #[test]
    fn test_streaming_roofing_filter_basic() {
        let mut rf = StreamingRoofingFilter::new(48, 10);
        let data = test_data(200);
        let mut results = vec![];
        for &v in &data {
            if let Some(r) = rf.next(v) {
                results.push(r);
            }
        }
        assert!(!results.is_empty());
        let has_pos = results.iter().any(|&v| v > 0.0);
        let has_neg = results.iter().any(|&v| v < 0.0);
        assert!(has_pos && has_neg, "roofing filter should oscillate");
    }

    #[test]
    fn test_streaming_decycler_basic() {
        let mut dc = StreamingDecycler::new(20);
        let data = test_data(100);
        let mut last = None;
        for &v in &data {
            last = dc.next(v);
        }
        assert!(last.is_some());
        assert!(dc.is_ready());
    }

    #[test]
    fn test_streaming_bandpass_basic() {
        let mut bp = StreamingBandpass::new(20, 0.3);
        let data = test_data(100);
        let mut last = None;
        for &v in &data {
            last = bp.next(v);
        }
        assert!(last.is_some());
    }

    #[test]
    fn test_streaming_instantaneous_trendline_basic() {
        let mut it = StreamingInstantaneousTrendline::new(0.07);
        let data: Vec<f64> = (0..100).map(|i| i as f64 * 0.5 + 10.0).collect();
        let mut last = None;
        for &v in &data {
            last = it.next(v);
        }
        assert!(last.is_some());
        // Should track a linear trend closely
        let final_val = last.unwrap();
        let expected = 99.0 * 0.5 + 10.0;
        assert!((final_val - expected).abs() < 10.0);
    }

    #[test]
    fn test_streaming_instantaneous_trendline_reset() {
        let mut it = StreamingInstantaneousTrendline::new(0.07);
        for i in 0..50 {
            it.next(i as f64);
        }
        it.reset();
        assert_eq!(it.count(), 0);
        assert!(!it.is_ready());
    }
}
