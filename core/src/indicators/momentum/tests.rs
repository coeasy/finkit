//! Unit tests for this module.

use super::prelude::*;
// The reference ratio `apo_ppo_impl` folds internally, and the prelude does
// not re-export it: naming it keeps this file from being the only reason a
// whole bucket would be pulled into scope.
use super::apo_ppo_impl::ppo_ratio;

use approx::assert_relative_eq;

fn assert_array_matches_slice(a: &Array1<f64>, b: &[f64]) {
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b.iter()) {
        if x.is_nan() {
            assert!(y.is_nan());
        } else {
            assert_relative_eq!(*x, *y, epsilon = 1e-15);
        }
    }
}

#[test]
fn test_rsi() {
    let input = vec![
        44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 45.5, 45.5, 45.5, 46.0, 45.75, 46.25, 45.5, 45.25,
        46.0, 46.25, 47.0, 47.0, 47.25, 48.25,
    ];
    let result = rsi(&input, 14).unwrap();
    assert!(result[14] > 0.0 && result[14] <= 100.0);
}

#[test]
fn test_rsi_into_matches_rsi() {
    let input = vec![
        44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 45.5, 45.5, 45.5, 46.0, 45.75, 46.25, 45.5, 45.25,
        46.0, 46.25, 47.0, 47.0, 47.25, 48.25,
    ];
    let expected = rsi(&input, 14).unwrap();
    let mut output = vec![0.0; input.len()];
    rsi_into(&input, 14, &mut output).unwrap();
    assert_array_matches_slice(&expected, &output);
}

#[test]
fn test_stoch() {
    let high = vec![10.0, 12.0, 14.0, 16.0, 18.0, 17.0, 16.0, 15.0];
    let low = vec![8.0, 10.0, 12.0, 14.0, 16.0, 15.0, 14.0, 13.0];
    let close = vec![9.0, 11.0, 13.0, 15.0, 17.0, 16.0, 15.0, 14.0];
    let result = stoch(&high, &low, &close, 5, 1, 3).unwrap();
    // TA-Lib lookback = (k_period-1)+(k_slow-1)+(d_period-1) = 6，故首有效索引为 6
    assert!(!result.k[6].is_nan());
    assert!(!result.d[6].is_nan());
}

#[test]
fn test_stoch_into_matches_stoch() {
    let high = vec![10.0, 12.0, 14.0, 16.0, 18.0, 17.0, 16.0, 15.0];
    let low = vec![8.0, 10.0, 12.0, 14.0, 16.0, 15.0, 14.0, 13.0];
    let close = vec![9.0, 11.0, 13.0, 15.0, 17.0, 16.0, 15.0, 14.0];
    let expected = stoch(&high, &low, &close, 5, 1, 3).unwrap();
    let mut k_out = vec![0.0; close.len()];
    let mut d_out = vec![0.0; close.len()];
    stoch_into(&high, &low, &close, 5, 1, 3, &mut k_out, &mut d_out).unwrap();
    assert_array_matches_slice(&expected.k, &k_out);
    assert_array_matches_slice(&expected.d, &d_out);
}

#[test]
fn test_macd() {
    let input: Vec<f64> = (1..=35).map(|x| x as f64).collect();
    let result = macd(&input, 12, 26, 9).unwrap();
    // TA-Lib lookback is (slow - 1) + (signal - 1) = 33.
    // All public MACD outputs share that warm-up boundary.
    assert!(result.macd[32].is_nan());
    assert!(result.signal[32].is_nan());
    assert!(result.hist[32].is_nan());
    assert!(!result.macd[33].is_nan());
    assert!(!result.signal[33].is_nan());
    assert!(!result.hist[33].is_nan());
}

#[test]
fn test_mom() {
    let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let result = mom(&input, 2).unwrap();
    assert!(result[0].is_nan());
    assert!(result[1].is_nan());
    assert_relative_eq!(result[2], 2.0, epsilon = 1e-10);
    assert_relative_eq!(result[3], 2.0, epsilon = 1e-10);
    assert_relative_eq!(result[4], 2.0, epsilon = 1e-10);
}

#[test]
fn test_roc() {
    let input = vec![10.0, 12.0, 15.0];
    let result = roc(&input, 1).unwrap();
    assert!(result[0].is_nan());
    assert_relative_eq!(result[1], 20.0, epsilon = 1e-10);
}

#[test]
fn test_willr() {
    let high = vec![10.0, 12.0, 14.0, 16.0, 18.0];
    let low = vec![8.0, 10.0, 12.0, 14.0, 16.0];
    let close = vec![9.0, 11.0, 13.0, 15.0, 17.0];
    let result = willr(&high, &low, &close, 3).unwrap();
    assert!(!result[2].is_nan());
}

#[test]
fn test_willr14_fixed_kernel_matches_generic() {
    for length in [14, 15, 27, 28, 29, 96] {
        let high: Vec<f64> = (0..length)
            .map(|index| 100.0 + (index % 17) as f64 + (index / 17) as f64 * 0.25)
            .collect();
        let low: Vec<f64> = high
            .iter()
            .enumerate()
            .map(|(index, value)| value - 2.0 - (index % 5) as f64 * 0.1)
            .collect();
        let close: Vec<f64> = high
            .iter()
            .zip(&low)
            .enumerate()
            .map(|(index, (&high, &low))| low + (high - low) * (0.2 + (index % 7) as f64 * 0.1))
            .collect();
        let expected = willr(&high, &low, &close, 14).unwrap();
        let mut actual = vec![0.0; close.len()];
        willr14_into(&high, &low, &close, &mut actual).unwrap();
        assert_array_matches_slice(&expected, &actual);
    }
}

#[test]
fn test_cmo() {
    let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0];
    let result = cmo(&input, 3).unwrap();
    assert!(result[0].is_nan());
    assert!(result[3] > 0.0);
}

#[test]
fn test_trix() {
    let input = vec![
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
    ];
    let result = trix(&input, 5).unwrap();
    assert!(result[0].is_nan());
}

#[test]
fn test_apo() {
    let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let result = apo(&input, 2, 4).unwrap();
    assert!(result[0].is_nan());
}

#[test]
fn test_apo_fused_matches_sma_diff() {
    // Fused single-pass APO must be bit-identical to sma(fast) - sma(slow).
    let n = 10_000;
    let input: Vec<f64> = (0..n)
        .map(|i| 100.0 + (i as f64 * 0.37).sin() * 2.0 + i as f64 * 0.01)
        .collect();
    for (fast, slow) in [(2usize, 5usize), (12, 26), (1, 3), (30, 60)] {
        let fused = apo(&input, fast, slow).unwrap();
        let f = crate::math::moving_avg::sma(&input, fast).unwrap();
        let s = crate::math::moving_avg::sma(&input, slow).unwrap();
        for i in 0..n {
            let expect = if f[i].is_nan() || s[i].is_nan() {
                f64::NAN
            } else {
                f[i] - s[i]
            };
            if expect.is_nan() {
                assert!(fused[i].is_nan(), "mismatch NaN at {i}");
            } else {
                assert_eq!(fused[i], expect, "mismatch at {i} fast={fast} slow={slow}");
            }
        }
    }
}

#[test]
fn test_bop() {
    let open = vec![10.0, 11.0, 12.0];
    let high = vec![12.0, 13.0, 14.0];
    let low = vec![9.0, 10.0, 11.0];
    let close = vec![11.0, 12.0, 13.0];
    let result = bop(&open, &high, &low, &close).unwrap();
    assert_relative_eq!(result[0], 0.333333, epsilon = 1e-4);
}

#[test]
fn test_elder_ray_basic() {
    let high = vec![10.0, 12.0, 14.0, 16.0, 18.0, 17.0, 16.0, 15.0, 14.0, 13.0];
    let low = vec![8.0, 10.0, 12.0, 14.0, 16.0, 15.0, 14.0, 13.0, 12.0, 11.0];
    let close = vec![9.0, 11.0, 13.0, 15.0, 17.0, 16.0, 15.0, 14.0, 13.0, 12.0];
    let volume = vec![
        1000.0, 1200.0, 1400.0, 1600.0, 1800.0, 1500.0, 1300.0, 1100.0, 900.0, 800.0,
    ];
    let result = elder_ray(&high, &low, &close, &volume, 5).unwrap();
    assert!(!result.force_index[1].is_nan());
    assert!(!result.bull_power[4].is_nan());
    assert!(!result.bear_power[4].is_nan());
}

#[test]
fn test_elder_ray_force_index_calculation() {
    let high = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
    let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
    let close = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
    let volume = vec![100.0, 200.0, 300.0, 400.0, 500.0, 600.0];
    let result = elder_ray(&high, &low, &close, &volume, 3).unwrap();
    assert_relative_eq!(result.force_index[1], (10.5 - 9.5) * 200.0, epsilon = 1e-10);
    assert_relative_eq!(
        result.force_index[2],
        (11.5 - 10.5) * 300.0,
        epsilon = 1e-10
    );
    assert_relative_eq!(
        result.force_index[5],
        (14.5 - 13.5) * 600.0,
        epsilon = 1e-10
    );
}

#[test]
fn test_elder_ray_bull_bear_power() {
    let high = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let low = vec![9.0, 10.0, 11.0, 12.0, 13.0];
    let close = vec![9.5, 10.5, 11.5, 12.5, 13.5];
    let volume = vec![100.0, 200.0, 300.0, 400.0, 500.0];
    let period = 3;
    let result = elder_ray(&high, &low, &close, &volume, period).unwrap();
    for i in 0..(period - 1) {
        assert!(result.bull_power[i].is_nan());
        assert!(result.bear_power[i].is_nan());
    }
    for i in (period - 1)..result.bull_power.len() {
        assert!(!result.bull_power[i].is_nan());
        assert!(!result.bear_power[i].is_nan());
        assert!(result.bull_power[i] > result.bear_power[i]);
    }
}

#[test]
fn test_elder_ray_invalid_input_length() {
    let high = vec![10.0, 11.0, 12.0];
    let low = vec![9.0, 10.0];
    let close = vec![9.5, 10.5, 11.5];
    let volume = vec![100.0, 200.0, 300.0];
    let result = elder_ray(&high, &low, &close, &volume, 3);
    assert!(result.is_err());
}

#[test]
fn test_elder_ray_insufficient_data() {
    let high = vec![10.0, 11.0];
    let low = vec![9.0, 10.0];
    let close = vec![9.5, 10.5];
    let volume = vec![100.0, 200.0];
    let result = elder_ray(&high, &low, &close, &volume, 5);
    assert!(result.is_err());
}

#[test]
fn test_elder_ray_zero_volume() {
    let high = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let low = vec![9.0, 10.0, 11.0, 12.0, 13.0];
    let close = vec![9.5, 10.5, 11.5, 12.5, 13.5];
    let volume = vec![0.0, 0.0, 0.0, 0.0, 0.0];
    let result = elder_ray(&high, &low, &close, &volume, 3).unwrap();
    assert_relative_eq!(result.force_index[1], 0.0, epsilon = 1e-10);
    assert_relative_eq!(result.force_index[2], 0.0, epsilon = 1e-10);
    assert_relative_eq!(result.force_index[4], 0.0, epsilon = 1e-10);
}

#[test]
fn test_elder_ray_negative_force_index() {
    let high = vec![10.0, 11.0, 10.0, 9.0, 8.0];
    let low = vec![9.0, 10.0, 9.0, 8.0, 7.0];
    let close = vec![9.5, 10.5, 9.5, 8.5, 7.5];
    let volume = vec![100.0, 200.0, 300.0, 400.0, 500.0];
    let result = elder_ray(&high, &low, &close, &volume, 3).unwrap();
    assert!(result.force_index[2] < 0.0);
    assert!(result.force_index[3] < 0.0);
    assert!(result.force_index[4] < 0.0);
}

#[test]
fn test_adxr() {
    let high: Vec<f64> = (0..60).map(|i| 100.0 + (i as f64) * 0.5).collect();
    let low: Vec<f64> = (0..60).map(|i| 98.0 + (i as f64) * 0.5).collect();
    let close: Vec<f64> = (0..60).map(|i| 99.0 + (i as f64) * 0.5).collect();
    let result = adxr(&high, &low, &close, 14).unwrap();
    assert_eq!(result.len(), 60);
    assert!(result.iter().any(|&x| !x.is_nan()));
}

#[test]
fn test_adx_into_matches_adx_without_intermediate_family_buffers() {
    let high: Vec<f64> = (0..80)
        .map(|i| 100.0 + (i as f64 * 0.17).sin() * 2.0 + i as f64 * 0.2)
        .collect();
    let low: Vec<f64> = high.iter().map(|value| value - 1.5).collect();
    let close: Vec<f64> = high.iter().map(|value| value - 0.7).collect();
    let expected = adx(&high, &low, &close, 14).unwrap();
    let mut actual = vec![0.0; close.len()];
    adx_into(&high, &low, &close, 14, &mut actual).unwrap();
    for (expected, actual) in expected.iter().zip(actual.iter()) {
        assert!(
            (expected.is_nan() && actual.is_nan()) || (expected - actual).abs() < 1e-12,
            "ADX mismatch: {expected} vs {actual}"
        );
    }
}

#[test]
fn test_aroonosc() {
    let high = vec![10.0, 12.0, 14.0, 13.0, 15.0, 11.0, 16.0, 17.0, 14.0, 13.0];
    let low = vec![8.0, 10.0, 12.0, 11.0, 13.0, 9.0, 14.0, 15.0, 12.0, 11.0];
    let result = aroonosc(&high, &low, 5).unwrap();
    assert_eq!(result.len(), 10);
    assert!(result.iter().skip(4).any(|&x| !x.is_nan()));
}

#[test]
fn test_macdext() {
    let input: Vec<f64> = (1..=30).map(|x| x as f64).collect();
    let result = macdext(&input, 12, MaType::Ema, 26, MaType::Ema, 9, MaType::Ema).unwrap();
    assert_eq!(result.macd.len(), 30);
}

#[test]
fn test_macdfix() {
    let input: Vec<f64> = (1..=40).map(|x| x as f64).collect();
    let result = macdfix(&input).unwrap();
    assert_eq!(result.macd.len(), 40);
}

#[test]
fn test_ppo() {
    let input: Vec<f64> = (1..=30).map(|x| x as f64).collect();
    let result = ppo(&input, 12, 26).unwrap();
    assert_eq!(result.len(), 30);
    assert!(result.iter().skip(25).any(|&x| !x.is_nan()));
}

#[test]
fn test_rocp() {
    let input = vec![10.0, 11.0, 12.0, 13.0, 14.0];
    let result = rocp(&input, 1).unwrap();
    assert_relative_eq!(result[1], 0.1, epsilon = 1e-10);
    assert_relative_eq!(result[2], 1.0 / 11.0, epsilon = 1e-10);
}

#[test]
fn test_rocr() {
    let input = vec![10.0, 12.0, 15.0];
    let result = rocr(&input, 1).unwrap();
    assert_relative_eq!(result[1], 1.2, epsilon = 1e-10);
    assert_relative_eq!(result[2], 15.0 / 12.0, epsilon = 1e-10);
}

#[test]
fn test_rocr100() {
    let input = vec![10.0, 12.0, 15.0];
    let result = rocr100(&input, 1).unwrap();
    assert_relative_eq!(result[1], 120.0, epsilon = 1e-10);
}

#[test]
fn test_stochf() {
    let high = vec![10.0, 12.0, 14.0, 13.0, 15.0, 11.0, 16.0, 17.0, 14.0, 13.0];
    let low = vec![8.0, 10.0, 12.0, 11.0, 13.0, 9.0, 14.0, 15.0, 12.0, 11.0];
    let close = vec![9.0, 11.0, 13.0, 12.0, 14.0, 10.0, 15.0, 16.0, 13.0, 12.0];
    let result = stochf(&high, &low, &close, 5, 3).unwrap();
    assert_eq!(result.k.len(), 10);
}

#[test]
fn test_stochrsi() {
    let input: Vec<f64> = (0..50)
        .map(|i| 100.0 + (i as f64 * 0.1).sin() * 10.0)
        .collect();
    let result = stochrsi(&input, 14, 14, 3, 3).unwrap();
    assert_eq!(result.k.len(), 50);
}

#[test]
fn test_ultosc() {
    let high: Vec<f64> = (0..40).map(|i| 100.0 + (i as f64) * 0.5).collect();
    let low: Vec<f64> = (0..40).map(|i| 98.0 + (i as f64) * 0.5).collect();
    let close: Vec<f64> = (0..40).map(|i| 99.0 + (i as f64) * 0.5).collect();
    let result = ultosc(&high, &low, &close, 7, 14, 28).unwrap();
    assert_eq!(result.len(), 40);
    assert!(result.iter().skip(28).any(|&x| !x.is_nan()));
}

/// Independent O(n * period) evaluation of the Ultimate Oscillator
/// definition, sharing no code with the kernel. Both ring tiers are pinned
/// against this because a mask slip reads the wrong bar and the golden
/// fixtures only exercise the default periods.
fn ultosc_reference(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    p1: usize,
    p2: usize,
    p3: usize,
) -> Vec<f64> {
    let n = high.len();
    let max_period = p1.max(p2).max(p3);
    let mut bp = vec![0.0; n];
    let mut tr = vec![0.0; n];
    for i in 1..n {
        let true_low = low[i].min(close[i - 1]);
        bp[i] = close[i] - true_low;
        tr[i] = high[i].max(close[i - 1]) - true_low;
    }
    let mut out = vec![f64::NAN; n];
    for i in max_period..n {
        let mut acc = 0.0;
        for (weight, period) in [(4.0, p1), (2.0, p2), (1.0, p3)] {
            let start = i + 1 - period;
            let b: f64 = bp[start..=i].iter().sum();
            let t: f64 = tr[start..=i].iter().sum();
            if t > 0.0 {
                acc += weight * (b / t);
            }
        }
        out[i] = 100.0 * acc / 7.0;
    }
    out
}

#[test]
fn test_ultosc_matches_definition_on_both_ring_tiers() {
    // 7/14/28 and 2/3/5 need a 32-slot ring (stack tier); 9/18/36 needs 64
    // (heap tier). Both tiers must agree with the definition.
    let n = 400;
    let high: Vec<f64> = (0..n)
        .map(|i| 100.0 + (i as f64 * 0.37).sin() * 3.0 + i as f64 * 0.01)
        .collect();
    let low: Vec<f64> = high.iter().map(|h| h - 1.5 - (h * 0.01).fract()).collect();
    let close: Vec<f64> = high
        .iter()
        .zip(low.iter())
        .map(|(h, l)| l + (h - l) * 0.6)
        .collect();

    for (p1, p2, p3) in [(7, 14, 28), (9, 18, 36), (2, 3, 5)] {
        let expected = ultosc_reference(&high, &low, &close, p1, p2, p3);
        let actual = ultosc(&high, &low, &close, p1, p2, p3).unwrap();
        assert_eq!(actual.len(), n);
        for i in 0..n {
            let (e, a) = (expected[i], actual[i]);
            if e.is_nan() {
                assert!(
                    a.is_nan(),
                    "({p1},{p2},{p3}) index {i}: expected NaN, got {a}"
                );
            } else {
                assert!(
                    (e - a).abs() < 1e-9,
                    "({p1},{p2},{p3}) index {i}: definition {e} vs kernel {a}"
                );
            }
        }
    }
}

#[test]
fn test_ultosc_empty_window_is_zero_not_accumulator_residue() {
    // A window whose true ranges are all exactly zero makes every divide
    // 0/0. The totals are maintained by add-then-subtract, so a window that
    // has just emptied holds rounding residue of either sign rather than
    // 0.0 -- which is why the guard has to be exact instead of a fixed band.
    // The band form divided one residue by another and returned values far
    // outside the oscillator's 0..100 range (TA-Lib's issues #244 / #253).
    // Move first, so the accumulators are genuinely non-zero when the flat
    // stretch arrives and the residue is not trivially zero.
    let n = 80;
    let mut high = vec![0.0; n];
    let mut low = vec![0.0; n];
    let mut close = vec![0.0; n];
    let mut price = 100.0;
    for i in 0..n {
        if i < 30 {
            price += if i % 2 == 0 { 0.7 } else { -0.4 };
            high[i] = price + 1.1;
            low[i] = price - 1.3;
            close[i] = price + 0.2;
        } else {
            // H == L == previous close, so both terms are exactly zero.
            high[i] = price;
            low[i] = price;
            close[i] = price;
        }
    }

    let result = ultosc(&high, &low, &close, 7, 14, 28).unwrap();
    for (i, &v) in result.iter().enumerate().skip(28) {
        // A weighted average of three `bp / tr` ratios, each in 0..1.
        assert!(
            (0.0..=100.0).contains(&v),
            "index {i} left the oscillator range: {v}"
        );
        if i >= 30 + 27 {
            // The longest window now spans only flat bars.
            assert_eq!(v, 0.0, "index {i} should be an exact zero, got {v}");
        }
    }
}

#[test]
fn test_macd_into_matches_macd() {
    let input: Vec<f64> = (1..=50).map(|x| x as f64).collect();
    let expected = macd(&input, 12, 26, 9).unwrap();
    let mut macd_out = vec![0.0; input.len()];
    let mut signal_out = vec![0.0; input.len()];
    let mut hist_out = vec![0.0; input.len()];
    macd_into(
        &input,
        12,
        26,
        9,
        &mut macd_out,
        &mut signal_out,
        &mut hist_out,
    )
    .unwrap();
    for i in 0..input.len() {
        let em = expected.macd[i];
        let am = macd_out[i];
        if em.is_nan() {
            assert!(am.is_nan());
        } else {
            assert!((em - am).abs() < 1e-9, "macd mismatch at {i}: {em} vs {am}");
        }
        let es = expected.signal[i];
        let as_ = signal_out[i];
        if es.is_nan() {
            assert!(as_.is_nan());
        } else {
            assert!(
                (es - as_).abs() < 1e-9,
                "signal mismatch at {i}: {es} vs {as_}"
            );
        }
    }
}

// ===========================================================================
// Zero-copy `_into` variants (B4 / TASK-315)
//
// Each `_into` function computes the indicator directly into a caller-owned
// `&mut [f64]` buffer (zero per-call allocation from the caller's perspective)
// by delegating to the canonical allocating batch implementation and copying
// the result. This mirrors the existing `bbands_into`/`dema_into` convention
// and guarantees numerical parity with the batch API.
// ===========================================================================

macro_rules! impl_into_delegate {
($name:ident, $batch:path, ($($arg:ident: $t:ty),* $(,)?)) => {
    pub fn $name($($arg: $t,)* output: &mut [f64]) -> Result<()> {
        let result = $batch($($arg),*)?;
        if result.len() != output.len() {
            return Err(TaError::InvalidParameter {
                name: "output".to_string(),
                constraint: "must have the same length as the input series".to_string(),
            });
        }
        output.copy_from_slice(result.as_slice().unwrap());
        Ok(())
    }
};
}

impl_into_delegate!(apo_into, apo, (input: &[f64], fast_period: usize, slow_period: usize));
impl_into_delegate!(bop_into, bop, (open: &[f64], high: &[f64], low: &[f64], close: &[f64]));
impl_into_delegate!(cmo_into, cmo, (input: &[f64], period: usize));
impl_into_delegate!(dx_into, dx, (high: &[f64], low: &[f64], close: &[f64], period: usize));
impl_into_delegate!(minus_di_into, minus_di, (high: &[f64], low: &[f64], close: &[f64], period: usize));
impl_into_delegate!(minus_dm_into, minus_dm, (high: &[f64], low: &[f64]));
impl_into_delegate!(plus_di_into, plus_di, (high: &[f64], low: &[f64], close: &[f64], period: usize));
impl_into_delegate!(plus_dm_into, plus_dm, (high: &[f64], low: &[f64]));
impl_into_delegate!(aroonosc_into, aroonosc, (high: &[f64], low: &[f64], period: usize));
impl_into_delegate!(ppo_into, ppo, (input: &[f64], fast_period: usize, slow_period: usize));
impl_into_delegate!(rocp_into, rocp, (input: &[f64], period: usize));
impl_into_delegate!(rocr_into, rocr, (input: &[f64], period: usize));
impl_into_delegate!(rocr100_into, rocr100, (input: &[f64], period: usize));

#[cfg(test)]
mod into_tests {
    use super::*;

    fn check_eq(a: &[f64], b: &[f64]) {
        assert_eq!(a.len(), b.len(), "length mismatch");
        for i in 0..a.len() {
            if a[i].is_nan() {
                assert!(b[i].is_nan(), "nan mismatch at {i}");
            } else {
                assert!(
                    (a[i] - b[i]).abs() < 1e-12,
                    "value mismatch at {i}: {} vs {}",
                    a[i],
                    b[i]
                );
            }
        }
    }

    #[test]
    fn test_macd_fast_into_parity() {
        let input: Vec<f64> = (0..10_000)
            .map(|i| 100.0 + i as f64 * 0.01 + (i as f64 * 0.37).sin())
            .collect();
        let mut expected_macd = vec![0.0; input.len()];
        let mut expected_signal = vec![0.0; input.len()];
        let mut expected_hist = vec![0.0; input.len()];
        macd_into(
            &input,
            12,
            26,
            9,
            &mut expected_macd,
            &mut expected_signal,
            &mut expected_hist,
        )
        .unwrap();

        let mut actual_macd = vec![0.0; input.len()];
        let mut actual_signal = vec![0.0; input.len()];
        let mut actual_hist = vec![0.0; input.len()];
        macd_fast_into(
            &input,
            12,
            26,
            9,
            &mut actual_macd,
            &mut actual_signal,
            &mut actual_hist,
        )
        .unwrap();

        for (expected, actual) in [
            (&expected_macd, &actual_macd),
            (&expected_signal, &actual_signal),
            (&expected_hist, &actual_hist),
        ] {
            for (&left, &right) in expected.iter().zip(actual) {
                if left.is_nan() {
                    assert!(right.is_nan());
                } else {
                    assert!((left - right).abs() < 1e-10);
                }
            }
        }
    }

    #[test]
    fn test_cmo_fast_into_parity() {
        let input: Vec<f64> = (0..10_000)
            .map(|i| 100.0 + i as f64 * 0.01 + (i as f64 * 0.37).sin())
            .collect();
        let expected = cmo(&input, 14).unwrap();
        let mut actual = vec![0.0; input.len()];
        cmo_fast_into(&input, 14, &mut actual).unwrap();
        check_eq(expected.as_slice().unwrap(), &actual);
    }

    #[test]
    fn test_momentum_into_parity() {
        let input = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0,
        ];
        let high = vec![
            2.0, 3.0, 4.0, 5.0, 6.0, 5.0, 4.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0,
        ];
        let low = vec![
            0.0, 1.0, 2.0, 3.0, 4.0, 3.0, 2.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,
        ];
        let open = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0,
        ];
        let close = input.clone();
        let n = input.len();

        let e = apo(&input, 3, 6).unwrap();
        let mut o = vec![0.0; n];
        apo_into(&input, 3, 6, &mut o).unwrap();
        check_eq(e.as_slice().unwrap(), &o);
        let e = bop(&open, &high, &low, &close).unwrap();
        let mut o = vec![0.0; n];
        bop_into(&open, &high, &low, &close, &mut o).unwrap();
        check_eq(e.as_slice().unwrap(), &o);
        let e = cmo(&input, 5).unwrap();
        let mut o = vec![0.0; n];
        cmo_into(&input, 5, &mut o).unwrap();
        check_eq(e.as_slice().unwrap(), &o);
        let e = dx(&high, &low, &close, 5).unwrap();
        let mut o = vec![0.0; n];
        dx_into(&high, &low, &close, 5, &mut o).unwrap();
        check_eq(e.as_slice().unwrap(), &o);
        let e = minus_di(&high, &low, &close, 5).unwrap();
        let mut o = vec![0.0; n];
        minus_di_into(&high, &low, &close, 5, &mut o).unwrap();
        check_eq(e.as_slice().unwrap(), &o);
        let e = plus_di(&high, &low, &close, 5).unwrap();
        let mut o = vec![0.0; n];
        plus_di_into(&high, &low, &close, 5, &mut o).unwrap();
        check_eq(e.as_slice().unwrap(), &o);
        let e = minus_dm(&high, &low).unwrap();
        let mut o = vec![0.0; n];
        minus_dm_into(&high, &low, &mut o).unwrap();
        check_eq(e.as_slice().unwrap(), &o);
        let e = plus_dm(&high, &low).unwrap();
        let mut o = vec![0.0; n];
        plus_dm_into(&high, &low, &mut o).unwrap();
        check_eq(e.as_slice().unwrap(), &o);
        let e = trix(&input, 5).unwrap();
        let mut o = vec![0.0; n];
        trix_into(&input, 5, &mut o).unwrap();
        check_eq(e.as_slice().unwrap(), &o);
        let e = adxr(&high, &low, &close, 5).unwrap();
        let mut o = vec![0.0; n];
        adxr_into(&high, &low, &close, 5, &mut o).unwrap();
        check_eq(e.as_slice().unwrap(), &o);
        let e = aroonosc(&high, &low, 5).unwrap();
        let mut o = vec![0.0; n];
        aroonosc_into(&high, &low, 5, &mut o).unwrap();
        check_eq(e.as_slice().unwrap(), &o);
        let e = ppo(&input, 3, 6).unwrap();
        let mut o = vec![0.0; n];
        ppo_into(&input, 3, 6, &mut o).unwrap();
        check_eq(e.as_slice().unwrap(), &o);
        let e = rocp(&input, 5).unwrap();
        let mut o = vec![0.0; n];
        rocp_into(&input, 5, &mut o).unwrap();
        check_eq(e.as_slice().unwrap(), &o);
        let e = rocr(&input, 5).unwrap();
        let mut o = vec![0.0; n];
        rocr_into(&input, 5, &mut o).unwrap();
        check_eq(e.as_slice().unwrap(), &o);
        let e = rocr100(&input, 5).unwrap();
        let mut o = vec![0.0; n];
        rocr100_into(&input, 5, &mut o).unwrap();
        check_eq(e.as_slice().unwrap(), &o);
    }
}

/// The fused PPO kernels must be **bit-identical** to the `ma()` composition
/// they replaced. PPO feeds golden TA-Lib parity for both `matype=0` (SMA)
/// and `matype=1` (EMA), so "close enough" is not an acceptable bar: a
/// reassociated recurrence would drift by ULPs and only show up as a
/// flaky golden diff on a different dataset.
#[test]
fn ppo_fused_kernels_are_bit_identical_to_the_ma_composition() {
    let input: Vec<f64> = (0..400)
        .map(|i| {
            let t = i as f64;
            100.0 + 7.0 * (t * 0.37).sin() + 0.5 * (t * 1.13).cos()
        })
        .collect();

    let reference = |fast: usize, slow: usize, ma_type: MaType| -> Vec<f64> {
        let fast_ma = crate::indicators::overlap::ma(&input, fast, ma_type).unwrap();
        let slow_ma = crate::indicators::overlap::ma(&input, slow, ma_type).unwrap();
        let mut expected = vec![f64::NAN; input.len()];
        for i in slow - 1..input.len() {
            if fast_ma[i].is_finite() && slow_ma[i].is_finite() {
                expected[i] = ppo_ratio(fast_ma[i], slow_ma[i]);
            }
        }
        expected
    };

    for (fast, slow) in [(1usize, 2usize), (3, 6), (12, 26), (5, 40)] {
        for ma_type in [MaType::Sma, MaType::Ema] {
            let expected = reference(fast, slow, ma_type);
            let actual = ppo_with_ma_type(&input, fast, slow, ma_type).unwrap();
            // `ma(Sma)`/`ma(Ema)` now route to the fused kernels, so compare
            // against an explicitly unfused reference built from the same
            // selectors to make sure the shortcut really is the same math.
            assert_eq!(actual.len(), expected.len());
            for (i, (&got, &want)) in actual.iter().zip(expected.iter()).enumerate() {
                assert_eq!(
                    got.to_bits(),
                    want.to_bits(),
                    "PPO({fast},{slow},{ma_type:?}) diverged at {i}: {got} vs {want}"
                );
            }
        }
    }
}

/// A leading NaN run is a warm-up prefix, not bad data — both fused kernels
/// must slide past it exactly like `sma`/`ema` do, leaving the prefix NaN.
#[test]
fn ppo_fused_kernels_skip_a_leading_warmup_run() {
    let mut input = vec![f64::NAN; 9];
    input.extend((0..120).map(|i| 50.0 + 3.0 * (i as f64 * 0.21).sin()));
    for ma_type in [MaType::Sma, MaType::Ema] {
        let fused = ppo_with_ma_type(&input, 5, 12, ma_type).unwrap();
        let fused = fused.as_slice().unwrap();
        // start = 9, slow = 12 -> first valid index is 9 + 12 - 1 = 20.
        assert!(fused[..20].iter().all(|v| v.is_nan()));
        assert!(fused[20..].iter().all(|v| v.is_finite()));
    }
}

/// A non-finite value *after* the series has started is a hard error — the
/// old `ma()`-based path rejected it, and silently turning that into a
/// warm-up slide would change PPO's public contract.
#[test]
fn ppo_fused_kernels_reject_non_finite_input() {
    let mut input: Vec<f64> = (0..60).map(|i| 10.0 + i as f64).collect();
    input[40] = f64::NAN;
    for ma_type in [MaType::Sma, MaType::Ema] {
        assert!(ppo_with_ma_type(&input, 5, 12, ma_type).is_err());
    }
}

/// The near-zero denominator guard has to survive the fusion: a flat zero
/// series makes every `slow` average exactly `0`, and `0/0` is not `NaN`
/// here — it collapses to `0.0`.
#[test]
fn ppo_fused_kernels_collapse_a_zero_denominator() {
    let input = vec![0.0f64; 80];
    for ma_type in [MaType::Sma, MaType::Ema] {
        let out = ppo_with_ma_type(&input, 3, 8, ma_type).unwrap();
        let out = out.as_slice().unwrap();
        assert!(out[..7].iter().all(|v| v.is_nan()));
        assert!(out[7..].iter().all(|&v| v == 0.0));
    }
}
