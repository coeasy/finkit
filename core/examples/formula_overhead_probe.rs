//! Decompose the formula engine's overhead over a direct kernel call.
//!
//! `EMA(CLOSE, 20)` in the engine costs ~49 us while `moving_avg::ema` on the
//! same slice costs ~18 us. Both end up in the *same* kernel, so the gap is
//! pure wrapper cost. This probe measures each candidate contributor
//! separately, in one process, interleaved.

use finkit::formula::{FormulaContext, FormulaEngine};
use finkit::math::moving_avg;
use ndarray::Array1;
use std::time::Instant;

const LEN: usize = 10_000;
const ROUNDS: usize = 25;
const REPEATS: usize = 20;

fn make_close(len: usize) -> Vec<f64> {
    (0..len)
        .map(|i| {
            let t = i as f64;
            100.0 + t * 0.01 + (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5
        })
        .collect()
}

fn make_ctx(close: &[f64]) -> FormulaContext {
    let high: Vec<f64> = close.iter().map(|c| c + 1.0).collect();
    let low: Vec<f64> = close.iter().map(|c| c - 1.0).collect();
    let open: Vec<f64> = close.iter().map(|c| c - 0.3).collect();
    let volume: Vec<f64> = close.iter().map(|_| 10_000.0).collect();
    FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close.to_vec()),
        Array1::from_vec(volume),
        None,
    )
}

fn median(samples: &mut [f64]) -> f64 {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    samples[samples.len() / 2]
}

/// Kernels the engine's fast path can pick, measured without allocation by
/// writing into a preallocated buffer (which is what the fast path does).
fn bench_kernel(name: &str, close: &[f64], rounds: usize, repeats: usize) -> f64 {
    let mut out = vec![0.0f64; close.len()];
    let mut samples = Vec::with_capacity(rounds);
    for _ in 0..3 {
        match name {
            "ema_simd_into" => finkit::math::simd_kernels::ema_simd_into(close, 20, &mut out),
            "ema_fast_into" => finkit::math::simd_kernels::ema_fast_into(close, 20, &mut out),
            "sma_simd_into" => finkit::math::simd_kernels::sma_simd_into(close, 20, &mut out),
            _ => unreachable!(),
        }
    }
    for _ in 0..rounds {
        let start = Instant::now();
        for _ in 0..repeats {
            match name {
                "ema_simd_into" => finkit::math::simd_kernels::ema_simd_into(close, 20, &mut out),
                "ema_fast_into" => finkit::math::simd_kernels::ema_fast_into(close, 20, &mut out),
                "sma_simd_into" => finkit::math::simd_kernels::sma_simd_into(close, 20, &mut out),
                _ => unreachable!(),
            }
        }
        samples.push(start.elapsed().as_secs_f64() * 1e6 / repeats as f64);
    }
    median(&mut samples)
}

fn main() {
    let close = make_close(LEN);
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("EMA(CLOSE, 20)").expect("compile");

    let mut kernel = Vec::with_capacity(ROUNDS);
    let mut kernel_owned = Vec::with_capacity(ROUNDS);
    let mut clone_only = Vec::with_capacity(ROUNDS);
    let mut ctx_build = Vec::with_capacity(ROUNDS);
    let mut var_view = Vec::with_capacity(ROUNDS);
    let mut engine_exec = Vec::with_capacity(ROUNDS);
    let mut engine_eval = Vec::with_capacity(ROUNDS);

    for _ in 0..3 {
        let mut ctx = make_ctx(&close);
        let _ = engine.execute(&compiled, &mut ctx);
        let _ = engine.eval("EMA(CLOSE, 20)", &mut ctx);
        let _ = moving_avg::ema(&close, 20);
        let _ = Array1::from_vec(close.clone());
    }

    for round in 0..ROUNDS {
        let forward = round % 2 == 0;
        let order: Vec<u8> = if forward {
            vec![0, 1, 2, 3, 4, 5, 6]
        } else {
            vec![6, 5, 4, 3, 2, 1, 0]
        };
        for slot in order {
            match slot {
                // 1. the kernel on a borrowed slice: the floor
                0 => {
                    let start = Instant::now();
                    for _ in 0..REPEATS {
                        std::hint::black_box(moving_avg::ema(&close, 20).unwrap());
                    }
                    kernel.push(start.elapsed().as_secs_f64() * 1e6 / REPEATS as f64);
                }
                // 2. clone into an Array1 first, then the kernel on its slice
                1 => {
                    let start = Instant::now();
                    for _ in 0..REPEATS {
                        let owned = Array1::from_vec(close.clone());
                        std::hint::black_box(moving_avg::ema(owned.as_slice().unwrap(), 20).unwrap());
                    }
                    kernel_owned.push(start.elapsed().as_secs_f64() * 1e6 / REPEATS as f64);
                }
                // 3. the clone alone
                2 => {
                    let start = Instant::now();
                    for _ in 0..REPEATS {
                        std::hint::black_box(Array1::from_vec(close.clone()));
                    }
                    clone_only.push(start.elapsed().as_secs_f64() * 1e6 / REPEATS as f64);
                }
                // 4. building a FormulaContext
                3 => {
                    let start = Instant::now();
                    for _ in 0..REPEATS {
                        std::hint::black_box(make_ctx(&close));
                    }
                    ctx_build.push(start.elapsed().as_secs_f64() * 1e6 / REPEATS as f64);
                }
                // 5. resolving CLOSE to an owned array through the context
                4 => {
                    let ctx = make_ctx(&close);
                    let start = Instant::now();
                    for _ in 0..REPEATS {
                        std::hint::black_box(ctx.close_view().to_owned());
                    }
                    var_view.push(start.elapsed().as_secs_f64() * 1e6 / REPEATS as f64);
                }
                // 6. engine execute on a pre-compiled formula
                5 => {
                    let mut ctx = make_ctx(&close);
                    let start = Instant::now();
                    for _ in 0..REPEATS {
                        std::hint::black_box(engine.execute(&compiled, &mut ctx).unwrap());
                    }
                    engine_exec.push(start.elapsed().as_secs_f64() * 1e6 / REPEATS as f64);
                }
                // 7. engine eval from source (cache lookup included)
                _ => {
                    let mut ctx = make_ctx(&close);
                    let start = Instant::now();
                    for _ in 0..REPEATS {
                        std::hint::black_box(engine.eval("EMA(CLOSE, 20)", &mut ctx).unwrap());
                    }
                    engine_eval.push(start.elapsed().as_secs_f64() * 1e6 / REPEATS as f64);
                }
            }
        }
    }

    let k = median(&mut kernel);
    let ko = median(&mut kernel_owned);
    let cl = median(&mut clone_only);
    let cb = median(&mut ctx_build);
    let vv = median(&mut var_view);
    let ee = median(&mut engine_exec);
    let ev = median(&mut engine_eval);

    println!("EMA(CLOSE,20) @ {LEN} bars — 开销分解（单进程交错，中位数）\n");
    println!("  {:<44}{:>10.2} µs", "1. moving_avg::ema(&slice, 20)  [内核底线]", k);
    println!("  {:<44}{:>10.2} µs", "2. clone→Array1 + 同内核", ko);
    println!("  {:<44}{:>10.2} µs", "3. 仅 Array1::from_vec(clone)", cl);
    println!("  {:<44}{:>10.2} µs", "4. 仅构建 FormulaContext", cb);
    println!("  {:<44}{:>10.2} µs", "5. 仅 ctx.close_view().to_owned()", vv);
    println!("  {:<44}{:>10.2} µs", "6. engine.execute(&compiled, ctx)", ee);
    println!("  {:<44}{:>10.2} µs", "7. engine.eval(source, ctx)", ev);
    println!("\n  可解释部分 (3) clone            = {:>7.2} µs", cl);
    println!("  未解释部分 (6 - 1 - 3)          = {:>7.2} µs", ee - k - cl);
    println!("  引擎总开销 (6 - 1)              = {:>7.2} µs  ({:.2}x)", ee - k, ee / k);

    println!("\n内核选型对比（均写入预分配缓冲，无分配）\n");
    for candidate in ["ema_simd_into", "ema_fast_into", "sma_simd_into"] {
        println!(
            "  {:<32}{:>10.2} µs",
            candidate,
            bench_kernel(candidate, &close, ROUNDS, REPEATS)
        );
    }

    // Numerical agreement between the EMA kernel candidates. The engine's fast
    // path must stay bit-identical to the general executor, so a faster kernel
    // is only adoptable inside the golden tolerance.
    let n = close.len();
    let mut a = vec![0.0f64; n];
    let mut b = vec![0.0f64; n];
    let mut c = vec![0.0f64; n];
    finkit::math::simd_kernels::ema_simd_into(&close, 20, &mut a);
    let _ = finkit::math::moving_avg::ema_into(&close, 20, &mut b);
    finkit::math::simd_kernels::ema_fast_into(&close, 20, &mut c);

    let cmp = |x: &[f64], y: &[f64], label: &str| {
        let mut bit_diff = 0usize;
        let mut max_abs = 0.0f64;
        let mut max_rel = 0.0f64;
        for (u, v) in x.iter().zip(y.iter()) {
            if u.to_bits() != v.to_bits() {
                bit_diff += 1;
            }
            let d = (u - v).abs();
            if d > max_abs {
                max_abs = d;
            }
            let denom = u.abs().max(v.abs());
            if denom > 0.0 {
                let r = d / denom;
                if r > max_rel {
                    max_rel = r;
                }
            }
        }
        println!(
            "  {label:<34} 位差 {bit_diff:>5}/{n}  max_abs {max_abs:.3e}  max_rel {max_rel:.3e}"
        );
    };
    // The contract that actually matters: the fast path must be bit-identical
    // to the general executor, which reaches `fn_ema` -> `moving_avg::ema`.
    let general = finkit::math::moving_avg::ema(&close, 20).unwrap();
    let general = general.as_slice().expect("Array1 is contiguous");
    println!("\nEMA 内核 vs 通用路径 (fn_ema -> moving_avg::ema)\n");
    cmp(&b, general, "moving_avg::ema_into (快路径当前选择)");
    cmp(&a, general, "simd_kernels::ema_simd_into (旧快路径)");
    cmp(&c, general, "ema_fast_into (AVX2 block-prefix)");
    println!("\n（对照）三内核互比，基准为 simd 现行内核\n");
    cmp(&b, &a, "moving_avg::ema_into vs ema_simd_into");
}
