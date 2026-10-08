//! Unit tests for the legacy formula catalogue.

use super::prelude::*;

// ---------------------------------------------------------------------------
// Tests for TA-Lib compatible Math Transform & Math Operator functions
// ---------------------------------------------------------------------------

#[cfg(test)]
mod talib_compat_tests {
    use super::*;
    use crate::formula::engine::FormulaEngine;

    fn make_ctx(len: usize) -> FormulaContext {
        let open = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.1).collect());
        let high = Array1::from_vec((0..len).map(|i| 11.0 + i as f64 * 0.2).collect());
        let low = Array1::from_vec((0..len).map(|i| 9.0 + i as f64 * 0.1).collect());
        let close = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.15).collect());
        let volume = Array1::from_vec((0..len).map(|i| 1000.0 + i as f64 * 10.0).collect());
        FormulaContext::new(open, high, low, close, volume, None)
    }

    fn build_args(arrays: Vec<Array1<f64>>) -> Vec<Array1<f64>> {
        arrays
    }

    // ---------------- Hyperbolic: SINH / COSH / TANH ----------------

    #[test]
    fn test_fn_sinh() {
        let ctx = make_ctx(3);
        let args = build_args(vec![Array1::from_vec(vec![0.0, 1.0, -1.0])]);
        let r = fn_sinh(&ctx, &args).unwrap();
        assert!((r[0] - 0.0).abs() < 1e-12);
        assert!((r[1] - 1.0_f64.sinh()).abs() < 1e-12);
        assert!((r[2] - (-1.0_f64).sinh()).abs() < 1e-12);
    }

    #[test]
    fn test_fn_cosh() {
        let ctx = make_ctx(3);
        let args = build_args(vec![Array1::from_vec(vec![0.0, 1.0, 2.0])]);
        let r = fn_cosh(&ctx, &args).unwrap();
        assert!((r[0] - 1.0).abs() < 1e-12);
        assert!((r[1] - 1.0_f64.cosh()).abs() < 1e-12);
        assert!((r[2] - 2.0_f64.cosh()).abs() < 1e-12);
    }

    #[test]
    fn test_fn_tanh() {
        let ctx = make_ctx(3);
        let args = build_args(vec![Array1::from_vec(vec![0.0, 1.0, -1.0])]);
        let r = fn_tanh(&ctx, &args).unwrap();
        assert!((r[0] - 0.0).abs() < 1e-12);
        assert!((r[1] - 1.0_f64.tanh()).abs() < 1e-12);
        assert!((r[2] - (-1.0_f64).tanh()).abs() < 1e-12);
    }

    // ---------------- Arithmetic two-input: ADD / SUB / MULT / DIV ----------------

    #[test]
    fn test_fn_add() {
        let ctx = make_ctx(3);
        let args = build_args(vec![
            Array1::from_vec(vec![1.0, 2.0, 3.0]),
            Array1::from_vec(vec![10.0, 20.0, 30.0]),
        ]);
        let r = fn_add(&ctx, &args).unwrap();
        assert_eq!(r.to_vec(), vec![11.0, 22.0, 33.0]);
    }

    #[test]
    fn test_fn_sub() {
        let ctx = make_ctx(3);
        let args = build_args(vec![
            Array1::from_vec(vec![10.0, 20.0, 30.0]),
            Array1::from_vec(vec![1.0, 5.0, 100.0]),
        ]);
        let r = fn_sub(&ctx, &args).unwrap();
        assert_eq!(r.to_vec(), vec![9.0, 15.0, -70.0]);
    }

    #[test]
    fn test_fn_mult() {
        let ctx = make_ctx(3);
        let args = build_args(vec![
            Array1::from_vec(vec![2.0, 3.0, 4.0]),
            Array1::from_vec(vec![5.0, 6.0, 7.0]),
        ]);
        let r = fn_mult(&ctx, &args).unwrap();
        assert_eq!(r.to_vec(), vec![10.0, 18.0, 28.0]);
    }

    #[test]
    fn test_fn_div() {
        let ctx = make_ctx(4);
        let args = build_args(vec![
            Array1::from_vec(vec![10.0, 20.0, 30.0, 5.0]),
            Array1::from_vec(vec![2.0, 4.0, 5.0, 0.0]),
        ]);
        let r = fn_div(&ctx, &args).unwrap();
        assert!((r[0] - 5.0).abs() < 1e-12);
        assert!((r[1] - 5.0).abs() < 1e-12);
        assert!((r[2] - 6.0).abs() < 1e-12);
        assert!(r[3].is_nan());
    }

    // ---------------- MINUS (period difference) ----------------

    #[test]
    fn test_fn_minus_basic() {
        let ctx = make_ctx(5);
        let data: Array1<f64> = Array1::from_vec(vec![1.0, 2.0, 4.0, 7.0, 11.0]);
        let n: Array1<f64> = Array1::from_vec(vec![2.0]);
        let r = fn_minus(&ctx, &[data, n]).unwrap();
        assert!(r[0].is_nan() && r[1].is_nan());
        assert!((r[2] - 3.0).abs() < 1e-12);
        assert!((r[3] - 5.0).abs() < 1e-12);
        assert!((r[4] - 7.0).abs() < 1e-12);
        // result length should match ctx.data_len
        assert_eq!(r.len(), ctx.data_len);
    }

    #[test]
    fn test_fn_minus_period_one() {
        let ctx = make_ctx(4);
        let data = Array1::from_vec(vec![1.0, 3.0, 6.0, 10.0]);
        let n = Array1::from_vec(vec![1.0]);
        let r = fn_minus(&ctx, &[data, n]).unwrap();
        assert!(r[0].is_nan());
        assert!((r[1] - 2.0).abs() < 1e-12);
        assert!((r[2] - 3.0).abs() < 1e-12);
        assert!((r[3] - 4.0).abs() < 1e-12);
    }

    // ---------------- MAXINDEX / MININDEX ----------------

    #[test]
    fn test_fn_maxindex() {
        let ctx = make_ctx(5);
        let data = Array1::from_vec(vec![3.0, 1.0, 4.0, 1.0, 5.0]);
        let n = Array1::from_vec(vec![3.0]);
        let r = fn_maxindex(&ctx, &[data, n]).unwrap();
        assert!(r[0].is_nan() && r[1].is_nan());
        // [3,1,4] max=4 at offset 2
        assert!((r[2] - 2.0).abs() < 1e-12);
        // [1,4,1] max=4 at offset 1
        assert!((r[3] - 1.0).abs() < 1e-12);
        // [4,1,5] max=5 at offset 2
        assert!((r[4] - 2.0).abs() < 1e-12);
        assert_eq!(r.len(), ctx.data_len);
    }

    #[test]
    fn test_fn_minindex() {
        let ctx = make_ctx(5);
        let data = Array1::from_vec(vec![3.0, 1.0, 4.0, 1.0, 5.0]);
        let n = Array1::from_vec(vec![3.0]);
        let r = fn_minindex(&ctx, &[data, n]).unwrap();
        assert!(r[0].is_nan() && r[1].is_nan());
        // [3,1,4] min=1 at offset 1
        assert!((r[2] - 1.0).abs() < 1e-12);
        // [1,4,1] min=1 at offset 0
        assert!((r[3] - 0.0).abs() < 1e-12);
        // [4,1,5] min=1 at offset 1
        assert!((r[4] - 1.0).abs() < 1e-12);
        assert_eq!(r.len(), ctx.data_len);
    }

    // ---------------- get_builtin_functions registration ----------------

    #[test]
    fn test_get_builtin_functions_contains_new_funcs() {
        let funcs = get_builtin_functions();
        for name in &[
            "SINH", "COSH", "TANH", "ADD", "SUB", "MULT", "DIV", "MINUS", "MAXINDEX", "MININDEX",
        ] {
            assert!(
                funcs.contains_key(*name),
                "missing function in registry: {}",
                name
            );
        }
    }

    // ---------------- Integration tests via FormulaEngine ----------------

    #[test]
    fn test_engine_sqrt_close() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval("SQRT(CLOSE)", &mut ctx).unwrap();
        for i in 0..5 {
            let expected = (10.0 + i as f64 * 0.15).sqrt();
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_engine_max_element_wise() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval("MAX(CLOSE, 20)", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            assert!((result[i] - close_val.max(20.0)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_engine_sum_vol_window() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(20);
        let result = engine.eval("SUM(VOL, 10)", &mut ctx).unwrap();
        // 前 9 个应为 NaN（窗口不足）
        for i in 0..9 {
            assert!(result[i].is_nan(), "expected NaN at index {}", i);
        }
        // 验证第 9 个及之后
        for i in 9..20 {
            let window_start = i + 1 - 10;
            let mut expected = 0.0_f64;
            for k in window_start..=i {
                expected += 1000.0 + k as f64 * 10.0;
            }
            assert!(
                (result[i] - expected).abs() < 1e-9,
                "mismatch at {}: {} vs {}",
                i,
                result[i],
                expected
            );
        }
    }

    #[test]
    fn test_engine_min_element_wise() {
        // MIN(CLOSE, 11) 在公式引擎中是 element-wise 版本（min(close, 11)）
        // 实际窗口版 MIN 见 indicators::math_operators::min
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval("MIN(CLOSE, 11)", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            assert!((result[i] - close_val.min(11.0)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_engine_sinh_cosh_tanh() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(3);
        let r_sinh = engine.eval("SINH(CLOSE)", &mut ctx).unwrap();
        let r_cosh = engine.eval("COSH(CLOSE)", &mut ctx).unwrap();
        let r_tanh = engine.eval("TANH(CLOSE)", &mut ctx).unwrap();
        for i in 0..3 {
            let c = 10.0 + i as f64 * 0.15;
            assert!((r_sinh[i] - c.sinh()).abs() < 1e-9);
            assert!((r_cosh[i] - c.cosh()).abs() < 1e-9);
            assert!((r_tanh[i] - c.tanh()).abs() < 1e-9);
        }
    }

    #[test]
    fn test_engine_add_sub_mult_div() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let r_add = engine.eval("ADD(CLOSE, OPEN)", &mut ctx).unwrap();
        let r_sub = engine.eval("SUB(CLOSE, OPEN)", &mut ctx).unwrap();
        let r_mult = engine.eval("MULT(CLOSE, 2)", &mut ctx).unwrap();
        let r_div = engine.eval("DIV(CLOSE, 2)", &mut ctx).unwrap();
        for i in 0..5 {
            let c = 10.0 + i as f64 * 0.15;
            let o = 10.0 + i as f64 * 0.1;
            assert!((r_add[i] - (c + o)).abs() < 1e-10);
            assert!((r_sub[i] - (c - o)).abs() < 1e-10);
            assert!((r_mult[i] - (c * 2.0)).abs() < 1e-10);
            assert!((r_div[i] - (c / 2.0)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_engine_minus() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval("MINUS(CLOSE, 2)", &mut ctx).unwrap();
        // 前 2 个为 NaN
        for i in 0..2 {
            assert!(result[i].is_nan());
        }
        for i in 2..5 {
            let cur = 10.0 + i as f64 * 0.15;
            let prev = 10.0 + (i - 2) as f64 * 0.15;
            let expected = cur - prev;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_engine_maxindex_minindex() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let r_max = engine.eval("MAXINDEX(CLOSE, 3)", &mut ctx).unwrap();
        let r_min = engine.eval("MININDEX(CLOSE, 3)", &mut ctx).unwrap();
        // 前 2 个为 NaN
        for i in 0..2 {
            assert!(r_max[i].is_nan());
            assert!(r_min[i].is_nan());
        }
        // close 是单调递增的，max 永远在最后 (offset 2)，min 永远在起点 (offset 0)
        for i in 2..5 {
            assert!((r_max[i] - 2.0).abs() < 1e-10);
            assert!((r_min[i] - 0.0).abs() < 1e-10);
        }
    }
    #[test]
    fn ts_to_date_parts_handles_epoch_boundaries() {
        assert_eq!(ts_to_date_parts(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(ts_to_date_parts(-1), (1969, 12, 31, 23, 59, 59));
        assert_eq!(ts_to_date_parts(-86_400), (1969, 12, 31, 0, 0, 0));
        assert_eq!(ts_to_date_parts(1_704_067_200), (2024, 1, 1, 0, 0, 0));
    }

    #[test]
    fn test_hhvbars_llvbars_keep_values_without_window_allocations() {
        let input = Array1::from_vec(vec![3.0, 1.0, 2.0, 2.0, 4.0]);
        let mut ctx = FormulaContext::new(
            input.clone(),
            input.clone(),
            input.clone(),
            input.clone(),
            input.clone(),
            None,
        );
        let mut engine = FormulaEngine::new();

        let hhv = engine.eval("HHVBARS(CLOSE, 3)", &mut ctx).unwrap();
        let llv = engine.eval("LLVBARS(CLOSE, 3)", &mut ctx).unwrap();

        assert_eq!(hhv.to_vec(), vec![0.0, 1.0, 2.0, 1.0, 0.0]);
        assert_eq!(llv.to_vec(), vec![0.0, 0.0, 1.0, 2.0, 2.0]);
    }
}

#[cfg(test)]
mod pr14_semantic_contract_tests {
    use super::*;
    use ndarray::array;

    fn context(close: Array1<f64>, volume: Array1<f64>) -> FormulaContext {
        let len = close.len();
        let open = close.clone();
        let high = close.mapv(|v| v + 1.0);
        let low = close.mapv(|v| v - 1.0);
        assert_eq!(volume.len(), len);
        FormulaContext::new(open, high, low, close, volume, None)
    }

    fn scalar(len: usize, value: f64) -> Array1<f64> {
        Array1::from_elem(len, value)
    }

    /// Differential guard for the sliding-violation LONGCROSS rewrite: the
    /// counter version must match a direct O(len·n) rescan on every bar,
    /// including the `i < n` cold zone and NaN bars.
    #[test]
    fn longcross_matches_naive_rescan() {
        let a: Vec<f64> = (0..60)
            .map(|i| {
                let t = i as f64;
                (t * 0.9).sin() * 3.0 + t * 0.05
            })
            .collect();
        let b: Vec<f64> = (0..60)
            .map(|i| {
                let t = i as f64;
                (t * 0.5).cos() * 1.5
            })
            .collect();
        let mut with_nan = a.clone();
        with_nan[10] = f64::NAN;
        with_nan[40] = f64::NAN;
        for (a_series, name) in [(&a, "finite"), (&with_nan, "with-nan")] {
            for n in [0usize, 1, 3, 10, 59] {
                let args = vec![
                    Array1::from(a_series.clone()),
                    Array1::from(b.clone()),
                    Array1::from(vec![n as f64]),
                ];
                let got = fn_longcross(&context(Array1::from(vec![0.0]), Array1::ones(1)), &args)
                    .unwrap();
                // Naive reference: the original double loop.
                let len = a.len();
                let mut expected = Array1::zeros(len);
                for i in 1..len {
                    if a_series[i] > b[i] {
                        let mut below_for_n = i >= n;
                        let mut j = i.saturating_sub(n);
                        while below_for_n && j < i {
                            if a_series[j] > b[j] {
                                below_for_n = false;
                            }
                            j += 1;
                        }
                        if below_for_n && a_series[i - 1] <= b[i - 1] {
                            expected[i] = 1.0;
                        }
                    }
                }
                assert_eq!(
                    got.as_slice().unwrap(),
                    expected.as_slice().unwrap(),
                    "LONGCROSS {name} n={n}"
                );
            }
        }
    }

    /// `BACKSET` fills *backwards* from each trigger, so bar `j` is on iff a
    /// trigger fires in `[j, j + n - 1]`. The countdown rewrite must match the
    /// original per-trigger fill exactly, including overlapping triggers and
    /// the last `n - 1` bars (where the window runs past the end).
    #[test]
    fn backset_matches_naive_fill() {
        let cond: Vec<f64> = (0..80)
            .map(|i| {
                if [3usize, 4, 20, 21, 22, 50, 79].contains(&i) {
                    1.0
                } else {
                    0.0
                }
            })
            .collect();
        let len = cond.len();
        let ctx = context(Array1::from(cond.clone()), Array1::ones(len));
        for n in [1usize, 2, 3, 5, 17, 40, 80, 200] {
            let args = vec![Array1::from(cond.clone()), Array1::from(vec![n as f64])];
            let got = fn_backset(&ctx, &args).unwrap();
            let mut expected = Array1::zeros(len);
            for i in 0..len {
                if cond[i] > 0.0 {
                    for j in (i + 1).saturating_sub(n)..=i {
                        expected[j] = 1.0;
                    }
                }
            }
            assert_eq!(
                got.as_slice().unwrap(),
                expected.as_slice().unwrap(),
                "BACKSET n={n}"
            );
        }
    }

    /// MODE used to pick its winner with `max_by_key` over a `HashMap`
    /// iteration, which is order-dependent (Rust randomises the seed per
    /// process) — tied counts could return different values run to run. The
    /// winner is now the earliest occurrence in the window, checked against a
    /// direct recount.
    #[test]
    fn mode_ties_break_to_earliest_occurrence() {
        // Window 3 over [5,7,5,7,...]: every window holds a 2-vs-1 or 1-vs-2
        // majority, and the first full window is a 1-1 tie broken by position.
        let input = vec![5.0, 7.0, 9.0, 5.0, 7.0, 7.0, 5.0, 5.0];
        let ctx = context(Array1::from(input.clone()), Array1::ones(input.len()));
        let args = vec![Array1::from(input.clone()), Array1::from(vec![3.0])];
        let got = fn_mode(&ctx, &args).unwrap();
        let mut expected = vec![f64::NAN; input.len()];
        for i in 0..input.len() {
            if i + 1 < 3 {
                continue;
            }
            let start = i + 1 - 3;
            let mut best: Option<(usize, usize, f64)> = None;
            for j in start..=i {
                let value = input[j];
                if value.is_nan() {
                    continue;
                }
                let count = (start..=i).filter(|&k| input[k] == value).count();
                let wins = match best {
                    None => true,
                    Some((best_count, best_first, _)) => {
                        count > best_count || (count == best_count && j < best_first)
                    }
                };
                if wins {
                    best = Some((count, j, value));
                }
            }
            if let Some((_, _, value)) = best {
                expected[i] = value;
            }
        }
        // `assert_eq!` on slices would fail on the leading NaNs (NaN != NaN).
        let same = got
            .iter()
            .zip(expected.iter())
            .all(|(g, e)| g == e || (g.is_nan() && e.is_nan()));
        assert!(
            same,
            "MODE got {:?} expected {:?}",
            got.as_slice().unwrap(),
            expected
        );
        // [5, 7, 9]: all appear once, so the earliest (5.0) wins rather than
        // whatever order the map happened to yield.
        assert_eq!(got[2], 5.0);
        // [5, 7, 7]: 7.0 is the actual majority, not just the earliest.
        assert_eq!(got[5], 7.0);
    }

    /// `LAST(cond, A, B)` is a fixed-width all-true test over `[i-A, i-B]`; the
    /// sliding violation counter must match the rescan on every bar, including
    /// the `i < A` cold zone, `A == B`, and NaN bars (which violate).
    #[test]
    fn last_matches_naive_rescan() {
        let cond: Vec<f64> = (0..60)
            .map(|i| if i % 7 == 3 && i % 5 == 1 { 0.0 } else { 1.0 })
            .collect();
        let mut with_nan = cond.clone();
        with_nan[9] = f64::NAN;
        with_nan[33] = f64::NAN;
        let ctx = context(Array1::from(cond.clone()), Array1::ones(cond.len()));
        for (series, name) in [(&cond, "finite"), (&with_nan, "with-nan")] {
            for (a, b) in [(1usize, 1usize), (3, 1), (5, 5), (10, 4), (30, 0), (70, 3)] {
                let args = vec![
                    Array1::from(series.clone()),
                    Array1::from(vec![a as f64]),
                    Array1::from(vec![b as f64]),
                ];
                let got = fn_last(&ctx, &args).unwrap();
                let len = series.len();
                let mut expected = Array1::zeros(len);
                for i in 0..len {
                    if i < a {
                        continue;
                    }
                    let mut all_true = true;
                    for j in (i - a)..=(i - b) {
                        if series[j] <= 0.0 || series[j].is_nan() {
                            all_true = false;
                            break;
                        }
                    }
                    expected[i] = if all_true { 1.0 } else { 0.0 };
                }
                assert_eq!(
                    got.as_slice().unwrap(),
                    expected.as_slice().unwrap(),
                    "LAST {name} A={a} B={b}"
                );
            }
        }
    }

    /// The monotone prefix-search fast path must agree bar for bar with the
    /// backwards rescan, on thresholds that are reached early, reached only
    /// from bar 0, and never reached at all.
    #[test]
    fn sumbars_matches_naive_walk() {
        let values: Vec<f64> = (0..120)
            .map(|i| 1.0 + ((i as f64) * 0.7).sin().abs() * 4.0)
            .collect();
        let ctx = context(Array1::from(values.clone()), Array1::ones(values.len()));
        for threshold in [0.0f64, 1.0, 7.5, 60.0, 1e9] {
            let args = vec![
                Array1::from(values.clone()),
                Array1::from_elem(values.len(), threshold),
            ];
            let got = fn_sumbars(&ctx, &args).unwrap();
            let mut expected = vec![f64::NAN; values.len()];
            for i in 0..values.len() {
                let mut cumsum = 0.0;
                let mut bars = 0.0;
                for j in (0..=i).rev() {
                    cumsum += values[j];
                    bars += 1.0;
                    if cumsum >= threshold {
                        break;
                    }
                }
                expected[i] = bars;
            }
            assert_eq!(
                got.as_slice().unwrap(),
                expected.as_slice(),
                "SUMBARS threshold={threshold}"
            );
        }

        // A per-bar threshold takes the same fast path (each search is
        // independent) and must still match.
        let varying: Vec<f64> = (0..values.len()).map(|i| 2.0 + (i as f64) * 0.05).collect();
        let args = vec![Array1::from(values.clone()), Array1::from(varying.clone())];
        let got = fn_sumbars(&ctx, &args).unwrap();
        let mut expected = vec![f64::NAN; values.len()];
        for i in 0..values.len() {
            let mut cumsum = 0.0;
            let mut bars = 0.0;
            for j in (0..=i).rev() {
                cumsum += values[j];
                bars += 1.0;
                if cumsum >= varying[i] {
                    break;
                }
            }
            expected[i] = bars;
        }
        assert_eq!(
            got.as_slice().unwrap(),
            expected.as_slice(),
            "SUMBARS varying threshold"
        );

        // Negative values break monotonicity and must keep the rescan: a window
        // can lose sum by growing, which the binary search cannot express.
        let signed = vec![-1.0f64, 5.0, -3.0, 4.0, 2.0];
        let signed_ctx = context(Array1::from(signed.clone()), Array1::ones(signed.len()));
        let args = vec![
            Array1::from(signed.clone()),
            Array1::from_elem(signed.len(), 3.0),
        ];
        let got = fn_sumbars(&signed_ctx, &args).unwrap();
        // i=0 never reaches 3 from bar 0 alone; i=2 needs all three bars
        // (-3 + 5 - 1 = 1 < 3) and reports 3 — the rescan, not a search.
        assert_eq!(got.as_slice().unwrap(), &[1.0, 1.0, 3.0, 1.0, 2.0]);
    }

    #[test]
    fn mth_recent_pivot_handles_few_pivots_and_zero_m() {
        let pivots = vec![(2usize, 30.0), (7, 40.0), (12, 35.0)];
        let mut result = vec![f64::NAN; 20];

        // m = 0 stays all-NaN (the old loop could never reach count == 0).
        fill_mth_recent_pivot(&pivots, 0, 20, &mut result, |_, _, pval| pval);
        assert!(result.iter().all(|v| v.is_nan()));

        // m = 2: bar 7 sees only one pivot before it, bar 8 onward sees two.
        fill_mth_recent_pivot(&pivots, 2, 20, &mut result, |_, _, pval| pval);
        assert!(result[..8].iter().all(|v| v.is_nan()));
        assert_eq!(result[8], 30.0, "second-most-recent pivot before bar 8");
        assert_eq!(result[13], 40.0, "second-most-recent pivot before bar 13");

        // m = 1 tracks the most recent pivot.
        fill_mth_recent_pivot(&pivots, 1, 20, &mut result, |_, _, pval| pval);
        assert_eq!(result[3], 30.0);
        assert_eq!(result[9], 40.0);
        assert_eq!(result[19], 35.0);

        // The bars variant reports (i - pidx) from the same window.
        let indices = vec![(2usize, 0.0), (7, 0.0), (12, 0.0)];
        fill_mth_recent_pivot(&indices, 1, 20, &mut result, |i, pidx, _| (i - pidx) as f64);
        assert_eq!(result[3], 1.0);
        assert_eq!(result[12], 5.0);
    }

    #[test]
    fn ma_and_terminal_sma_are_distinct_algorithms() {
        let values = array![1.0, 2.0, 3.0, 4.0, 5.0];
        let ctx = context(values.clone(), Array1::ones(values.len()));
        let n = scalar(values.len(), 3.0);
        let ma = fn_ma(&ctx, &[values.clone(), n.clone()]).unwrap();
        let sma = fn_sma(&ctx, &[values, n]).unwrap();
        assert!(ma[0].is_nan());
        assert!((ma[2] - 2.0).abs() < 1e-12);
        assert!((sma[2] - (17.0 / 9.0)).abs() < 1e-12);
        assert_ne!(ma[4], sma[4]);
    }

    #[test]
    fn registry_aliases_cannot_shadow_different_formula_implementations() {
        let map = get_builtin_functions();
        let registry = crate::registry::builtin_function_registry();
        for spec in registry.iter() {
            let Some(&canonical_fn) = map.get(spec.name) else {
                continue;
            };
            for &alias in spec.aliases {
                let alias_fn = *map
                    .get(alias)
                    .unwrap_or_else(|| panic!("registry alias {alias} is not executable"));
                assert!(
                    std::ptr::fn_addr_eq(alias_fn, canonical_fn),
                    "alias {alias} differs from canonical {}",
                    spec.name
                );
            }
        }
    }

    #[test]
    fn pine_source_overloads_preserve_requested_series() {
        let close = array![10.0, 20.0, 30.0, 40.0];
        let volume = array![1.0, 2.0, 1.0, 2.0];
        let ctx = context(close.clone(), volume.clone());
        let n = scalar(close.len(), 3.0);

        let cci = fn_cci(&ctx, &[close.clone(), n]).unwrap();
        assert!(cci[2].is_finite());

        let vwap = fn_vwap(&ctx, &[close, volume]).unwrap();
        assert!((vwap[0] - 10.0).abs() < 1e-12);
        assert!((vwap[1] - (50.0 / 3.0)).abs() < 1e-12);
        assert!((vwap[2] - 20.0).abs() < 1e-12);
    }

    #[test]
    fn adx_keeps_di_length_distinct_from_adx_smoothing() {
        let close = ndarray::array![10.0, 10.5, 11.0, 10.7, 11.6, 12.2, 11.9, 12.8, 13.4, 13.1];
        let volume = Array1::ones(close.len());
        let ctx = context(close.clone(), volume);
        let high = close.mapv(|v| v + 0.8);
        let low = close.mapv(|v| v - 0.6);
        let len = close.len();
        let di3 = scalar(len, 3.0);
        let adx2 = scalar(len, 2.0);
        let adx5 = scalar(len, 5.0);
        let fast = fn_adx(
            &ctx,
            &[high.clone(), low.clone(), close.clone(), di3.clone(), adx2],
        )
        .unwrap();
        let slow = fn_adx(&ctx, &[high, low, close, di3, adx5]).unwrap();
        assert!(fast
            .iter()
            .zip(slow.iter())
            .any(|(a, b)| a.is_finite() && b.is_finite() && (*a - *b).abs() > 1e-12));
    }

    #[test]
    fn sar_keeps_legacy_four_arg_form_and_separate_increment() {
        let high = array![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low = array![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let len = high.len();
        let ctx = context(array![9.5, 10.5, 11.5, 12.5, 13.5, 14.5], Array1::ones(len));
        let start = scalar(len, 0.02);
        let inc_small = scalar(len, 0.01);
        let inc_large = scalar(len, 0.05);
        let max = scalar(len, 0.2);

        let legacy = fn_sar(
            &ctx,
            &[high.clone(), low.clone(), start.clone(), max.clone()],
        )
        .unwrap();
        let small = fn_sar(
            &ctx,
            &[
                high.clone(),
                low.clone(),
                start.clone(),
                inc_small,
                max.clone(),
            ],
        )
        .unwrap();
        let large = fn_sar(&ctx, &[high, low, start, inc_large, max]).unwrap();
        assert_eq!(legacy.len(), len);
        assert_eq!(small.len(), len);
        assert_eq!(large.len(), len);
        assert!(small
            .iter()
            .zip(large.iter())
            .any(|(a, b)| (*a - *b).abs() > 1e-12));
    }
}
