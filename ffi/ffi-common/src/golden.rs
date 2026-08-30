//! Cross-language golden reference vectors for AlphaTA indicators.
//!
//! This module is the **single source of truth for golden test values**, shared
//! by every FFI binding's test suite. Each [`GoldenVector`] records an input
//! series, a period, and the analytically-expected last output value, so a
//! binding can assert its implementation reproduces the reference without
//! depending on another implementation.
//!
//! The last-element convention is intentionally used (rather than the full
//! series) so the check is robust to warm-up / length conventions across
//! languages: the final value of a moving average is independent of how many
//! leading elements a given binding drops.

/// A golden test case: the last output value of `f(input, period)` must equal
/// `expected_last` within `epsilon`.
#[derive(Debug, Clone, Copy)]
pub struct GoldenVector {
    pub name: &'static str,
    pub input: &'static [f64],
    pub period: usize,
    pub expected_last: f64,
}

/// Assert two floats are within `eps` of each other.
pub fn assert_close(actual: f64, expected: f64, eps: f64) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= eps,
        "golden mismatch: actual={actual}, expected={expected}, |Δ|={diff} > eps={eps}"
    );
}

/// Simple Moving Average golden vectors.
/// `expected_last` = mean of the final `period` elements (analytic).
pub const SMA_GOLDEN: &[GoldenVector] = &[
    GoldenVector {
        name: "rising_1_10_p3",
        input: &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0],
        period: 3,
        expected_last: 9.0, // mean(8,9,10)
    },
    GoldenVector {
        name: "rising_1_5_p3",
        input: &[1.0, 2.0, 3.0, 4.0, 5.0],
        period: 3,
        expected_last: 4.0, // mean(3,4,5)
    },
];

/// Exponential Moving Average golden vectors (alpha = 2/(period+1) = 0.5).
/// Seeded with the SMA of the first `period` values (TA-Lib / Wilder
/// convention, matching `alpha_ta_core::math::moving_avg::ema`).
pub const EMA_GOLDEN: &[GoldenVector] = &[
    GoldenVector {
        name: "rising_1_10_p3",
        input: &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0],
        period: 3,
        expected_last: 9.0,
    },
    GoldenVector {
        name: "rising_1_5_p3",
        input: &[1.0, 2.0, 3.0, 4.0, 5.0],
        period: 3,
        expected_last: 4.0,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_sma_data_is_analytic() {
        for g in SMA_GOLDEN {
            let n = g.input.len();
            let window = &g.input[n - g.period..n];
            let mean = window.iter().sum::<f64>() / g.period as f64;
            assert_close(mean, g.expected_last, 1e-12);
        }
    }

    #[test]
    fn golden_ema_data_matches_recurrence() {
        for g in EMA_GOLDEN {
            let alpha = 2.0 / (g.period as f64 + 1.0);
            // Seed with the SMA of the first `period` values (TA-Lib convention).
            let seed: f64 = g.input[..g.period].iter().sum::<f64>() / g.period as f64;
            let mut ema = seed;
            for &x in &g.input[g.period..] {
                ema = alpha * x + (1.0 - alpha) * ema;
            }
            assert_close(ema, g.expected_last, 1e-9);
        }
    }
}
