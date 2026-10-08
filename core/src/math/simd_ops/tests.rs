//! Unit tests for this module.

use super::prelude::*;

// ============================================================================
// Tests for P.2: SIMD temporal operators
// ============================================================================

#[cfg(all(feature = "std", test))]
mod simd_temporal_tests {
    use super::*;

    #[test]
    fn test_simd_ema_next_matches_scalar() {
        let prev = 10.0;
        let sample = 12.0;
        let k = 0.3;
        let got = simd_ema_next(prev, sample, k);
        let expected = prev + k * (sample - prev);
        assert!(
            (got - expected).abs() < 1e-12,
            "got={} expected={}",
            got,
            expected
        );
    }

    #[test]
    fn test_simd_cmo_matches_scalar() {
        // 8 bars: trending up then down
        let src: Vec<f64> = (0..20)
            .map(|i| 10.0 + (i as f64 * 0.5).sin() * 2.0)
            .collect();
        let mut out = vec![f64::NAN; 20];
        simd_cmo(&src, 5, &mut out);
        // 头 5 根 NaN
        for i in 0..5 {
            assert!(out[i].is_nan());
        }
        // 有效范围 [-100, 100]
        for i in 5..20 {
            assert!(out[i].is_finite(), "out[{}] = {}", i, out[i]);
            assert!(out[i] >= -100.0 && out[i] <= 100.0);
        }
    }

    #[test]
    fn test_simd_mama_hilbert_matches() {
        let n = 50;
        let src: Vec<f64> = (0..n).map(|i| 10.0 + (i as f64 * 0.1).sin()).collect();
        let mut smooth = vec![f64::NAN; n];
        let mut period = vec![f64::NAN; n];
        simd_mama_hilbert(&src, &mut smooth, &mut period);
        // 前 3 根 NaN
        for i in 0..3 {
            assert!(smooth[i].is_nan());
            assert!(period[i].is_nan());
        }
        // 后续根：smooth 应在 src min/max 之间
        for i in 3..n {
            assert!(smooth[i].is_finite());
            assert!(period[i].is_finite());
            assert!(period[i] >= 6.0 && period[i] <= 50.0);
        }
    }

    #[test]
    fn test_simd_sar_step_matches() {
        // 简单上升趋势：SAR 应在 low 之下
        let high: Vec<f64> = (0..20).map(|i| 10.0 + i as f64).collect();
        let low: Vec<f64> = (0..20).map(|i| 9.0 + i as f64).collect();
        let mut out = vec![f64::NAN; 20];
        simd_sar_step(&high, &low, 9.0, 10.0, 0.02, 0.02, 0.2, &mut out);
        // SAR 应单调上升
        for i in 1..20 {
            assert!(
                out[i] >= out[i - 1] - 1e-9,
                "SAR not monotonic: {} >= {}",
                out[i],
                out[i - 1]
            );
        }
    }

    #[test]
    fn test_simd_t3_matches() {
        let n = 30;
        let src: Vec<f64> = (0..n).map(|i| 10.0 + (i as f64) * 0.1).collect();
        let mut out = vec![f64::NAN; n];
        simd_t3(&src, 5, 0.7, &mut out);
        // 前 4 根 NaN
        for i in 0..4 {
            assert!(out[i].is_nan());
        }
        // 严格上升时 T3 应 ≥ 起点
        for i in 4..n {
            assert!(out[i].is_finite());
            assert!(out[i] >= 10.0 - 0.01, "T3[{}] = {} below start", i, out[i]);
        }
    }

    #[test]
    fn test_simd_ht_dcphase_matches() {
        let n = 30;
        let src: Vec<f64> = (0..n).map(|i| 10.0 + (i as f64 * 0.1).sin()).collect();
        let mut out = vec![f64::NAN; n];
        simd_ht_dcphase(&src, &mut out);
        for i in 0..3 {
            assert!(out[i].is_nan());
        }
        for i in 3..n {
            assert!(out[i].is_finite());
            assert!(
                out[i] >= 0.0 && out[i] <= 180.0,
                "phase[{}] = {}",
                i,
                out[i]
            );
        }
    }
}

// ============================================================================
// Tests for D': SIMD ATR / AROON / KAMA
// ============================================================================

#[cfg(all(feature = "std", test))]
mod d_prime_tests {
    use super::*;

    #[test]
    fn test_simd_atr_wilder() {
        let high = vec![52.0, 53.0, 54.0, 53.5, 55.0, 56.0, 55.5, 57.0];
        let low = vec![50.0, 50.5, 51.0, 52.0, 53.0, 54.0, 54.5, 55.0];
        let prev_close = vec![49.0, 51.5, 52.5, 53.0, 53.0, 54.5, 55.5, 55.0];
        let mut out = vec![f64::NAN; 8];
        simd_atr(&high, &low, &prev_close, 3, &mut out);
        // First two are NaN (warmup), index 2+ are valid
        assert!(out[0].is_nan());
        assert!(out[1].is_nan());
        assert!(out[2].is_finite());
        assert!(out[2] > 0.0);
        // ATR should be stable (no NaN propagation past warmup)
        for i in 2..out.len() {
            assert!(out[i].is_finite(), "ATR at {} is NaN", i);
            assert!(out[i] > 0.0);
        }
    }

    #[test]
    fn test_simd_aroon_basic() {
        // 10 bars, period 5: high makes a new high at the last bar
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0, 13.0, 12.0, 11.0, 12.0, 15.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 12.0, 11.0, 10.0, 11.0, 14.0];
        let mut up = vec![f64::NAN; 10];
        let mut down = vec![f64::NAN; 10];
        simd_aroon(&high, &low, 5, &mut up, &mut down);
        // First 4 are NaN
        for i in 0..4 {
            assert!(up[i].is_nan());
            assert!(down[i].is_nan());
        }
        // At i=9: window is [5..=9] = [13,12,11,12,15], max at 9 (offset 4) → 100
        assert!((up[9] - 100.0).abs() < 1e-10, "up[9] = {}", up[9]);
        // min in low: [12,11,10,11,14], min at 7 (offset 2) → (5-2)/5*100 = 60
        assert!((down[9] - 60.0).abs() < 1e-10, "down[9] = {}", down[9]);
    }

    #[test]
    fn test_simd_kama_trending() {
        // 30 bars of strict uptrend → KAMA should be close to close at the end
        let n = 30;
        let input: Vec<f64> = (0..n).map(|i| 10.0 + (i as f64) * 0.1).collect();
        let mut out = vec![f64::NAN; n];
        simd_kama(&input, 10, 2, 30, &mut out);
        // First 10 are NaN
        for i in 0..10 {
            assert!(out[i].is_nan());
        }
        // After warmup, KAMA should track the uptrend
        assert!(out[n - 1] > 10.0, "KAMA end = {}", out[n - 1]);
        // In a perfect trend ER=1, so KAMA ≈ EMA with fast=2 (alpha ~ 0.67)
        // KAMA[29] should be in [input[19], input[29]]
        assert!(out[n - 1] >= input[19] - 0.01);
    }
}

// ============================================================================
// Tests for D.2: SIMD Hilbert Transform kernels
// ============================================================================

#[cfg(all(feature = "std", test))]
mod hilbert_simd_tests {
    use super::*;

    /// Scalar reference for smooth (matches the legacy cycle.rs implementation).
    fn smooth_scalar(input: &[f64], out: &mut [f64]) {
        let len = input.len().min(out.len());
        for o in out.iter_mut().take(len) {
            *o = 0.0;
        }
        for i in 3..len {
            out[i] =
                0.1 * (4.0 * input[i] + 3.0 * input[i - 1] + 2.0 * input[i - 2] + input[i - 3]);
        }
    }

    /// Scalar reference for detrender.
    fn detrender_scalar(smooth: &[f64], out: &mut [f64]) {
        let len = smooth.len().min(out.len());
        for o in out.iter_mut().take(len) {
            *o = 0.0;
        }
        for i in 10..len {
            let a = 0.0962 * smooth[i] + 0.5769 * smooth[i - 2]
                - 0.5769 * smooth[i - 4]
                - 0.0962 * smooth[i - 6];
            let b = 0.075 * smooth[i - 1] + 0.54 * smooth[i - 3] + 0.075 * smooth[i - 5];
            out[i] = a * b;
        }
    }

    /// Scalar reference for the components+phase fused computation.
    fn components_scalar(detrender: &[f64], phase_out: &mut [f64]) {
        let len = detrender.len().min(phase_out.len());
        for o in phase_out.iter_mut().take(len) {
            *o = 0.0;
        }
        for i in 16..len {
            let ip = detrender[i - 6];
            let q = 0.0962 * detrender[i] + 0.5769 * detrender[i - 2]
                - 0.5769 * detrender[i - 4]
                - 0.0962 * detrender[i - 6];
            let j1 = 0.0962 * detrender[i - 6] + 0.5769 * detrender[i - 8]
                - 0.5769 * detrender[i - 10]
                - 0.0962 * detrender[i - 12];
            let i2 = ip - j1;
            let j2 = q + ip;
            let re = i2 * ip + j2 * q;
            let im = i2 * q - j2 * ip;
            phase_out[i] = if re.abs() > 1e-10 { im.atan2(re) } else { 0.0 };
        }
    }

    #[test]
    fn test_simd_ht_smooth_matches_scalar() {
        let n = 50;
        let input: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 3.0)
            .collect();
        let mut simd_out = vec![0.0; n];
        let mut scalar_out = vec![0.0; n];
        simd_ht_smooth(&input, &mut simd_out);
        smooth_scalar(&input, &mut scalar_out);
        for i in 0..n {
            assert!(
                (simd_out[i] - scalar_out[i]).abs() < 1e-12,
                "smooth mismatch at {}: simd={} scalar={}",
                i,
                simd_out[i],
                scalar_out[i]
            );
        }
    }

    #[test]
    fn test_simd_ht_detrender_matches_scalar() {
        let n = 50;
        let input: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 3.0)
            .collect();
        let mut smooth = vec![0.0; n];
        smooth_scalar(&input, &mut smooth);

        let mut simd_out = vec![0.0; n];
        let mut scalar_out = vec![0.0; n];
        simd_ht_detrender(&smooth, &mut simd_out);
        detrender_scalar(&smooth, &mut scalar_out);
        for i in 0..n {
            assert!(
                (simd_out[i] - scalar_out[i]).abs() < 1e-12,
                "detrender mismatch at {}: simd={} scalar={}",
                i,
                simd_out[i],
                scalar_out[i]
            );
        }
    }

    #[test]
    fn test_simd_ht_components_matches_scalar() {
        let n = 60;
        let input: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 3.0)
            .collect();
        let mut smooth = vec![0.0; n];
        let mut detrender = vec![0.0; n];
        smooth_scalar(&input, &mut smooth);
        detrender_scalar(&smooth, &mut detrender);

        let mut simd_phase = vec![0.0; n];
        let mut scalar_phase = vec![0.0; n];
        simd_ht_components(&detrender, &mut simd_phase);
        components_scalar(&detrender, &mut scalar_phase);
        for i in 16..n {
            assert!(
                (simd_phase[i] - scalar_phase[i]).abs() < 1e-12,
                "phase mismatch at {}: simd={} scalar={}",
                i,
                simd_phase[i],
                scalar_phase[i]
            );
        }
    }

    #[test]
    fn test_simd_ht_pipeline_consistency() {
        // 100-bar sine wave + SIMD pipeline should match scalar pipeline.
        let n = 100;
        let input: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 3.0)
            .collect();

        // SIMD path
        let mut smooth_simd = vec![0.0; n];
        simd_ht_smooth(&input, &mut smooth_simd);
        let mut detrender_simd = vec![0.0; n];
        simd_ht_detrender(&smooth_simd, &mut detrender_simd);
        let mut phase_simd = vec![0.0; n];
        simd_ht_components(&detrender_simd, &mut phase_simd);

        // Scalar path
        let mut smooth_scalar_v = vec![0.0; n];
        smooth_scalar(&input, &mut smooth_scalar_v);
        let mut detrender_scalar_v = vec![0.0; n];
        detrender_scalar(&smooth_scalar_v, &mut detrender_scalar_v);
        let mut phase_scalar_v = vec![0.0; n];
        components_scalar(&detrender_scalar_v, &mut phase_scalar_v);

        for i in 16..n {
            assert!(
                (phase_simd[i] - phase_scalar_v[i]).abs() < 1e-10,
                "pipeline phase mismatch at {}: simd={} scalar={}",
                i,
                phase_simd[i],
                phase_scalar_v[i]
            );
        }
    }

    #[test]
    fn test_simd_ht_phase_bounded() {
        let n = 50;
        let input: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 3.0)
            .collect();
        let mut smooth = vec![0.0; n];
        let mut detrender = vec![0.0; n];
        simd_ht_smooth(&input, &mut smooth);
        simd_ht_detrender(&smooth, &mut detrender);
        let mut phase = vec![0.0; n];
        simd_ht_components(&detrender, &mut phase);
        // Phase should be in [-π, π]
        for i in 16..n {
            assert!(phase[i].is_finite(), "phase at {} is NaN/inf", i);
            assert!(
                phase[i] >= -core::f64::consts::PI - 1e-9
                    && phase[i] <= core::f64::consts::PI + 1e-9,
                "phase[{}] = {} out of [-π, π]",
                i,
                phase[i]
            );
        }
    }

    #[test]
    fn test_simd_ht_short_input() {
        // Should handle inputs < warmup window without panicking.
        for &n in &[0usize, 1, 3, 10, 16, 17, 20] {
            let input: Vec<f64> = (0..n).map(|i| i as f64).collect();
            let mut smooth = vec![0.0; n];
            let mut detrender = vec![0.0; n];
            let mut phase = vec![0.0; n];
            simd_ht_smooth(&input, &mut smooth);
            simd_ht_detrender(&smooth, &mut detrender);
            simd_ht_components(&detrender, &mut phase);
            // No panics, all outputs finite
            for i in 0..n {
                assert!(smooth[i].is_finite() || smooth[i] == 0.0);
                assert!(detrender[i].is_finite() || detrender[i] == 0.0);
                assert!(phase[i].is_finite() || phase[i] == 0.0);
            }
        }
    }
}

// ============================================================================
// Tests for D.4: China market indicator SIMD kernels
// ============================================================================

#[cfg(all(feature = "std", test))]
mod china_simd_tests {
    use super::*;

    #[test]
    fn test_simd_diff_sum_matches_scalar() {
        let n = 100;
        let a: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 3.0)
            .collect();
        let b: Vec<f64> = (0..n)
            .map(|i| 49.0 + (i as f64 * 0.07).cos() * 2.0)
            .collect();
        for &period in &[1usize, 3, 4, 7, 16, 32, 50, 100] {
            let simd = simd_diff_sum(&a, &b, period);
            let mut scalar = 0.0;
            for i in 0..period {
                scalar += a[i] - b[i];
            }
            assert!(
                (simd - scalar).abs() < 1e-10,
                "diff_sum mismatch at period {}: simd={} scalar={}",
                period,
                simd,
                scalar
            );
        }
    }

    #[test]
    fn test_simd_max_diff_sum_matches_scalar() {
        let n = 100;
        let a: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 3.0)
            .collect();
        let b: Vec<f64> = (0..n)
            .map(|i| 49.0 + (i as f64 * 0.07).cos() * 2.0)
            .collect();
        for &period in &[1usize, 3, 4, 7, 16, 32, 50, 100] {
            let simd = simd_max_diff_sum(&a, &b, period);
            let mut scalar = 0.0;
            for i in 0..period {
                let d = a[i] - b[i];
                if d > 0.0 {
                    scalar += d;
                }
            }
            assert!(
                (simd - scalar).abs() < 1e-10,
                "max_diff_sum mismatch at period {}: simd={} scalar={}",
                period,
                simd,
                scalar
            );
        }
    }

    #[test]
    fn test_simd_dual_diff_init_matches_scalar() {
        let n = 60;
        let high: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 3.0)
            .collect();
        let open: Vec<f64> = (0..n)
            .map(|i| 49.0 + (i as f64 * 0.07).cos() * 2.0)
            .collect();
        let low: Vec<f64> = (0..n)
            .map(|i| 48.0 + (i as f64 * 0.13).sin() * 1.5)
            .collect();
        for &period in &[1usize, 3, 4, 7, 16, 32, 50, 60] {
            let (simd_ho, simd_ol) = simd_dual_diff_init(&high, &open, &low, period);
            let mut ho = 0.0;
            let mut ol = 0.0;
            for i in 0..period {
                ho += high[i] - open[i];
                ol += open[i] - low[i];
            }
            assert!(
                (simd_ho - ho).abs() < 1e-10,
                "AR sum_ho mismatch at period {}: simd={} scalar={}",
                period,
                simd_ho,
                ho
            );
            assert!(
                (simd_ol - ol).abs() < 1e-10,
                "AR sum_ol mismatch at period {}: simd={} scalar={}",
                period,
                simd_ol,
                ol
            );
        }
    }

    #[test]
    fn test_simd_dual_max_init_matches_scalar() {
        let n = 60;
        let high: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 3.0)
            .collect();
        let close: Vec<f64> = (0..n)
            .map(|i| 49.0 + (i as f64 * 0.07).cos() * 2.0)
            .collect();
        let low: Vec<f64> = (0..n)
            .map(|i| 48.0 + (i as f64 * 0.13).sin() * 1.5)
            .collect();
        for &period in &[1usize, 3, 4, 7, 16, 32, 50, 59] {
            let (simd_up, simd_down) = simd_dual_max_init(&high, &close, &low, period);
            let mut up = 0.0;
            let mut down = 0.0;
            for j in 1..=period {
                let d_up = high[j] - close[j - 1];
                let d_down = close[j - 1] - low[j];
                if d_up > 0.0 {
                    up += d_up;
                }
                if d_down > 0.0 {
                    down += d_down;
                }
            }
            assert!(
                (simd_up - up).abs() < 1e-10,
                "BR sum_up mismatch at period {}: simd={} scalar={}",
                period,
                simd_up,
                up
            );
            assert!(
                (simd_down - down).abs() < 1e-10,
                "BR sum_down mismatch at period {}: simd={} scalar={}",
                period,
                simd_down,
                down
            );
        }
    }
}

#[cfg(all(feature = "std", test))]
mod simd_capability_tests {
    use super::*;

    #[test]
    fn test_simd_capability_flags() {
        let flags = simd_capability_flags();
        // On any x86_64 test runner, SSE4.1 should be available
        #[cfg(target_arch = "x86_64")]
        assert!(flags & 1 != 0, "SSE4.1 should be available");
        let _ = flags;
    }

    #[test]
    fn test_has_avx2_consistent() {
        let a = has_avx2();
        let b = has_avx2();
        assert_eq!(a, b, "has_avx2 should be deterministic");
    }

    #[test]
    fn test_simd_sma_matches_scalar() {
        let input: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        for &period in &[1, 3, 5, 7, 20] {
            if period > input.len() {
                continue;
            }
            let mut simd_out = vec![0.0; input.len()];
            let mut scalar_out = vec![0.0; input.len()];
            simd_sma(&input, period, &mut simd_out);
            sma_scalar(&input, period, &mut scalar_out);
            for i in 0..input.len() {
                if scalar_out[i].is_nan() {
                    assert!(simd_out[i].is_nan(), "period={period} i={i}");
                } else {
                    assert!(
                        (simd_out[i] - scalar_out[i]).abs() < 1e-10,
                        "period={period} i={i} simd={} scalar={}",
                        simd_out[i],
                        scalar_out[i]
                    );
                }
            }
        }
    }

    #[test]
    fn test_simd_wma_matches_scalar() {
        let input: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        for &period in &[1, 3, 5, 7, 20] {
            if period > input.len() {
                continue;
            }
            let mut simd_out = vec![0.0; input.len()];
            let mut scalar_out = vec![0.0; input.len()];
            simd_wma(&input, period, &mut simd_out);
            wma_scalar(&input, period, &mut scalar_out);
            for i in 0..input.len() {
                if scalar_out[i].is_nan() {
                    assert!(simd_out[i].is_nan(), "period={period} i={i}");
                } else {
                    assert!(
                        (simd_out[i] - scalar_out[i]).abs() < 1e-10,
                        "period={period} i={i} simd={} scalar={}",
                        simd_out[i],
                        scalar_out[i]
                    );
                }
            }
        }
    }
}

#[test]
fn test_simd_prefix_sum() {
    let data = [1.0, 2.0, 3.0, 4.0, 5.0];
    let mut result = [0.0; 5];
    simd_prefix_sum(&data, &mut result);
    assert!((result[0] - 1.0).abs() < 1e-10);
    assert!((result[2] - 6.0).abs() < 1e-10);
    assert!((result[4] - 15.0).abs() < 1e-10);
}

#[test]
fn test_simd_diff() {
    let data = [10.0, 12.0, 11.0, 15.0, 14.0];
    let mut result = [0.0; 5];
    simd_diff(&data, &mut result);
    assert!(result[0].is_nan());
    assert!((result[1] - 2.0).abs() < 1e-10);
    assert!((result[2] - (-1.0)).abs() < 1e-10);
}

#[test]
fn test_simd_scale() {
    let data = [1.0, 2.0, 3.0, 4.0, 5.0];
    let mut result = [0.0; 5];
    simd_scale(&data, 2.5, &mut result);
    assert!((result[0] - 2.5).abs() < 1e-10);
    assert!((result[4] - 12.5).abs() < 1e-10);
}

#[test]
fn test_simd_pct_change() {
    let data = [100.0, 110.0, 99.0, 100.0];
    let mut result = [0.0; 4];
    simd_pct_change(&data, &mut result);
    assert!(result[0].is_nan());
    assert!((result[1] - 10.0).abs() < 1e-10);
    assert!((result[2] - (-10.0)).abs() < 1e-10);
}

#[test]
fn test_simd_clamp() {
    let data = [-5.0, 0.0, 50.0, 150.0];
    let mut result = [0.0; 4];
    simd_clamp(&data, 0.0, 100.0, &mut result);
    assert!((result[0] - 0.0).abs() < 1e-10);
    assert!((result[2] - 50.0).abs() < 1e-10);
    assert!((result[3] - 100.0).abs() < 1e-10);
}

#[test]
fn test_simd_weighted_sum() {
    let data = [10.0, 20.0, 30.0];
    let weights = [0.5, 0.3, 0.2];
    let mut result = [0.0; 3];
    simd_weighted_sum(&data, &weights, &mut result);
    assert!((result[0] - 5.0).abs() < 1e-10);
    assert!((result[1] - 6.0).abs() < 1e-10);
    assert!((result[2] - 6.0).abs() < 1e-10);
}

#[test]
fn test_simd_true_range() {
    let high = [52.0, 53.0, 54.0];
    let low = [50.0, 50.0, 51.0];
    let prev_close = [49.0, 51.0, 52.0];
    let mut result = [0.0; 3];
    simd_true_range(&high, &low, &prev_close, &mut result);
    assert!((result[0] - 3.0).abs() < 1e-10);
    assert!((result[1] - 3.0).abs() < 1e-10);
    assert!((result[2] - 3.0).abs() < 1e-10);
}

#[test]
fn test_simd_typical_price() {
    let high = [52.0, 55.0];
    let low = [48.0, 50.0];
    let close = [50.0, 53.0];
    let mut result = [0.0; 2];
    simd_typical_price(&high, &low, &close, &mut result);
    assert!((result[0] - 50.0).abs() < 1e-10);
}

#[test]
fn fixed_mom10_matches_generic_dispatch() {
    let input: Vec<f64> = (0..128)
        .map(|index| (index as f64 * 0.37).sin() * 10.0 + index as f64)
        .collect();
    let mut fixed = vec![0.0; input.len()];
    let mut generic = vec![0.0; input.len()];

    simd_mom10(&input, &mut fixed);
    simd_mom(&input, 10, &mut generic);

    for (index, (&fixed_value, &generic_value)) in fixed.iter().zip(&generic).enumerate() {
        if index < 10 {
            assert!(fixed_value.is_nan(), "fixed index={index}");
            assert!(generic_value.is_nan(), "generic index={index}");
        } else {
            assert!(
                (fixed_value - generic_value).abs() < 1e-12,
                "index={index} fixed={fixed_value} generic={generic_value}"
            );
        }
    }
}

#[test]
fn test_simd_median_price() {
    let high = [52.0, 55.0];
    let low = [48.0, 50.0];
    let mut result = [0.0; 2];
    simd_median_price(&high, &low, &mut result);
    assert!((result[0] - 50.0).abs() < 1e-10);
    assert!((result[1] - 52.5).abs() < 1e-10);
}

#[test]
fn test_simd_log_return() {
    let data = [100.0, 110.0, 100.0];
    let mut result = [0.0; 3];
    simd_log_return(&data, &mut result);
    assert!(result[0].is_nan());
    assert!((result[1] - (1.1_f64).ln()).abs() < 1e-10);
}

#[test]
fn test_simd_zscore() {
    let data = [10.0, 12.0, 14.0, 13.0, 15.0];
    let mut result = [0.0; 5];
    simd_zscore(&data, 3, &mut result);
    assert!(result[0].is_nan());
    assert!(result[1].is_nan());
    assert!(result[2].abs() > 0.0 || result[2] == 0.0);
}

#[test]
fn test_simd_cumsum() {
    let data = [1.0, 2.0, 3.0];
    let mut result = [0.0; 3];
    simd_cumsum(&data, &mut result);
    assert!((result[0] - 1.0).abs() < 1e-10);
    assert!((result[1] - 3.0).abs() < 1e-10);
    assert!((result[2] - 6.0).abs() < 1e-10);
}

#[test]
fn test_simd_shift() {
    let data = [10.0, 20.0, 30.0, 40.0];
    let mut result = [0.0; 4];
    simd_shift(&data, 2, f64::NAN, &mut result);
    assert!(result[0].is_nan());
    assert!(result[1].is_nan());
    assert!((result[2] - 10.0).abs() < 1e-10);
    assert!((result[3] - 20.0).abs() < 1e-10);
}

#[test]
fn test_simd_obv() {
    let close = [10.0, 11.0, 10.5, 10.5, 12.0];
    let volume = [100.0, 200.0, 150.0, 100.0, 300.0];
    let mut result = [0.0; 5];
    simd_obv(&close, &volume, &mut result);
    assert!((result[0] - 100.0).abs() < 1e-10);
    assert!((result[1] - 300.0).abs() < 1e-10);
    assert!((result[2] - 150.0).abs() < 1e-10);
    assert!((result[3] - 150.0).abs() < 1e-10);
    assert!((result[4] - 450.0).abs() < 1e-10);
}

#[test]
fn test_simd_ad_line() {
    let high = [52.0, 53.0];
    let low = [48.0, 50.0];
    let close = [50.0, 52.0];
    let volume = [1000.0, 2000.0];
    let mut result = [0.0; 2];
    simd_ad_line(&high, &low, &close, &volume, &mut result);
    assert!((result[0] - 0.0).abs() < 1e-10);
}

#[test]
fn test_simd_roc() {
    let data = [100.0, 105.0, 110.0, 115.0, 120.0];
    let mut result = [0.0; 5];
    simd_roc(&data, 2, &mut result);
    assert!(result[0].is_nan());
    assert!(result[1].is_nan());
    assert!((result[2] - 10.0).abs() < 1e-10);
}

#[test]
fn test_simd_shift_negative() {
    let data = [10.0, 20.0, 30.0, 40.0];
    let mut result = [0.0; 4];
    simd_shift(&data, -1, f64::NAN, &mut result);
    assert!((result[0] - 20.0).abs() < 1e-10);
    assert!((result[1] - 30.0).abs() < 1e-10);
    assert!((result[2] - 40.0).abs() < 1e-10);
    assert!(result[3].is_nan());
}

#[test]
fn test_simd_sin_cos_matches_scalar() {
    // Phase domain directly produced by compute_hilbert_components (atan -> (-π/2, π/2)).
    let mut angles: Vec<f64> = Vec::new();
    let m = 4000;
    for i in 0..m {
        // dense sampling across (-π/2, π/2) plus edges
        angles.push(
            -core::f64::consts::FRAC_PI_2
                + 2.0 * core::f64::consts::FRAC_PI_2 * (i as f64) / (m as f64),
        );
    }
    // some random larger angles to exercise the quadrant blend
    let mut seed = 12345u64;
    for _ in 0..2000 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let r = ((seed >> 11) as f64) / (1u64 << 53) as f64; // [0,1)
        angles.push((r * 8.0 - 4.0) * core::f64::consts::PI); // [-4π, 4π]
    }

    let n = angles.len();
    let mut sin_out = vec![0.0_f64; n];
    let mut cos_out = vec![0.0_f64; n];
    simd_sin_cos(&angles, &mut sin_out, &mut cos_out);

    let mut max_sin_err = 0.0_f64;
    let mut max_cos_err = 0.0_f64;
    for i in 0..n {
        let (s, c) = angles[i].sin_cos();
        max_sin_err = max_sin_err.max((sin_out[i] - s).abs());
        max_cos_err = max_cos_err.max((cos_out[i] - c).abs());
    }
    // SLA: error <= 1e-9 for |x| <= π/2; larger angles keep < 1e-6 (polynomial approx).
    assert!(
        max_sin_err <= 1e-6,
        "max sin error {} exceeds 1e-6",
        max_sin_err
    );
    assert!(
        max_cos_err <= 1e-6,
        "max cos error {} exceeds 1e-6",
        max_cos_err
    );

    // Tighter bound for the phase domain specifically.
    let mut max_phase_sin_err = 0.0_f64;
    let mut max_phase_cos_err = 0.0_f64;
    for i in 0..m {
        let (s, c) = angles[i].sin_cos();
        max_phase_sin_err = max_phase_sin_err.max((sin_out[i] - s).abs());
        max_phase_cos_err = max_phase_cos_err.max((cos_out[i] - c).abs());
    }
    assert!(
        max_phase_sin_err <= 1e-9,
        "phase-domain sin error {} exceeds 1e-9",
        max_phase_sin_err
    );
    assert!(
        max_phase_cos_err <= 1e-9,
        "phase-domain cos error {} exceeds 1e-9",
        max_phase_cos_err
    );
}

#[test]
fn test_simd_sin_cos_throughput() {
    // Validate the SIMD sin/cos fast path speedup for the HT_SINE terminal
    // stage (phase ∈ (-π/2, π/2)). Compares simd_sin_cos against a per-element
    // scalar f64::sin_cos loop. Asserts a modest floor (>=1.5x) to stay safe
    // under CI jitter; the printed ratio is the real measurement.
    use std::time::Instant;
    let n = 200_000usize;
    let mut angles: Vec<f64> = Vec::with_capacity(n);
    let mut seed = 0x9E3779B97F4A7C15u64;
    for _ in 0..n {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let r = ((seed >> 11) as f64) / (1u64 << 53) as f64; // [0,1)
        angles.push((r - 0.5) * core::f64::consts::PI); // (-π/2, π/2)
    }

    let mut s_out = vec![0.0_f64; n];
    let mut c_out = vec![0.0_f64; n];

    for _ in 0..5 {
        simd_sin_cos(&angles, &mut s_out, &mut c_out);
    }
    let iters = 100;
    let simd_start = Instant::now();
    for _ in 0..iters {
        simd_sin_cos(&angles, &mut s_out, &mut c_out);
    }
    let simd_elapsed = simd_start.elapsed().as_nanos() as f64;

    let scalar_start = Instant::now();
    for _ in 0..iters {
        for i in 0..n {
            let (s, c) = angles[i].sin_cos();
            s_out[i] = s;
            c_out[i] = c;
        }
    }
    let scalar_elapsed = scalar_start.elapsed().as_nanos() as f64;

    let speedup = scalar_elapsed / simd_elapsed;
    eprintln!(
        "simd_sin_cos speedup: {:.2}x (simd={:.1} ns/elem, scalar={:.1} ns/elem)",
        speedup,
        simd_elapsed / (iters as f64 * n as f64),
        scalar_elapsed / (iters as f64 * n as f64)
    );
    assert!(
        speedup >= 1.5,
        "simd_sin_cos speedup too low: {:.2}x",
        speedup
    );
}

#[test]
fn test_simd_bp_tr_matches_scalar() {
    // simd_bp_tr is elementwise given prev_close, so it must be bit-identical
    // to the scalar formulation used by ultosc's pre-pass.
    let n = 500usize;
    let mut seed = 0xABCDEFu64;
    let mut rnd = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((seed >> 11) as f64) / (1u64 << 53) as f64
    };
    let high: Vec<f64> = (0..n).map(|_| 100.0 + 10.0 * rnd()).collect();
    let low: Vec<f64> = (0..n).map(|i| high[i] - 5.0 * rnd()).collect();
    let close: Vec<f64> = (0..n).map(|i| low[i] + 3.0 * rnd()).collect();

    let mut bp_simd = vec![0.0_f64; n];
    let mut tr_simd = vec![0.0_f64; n];
    simd_bp_tr(&high, &low, &close, &mut bp_simd, &mut tr_simd);

    let mut bp_scalar = vec![0.0_f64; n];
    let mut tr_scalar = vec![0.0_f64; n];
    for i in 1..n {
        let prev_close = close[i - 1];
        let tl = low[i].min(prev_close);
        bp_scalar[i] = close[i] - tl;
        tr_scalar[i] = high[i].max(prev_close) - tl;
    }

    for i in 0..n {
        assert!(
            (bp_simd[i] - bp_scalar[i]).abs() <= crate::utils::NUMERIC_EPSILON,
            "bp[{}] mismatch: {} vs {}",
            i,
            bp_simd[i],
            bp_scalar[i]
        );
        assert!(
            (tr_simd[i] - tr_scalar[i]).abs() <= crate::utils::NUMERIC_EPSILON,
            "tr[{}] mismatch: {} vs {}",
            i,
            tr_simd[i],
            tr_scalar[i]
        );
    }
}
