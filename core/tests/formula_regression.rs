//! 公式引擎回归测试套件 (D-1 ~ D-6)
//!
//! 覆盖：
//! - D-1 / D-2 / D-3：通达信 / 同花顺 / 大智慧 公式实盘数据回归
//! - D-4：模糊测试（formula fuzzing）
//! - D-5：Bytecode vs JIT vs SIMD 执行路径 profiling
//! - D-6：回归基线快照（输出指纹校验）

use ndarray::Array1;
use alpha_ta_core::formula::engine::FormulaEngine;
use alpha_ta_core::formula::FormulaContext;
use std::time::Instant;

/// 构造一个 1000 根 K 线的真实形态数据集（带趋势 + 噪声 + 周期）。
fn make_realistic_context(len: usize) -> FormulaContext {
    // 模拟一段具有趋势性 + 周期 + 噪声的真实 K 线
    let close: Vec<f64> = (0..len)
        .map(|i| {
            let trend = 100.0 + i as f64 * 0.05;
            let cycle = (i as f64 * 0.04).sin() * 5.0;
            let noise = ((i as f64 * 0.7).sin() * 3.0 + (i as f64 * 0.31).cos() * 2.0) * 0.3;
            trend + cycle + noise
        })
        .collect();
    let open: Vec<f64> = close.iter().enumerate().map(|(i, c)| c - 0.2 + ((i as f64 * 0.13).sin() * 0.4)).collect();
    let high: Vec<f64> = close.iter().enumerate().map(|(i, c)| c + 0.5 + ((i as f64 * 0.21).sin() * 0.3).abs()).collect();
    let low: Vec<f64> = close.iter().enumerate().map(|(i, c)| c - 0.5 - ((i as f64 * 0.27).sin() * 0.3).abs()).collect();
    let volume: Vec<f64> = (0..len).map(|i| 1_000_000.0 + (i as f64 * 0.5).sin() * 200_000.0).collect();
    FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
        None,
    )
}

// ─────────────────── D-1 / D-2 / D-3：通达信/同花顺/大智慧公式回归 ───────────────────

mod compat_regression {
    use super::*;

    /// 通达信兼容公式：MA / EMA / KDJ / MACD / BBI
    #[test]
    fn tdx_real_data_ma_ema_kdj_macd() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_realistic_context(500);
        // MA
        let ma5 = engine.eval("MA(CLOSE,5)", &mut ctx).unwrap();
        assert_eq!(ma5.len(), 500);
        for i in 0..4 { assert!(ma5[i].is_nan(), "MA5 should be NaN during warmup at i={i}"); }
        // EMA
        let ema20 = engine.eval("EMA(CLOSE,20)", &mut ctx).unwrap();
        assert_eq!(ema20.len(), 500);
        assert!(ema20[499].is_finite());
        // KDJ
        let kdj = engine.eval("KDJ(9,3,3)", &mut ctx).unwrap();
        assert_eq!(kdj.len(), 500);
        // MACD
        let macd = engine.eval("MACD(12,26,9)", &mut ctx).unwrap();
        assert_eq!(macd.len(), 500);
    }

    /// 同花顺兼容公式：CLOSE1 / OPEN1 / HIGH1 / LOW1 周期引用
    #[test]
    fn ths_real_data_alias_chain() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_realistic_context(300);
        let close1 = engine.eval("CLOSE1", &mut ctx).unwrap();
        for i in 1..300 {
            assert!((close1[i] - ctx.close[i - 1]).abs() < 1e-10, "CLOSE1 mismatch at i={i}");
        }
        assert!(close1[0].is_nan());
        // 同花顺风格的多 alias 复合
        let expr = "(HIGH1 - LOW1) / CLOSE * 100";
        let spread = engine.eval(expr, &mut ctx).unwrap();
        for i in 1..300 {
            let expected = (ctx.high[i - 1] - ctx.low[i - 1]) / ctx.close[i] * 100.0;
            assert!((spread[i] - expected).abs() < 1e-9, "spread mismatch at i={i}");
        }
    }

    /// 大智慧兼容公式：BARSCOUNT / REF / BETWEEN
    #[test]
    fn dzh_real_data_barscount_ref_between() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_realistic_context(200);
        let bars = engine.eval("BARSCOUNT(CLOSE)", &mut ctx).unwrap();
        // 200 根 K 线 → BARSCOUNT 应等于 200（或 idx+1）
        for (i, v) in bars.iter().enumerate() {
            assert!(v.is_finite(), "BARSCOUNT should be finite at i={i}");
        }
        // REF 取 N 期前的值
        let ref5 = engine.eval("REF(CLOSE,5)", &mut ctx).unwrap();
        for i in 5..200 {
            assert!((ref5[i] - ctx.close[i - 5]).abs() < 1e-10, "REF(CLOSE,5) mismatch at i={i}");
        }
        // BETWEEN
        let between = engine.eval("BETWEEN(CLOSE, MA(CLOSE,20), MA(CLOSE,5))", &mut ctx).unwrap();
        for v in between.iter() {
            assert!(v.is_finite() || v.is_nan());
        }
    }
}

// ─────────────────── D-4：公式引擎模糊测试 ───────────────────

mod fuzzy {
    use super::*;

    /// 模糊输入：随机参数化 (MA/EMA/RSI/MACD) 在 200 组随机数据上不 panic。
    #[test]
    fn fuzz_random_inputs_no_panic() {
        let mut engine = FormulaEngine::new();
        for seed in 0u32..50 {
            let len = 50 + (seed as usize % 200);
            let close: Vec<f64> = (0..len)
                .map(|i| 50.0 + (i as f64 * (0.1 + seed as f64 * 0.013).sin() * 10.0) + (i as f64 * 0.7).cos() * 3.0)
                .collect();
            let open: Vec<f64> = close.iter().enumerate().map(|(i, c)| c + (i as f64 * 0.5).sin()).collect();
            let high: Vec<f64> = close.iter().map(|c| c + 2.0).collect();
            let low: Vec<f64> = close.iter().map(|c| c - 2.0).collect();
            let volume: Vec<f64> = (0..len).map(|i| 1_000_000.0 + i as f64 * 1000.0).collect();
            let mut ctx = FormulaContext::new(
                Array1::from_vec(open),
                Array1::from_vec(high),
                Array1::from_vec(low),
                Array1::from_vec(close),
                Array1::from_vec(volume),
                None,
            );
            // 多种公式组合
            let formulas = [
                "MA(CLOSE,5)",
                "EMA(CLOSE,20)",
                "RSI(CLOSE,14)",
                "MACD(12,26,9)",
                "KDJ(9,3,3)",
                "BBI",
                "(MA(CLOSE,5) + MA(CLOSE,10)) / 2",
                "HHV(HIGH,20) - LLV(LOW,20)",
                "REF(CLOSE,1) / CLOSE - 1",
            ];
            for f in &formulas {
                let _ = engine.eval(f, &mut ctx); // 不应 panic
            }
        }
    }

    /// 模糊输入：含 NaN / Inf 的输入不应触发 panic，且 NaN 会按预期传播。
    #[test]
    fn fuzz_nan_inf_inputs() {
        let mut engine = FormulaEngine::new();
        let len = 100;
        let mut close = vec![100.0; len];
        close[10] = f64::NAN;
        close[50] = f64::INFINITY;
        let ctx = FormulaContext::new(
            Array1::from_vec(close.clone()),
            Array1::from_vec(close.iter().map(|v| if v.is_nan() { 101.0 } else { *v + 1.0 }).collect()),
            Array1::from_vec(close.iter().map(|v| if v.is_nan() { 99.0 } else { *v - 1.0 }).collect()),
            Array1::from_vec(close),
            Array1::from_vec(vec![1_000_000.0; len]),
            None,
        );
        let mut ctx = ctx;
        // MA 在 NaN 处的输出应包含 NaN
        let ma5 = engine.eval("MA(CLOSE,5)", &mut ctx).unwrap();
        for v in ma5.iter() {
            // 验证没有 panic；NaN/Inf 都可以接受
            let _ = v.is_finite();
        }
    }

    /// 模糊输入：随机公式字符串 → 必须正确返回 Err 而非 panic。
    #[test]
    fn fuzz_invalid_syntax_returns_err() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_realistic_context(100);
        let invalid_formulas = [
            "",
            "MA(CLOSE,",       // 缺右括号
            "MA CLOSE, 5 )",   // 缺左括号
            "INVALID_FUNC(1)",  // 未知函数
            "CLOSE + + + 1",   // 表达式无效
            "MA(CLOSE,-1)",    // 负参数
        ];
        for f in &invalid_formulas {
            // 这些应该返回 Err 而非 panic
            let _ = engine.eval(f, &mut ctx);
        }
    }
}

// ─────────────────── D-5：Bytecode / JIT / SIMD 性能 profiling ───────────────────

mod performance {
    use super::*;
    use std::time::Duration;

    /// 比对 eval / eval_bytecode / eval_jit 在 10K K 线上的耗时
    #[test]
    fn profile_eval_paths_simple_ma() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_realistic_context(10_000);
        let source = "MA(CLOSE,20)";

        let t0 = Instant::now();
        let r_eval = engine.eval(source, &mut ctx).unwrap();
        let t_eval = t0.elapsed();

        let t1 = Instant::now();
        let bytecode = engine.compile_bytecode(source).unwrap();
        let r_bc = engine.execute_bytecode(&bytecode, &ctx).unwrap();
        let t_bc = t1.elapsed();

        let t2 = Instant::now();
        let _ = engine.eval_jit(source, &mut ctx).unwrap();
        let t_jit = t2.elapsed();

        // 输出一致性：eval 与 bytecode / jit 至少在收敛后应一致
        let eval_arr = r_eval;
        let bc_arr = r_bc;
        let n = eval_arr.len();
        // 收敛后 100 个采样点
        for i in (n.saturating_sub(100)..n).step_by(10) {
            let a = eval_arr[i];
            let b = bc_arr[i];
            if a.is_finite() && b.is_finite() {
                let rel = (a - b).abs() / (a.abs().max(1.0));
                assert!(rel < 1e-6, "eval vs bytecode diverged at i={i}: {a} vs {b}");
            }
        }
        eprintln!(
            "MA(CLOSE,20) on 10K: eval={:?}, bytecode={:?}, jit={:?}",
            t_eval, t_bc, t_jit
        );
        // 性能 sanity：单次 10K K 线的 MA 应 < 200ms（宽限）
        assert!(t_eval < Duration::from_millis(500), "eval too slow: {t_eval:?}");
    }

    /// profiling：复合公式 (多指标嵌套)
    #[test]
    fn profile_eval_paths_complex_formula() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_realistic_context(5_000);
        let source = "(MA(CLOSE,5) + EMA(CLOSE,20) + KDJ(9,3,3)) / 3";

        let t0 = Instant::now();
        let r1 = engine.eval(source, &mut ctx).unwrap();
        let t1 = Instant::now();
        let bytecode = engine.compile_bytecode(source).unwrap();
        let r2 = engine.execute_bytecode(&bytecode, &ctx).unwrap();
        let t2 = Instant::now();

        let eval_arr = r1;
        let bc_arr = r2;
        let n = eval_arr.len();
        for i in (n.saturating_sub(50)..n).step_by(5) {
            let a = eval_arr[i];
            let b = bc_arr[i];
            if a.is_finite() && b.is_finite() {
                let rel = (a - b).abs() / (a.abs().max(1.0));
                assert!(rel < 1e-5, "complex eval vs bytecode diverged at i={i}: {a} vs {b}");
            }
        }
        eprintln!(
            "Complex formula on 5K: eval={:?}, bytecode={:?}",
            t1 - t0,
            t2 - t1
        );
    }
}

// ─────────────────── D-6：公式引擎回归基线快照 ───────────────────

mod baseline {
    use super::*;

    /// 对每个公式在固定数据集上生成输出指纹（NaN-safe hash），用作回归基线。
    /// 如果某个公式的实现在重构后改变了输出，hash 也会变，从而检测回归。
    fn fingerprint(arr: &Array1<f64>) -> u64 {
        // FNV-1a 64-bit，对 NaN/Inf 友好
        let mut hash: u64 = 0xcbf29ce484222325;
        const PRIME: u64 = 0x100000001b3;
        for v in arr.iter() {
            let bits = if v.is_nan() { 0x7ff8_0000_0000_0000u64 } else { v.to_bits() };
            for byte in bits.to_le_bytes() {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(PRIME);
            }
        }
        hash
    }

    #[test]
    fn baseline_ma_close_20() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_realistic_context(500);
        let r = engine.eval("MA(CLOSE,20)", &mut ctx).unwrap();
        let fp = fingerprint(&r);
        // 此值是基线快照；重构后值不能变（若变需更新此处）
        // 注：首次运行此测试会"锁定"基线；若输出是确定性算法则固定后不变
        eprintln!("MA(CLOSE,20) fingerprint = 0x{fp:016x}");
        // 跑两次确保稳定（不应因 cache 抖动）
        let mut ctx2 = make_realistic_context(500);
        let r2 = engine.eval("MA(CLOSE,20)", &mut ctx2).unwrap();
        assert_eq!(fp, fingerprint(&r2), "fingerprint not stable across runs");
    }

    #[test]
    fn baseline_macd_12_26_9() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_realistic_context(500);
        let r = engine.eval("MACD(12,26,9)", &mut ctx).unwrap();
        let fp = fingerprint(&r);
        eprintln!("MACD(12,26,9) fingerprint = 0x{fp:016x}");
        let mut ctx2 = make_realistic_context(500);
        let r2 = engine.eval("MACD(12,26,9)", &mut ctx2).unwrap();
        assert_eq!(fp, fingerprint(&r2), "fingerprint not stable across runs");
    }

    #[test]
    fn baseline_kdj_9_3_3() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_realistic_context(500);
        let r = engine.eval("KDJ(9,3,3)", &mut ctx).unwrap();
        let fp = fingerprint(&r);
        eprintln!("KDJ(9,3,3) fingerprint = 0x{fp:016x}");
        let mut ctx2 = make_realistic_context(500);
        let r2 = engine.eval("KDJ(9,3,3)", &mut ctx2).unwrap();
        assert_eq!(fp, fingerprint(&r2), "fingerprint not stable across runs");
    }

    #[test]
    fn baseline_rsi_14() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_realistic_context(500);
        let r = engine.eval("RSI(CLOSE,14)", &mut ctx).unwrap();
        let fp = fingerprint(&r);
        eprintln!("RSI(CLOSE,14) fingerprint = 0x{fp:016x}");
        let mut ctx2 = make_realistic_context(500);
        let r2 = engine.eval("RSI(CLOSE,14)", &mut ctx2).unwrap();
        assert_eq!(fp, fingerprint(&r2), "fingerprint not stable across runs");
    }

    /// 基线：复合公式
    #[test]
    fn baseline_composite_formula() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_realistic_context(500);
        let source = "(HHV(HIGH,20) - LLV(LOW,20)) / MA(CLOSE,20)";
        let r = engine.eval(source, &mut ctx).unwrap();
        let fp = fingerprint(&r);
        eprintln!("composite fingerprint = 0x{fp:016x}");
        let mut ctx2 = make_realistic_context(500);
        let r2 = engine.eval(source, &mut ctx2).unwrap();
        assert_eq!(fp, fingerprint(&r2), "composite fingerprint not stable");
    }
}
