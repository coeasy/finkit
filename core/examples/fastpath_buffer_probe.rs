//! Quantify the formula fast-path buffer change: `Array1::from_elem(NAN)`
//! pre-fill -> `uninit_output`, and KAMA's allocating `kama` + copy ->
//! `kama_into` straight into the buffer.
//!
//! Everything is measured in ONE process, interleaved, median of rounds, so
//! the numbers are not contaminated by the cross-run drift that makes
//! separate binaries (±20%) useless for micro-deltas.
//!
//! `fastpath_fill_probe` proves the resulting buffers are safe (every kernel
//! is a full writer); this probe shows what that safety buys.

#![allow(clippy::uninit_vec)]

use finkit::formula::{FormulaContext, FormulaEngine};
use finkit::math::moving_avg;
use ndarray::Array1;
use std::hint::black_box;
use std::time::Instant;

const ROUNDS: usize = 31;
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

/// Same allocation `crate::utils::uninit_output` performs (it is `pub(crate)`).
unsafe fn uninit_vec(len: usize) -> Vec<f64> {
    let mut v = Vec::with_capacity(len);
    unsafe { v.set_len(len) };
    v
}

fn median(samples: &mut [f64]) -> f64 {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    samples[samples.len() / 2]
}

/// Interleaved A/B: runs `a` on even rounds and `b` on odd rates in one loop.
fn duel<A: FnMut() -> f64, B: FnMut() -> f64>(rounds: usize, mut a: A, mut b: B) -> (f64, f64) {
    let mut sa = Vec::with_capacity(rounds);
    let mut sb = Vec::with_capacity(rounds);
    for round in 0..rounds {
        if round % 2 == 0 {
            let t = Instant::now();
            for _ in 0..REPEATS {
                black_box(a());
            }
            sa.push(t.elapsed().as_secs_f64() * 1e6 / REPEATS as f64);

            let t = Instant::now();
            for _ in 0..REPEATS {
                black_box(b());
            }
            sb.push(t.elapsed().as_secs_f64() * 1e6 / REPEATS as f64);
        } else {
            let t = Instant::now();
            for _ in 0..REPEATS {
                black_box(b());
            }
            sb.push(t.elapsed().as_secs_f64() * 1e6 / REPEATS as f64);

            let t = Instant::now();
            for _ in 0..REPEATS {
                black_box(a());
            }
            sa.push(t.elapsed().as_secs_f64() * 1e6 / REPEATS as f64);
        }
    }
    (median(&mut sa), median(&mut sb))
}

fn line(label: &str, old: f64, new: f64) {
    let ratio = old / new;
    let verdict = if ratio > 1.02 {
        format!("{ratio:.2}x faster")
    } else if ratio < 0.98 {
        format!("{:.2}x slower", ratio)
    } else {
        "~same".to_string()
    };
    println!("  {label:<40} old {old:>10.2} us | new {new:>10.2} us | {verdict}");
}

fn main() {
    for (len, tag) in [(10_000usize, "10k"), (100_000usize, "100k")] {
        let close = make_close(len);
        let mut engine = FormulaEngine::new();
        // warm up
        for _ in 0..3 {
            let mut ctx = make_ctx(&close);
            let _ = engine.eval("KAMA(CLOSE,20)", &mut ctx);
            let _ = engine.eval("WMA(CLOSE,20)", &mut ctx);
            let _ = moving_avg::kama(&close, 20, 2, 30);
        }

        println!("\n=== {tag} bars (n={len}) ===");

        // --- 1. the buffer pre-fill alone ---
        let (fill, nofill) = duel(
            ROUNDS,
            || {
                let a = Array1::from_elem(len, f64::NAN);
                black_box(a.as_slice().unwrap()[len - 1]);
                0.0
            },
            || {
                let a = Array1::from_vec(unsafe { uninit_vec(len) });
                unsafe { black_box(a.as_ptr().add(len - 1).read()) };
                0.0
            },
        );
        line("buffer: from_elem(NAN) vs uninit", fill, nofill);

        // --- 2. KAMA full work: allocating + copy vs kama_into ---
        let (kama_old, kama_new) = duel(
            ROUNDS,
            || {
                let mut output = Array1::from_elem(len, f64::NAN);
                let values = moving_avg::kama(&close, 20, 2, 30).unwrap();
                output
                    .as_slice_mut()
                    .unwrap()
                    .copy_from_slice(values.as_slice().unwrap());
                black_box(output.as_slice().unwrap()[len - 1]);
                0.0
            },
            || {
                let mut output = Array1::from_vec(unsafe { uninit_vec(len) });
                moving_avg::kama_into(&close, 20, 2, 30, output.as_slice_mut().unwrap()).unwrap();
                black_box(output.as_slice().unwrap()[len - 1]);
                0.0
            },
        );
        line("KAMA work: alloc+copy vs kama_into", kama_old, kama_new);

        // --- 3. engine end-to-end (what a user actually calls) ---
        for src in [
            "KAMA(CLOSE,20)",
            "WMA(CLOSE,20)",
            "TRIMA(CLOSE,20)",
            "ATR(HIGH,LOW,CLOSE,14)",
        ] {
            let mut ctx = make_ctx(&close);
            let mut samples = Vec::with_capacity(ROUNDS);
            for _ in 0..ROUNDS {
                let t = Instant::now();
                for _ in 0..REPEATS {
                    black_box(engine.eval(src, &mut ctx).unwrap());
                }
                samples.push(t.elapsed().as_secs_f64() * 1e6 / REPEATS as f64);
            }
            println!(
                "  {:<40} {:>10.2} us",
                format!("engine.eval({src})"),
                median(&mut samples)
            );
        }
    }

    // --- 4. slices path EMA now matches eval EMA bit-for-bit ---
    let close = make_close(10_000);
    let high: Vec<f64> = close.iter().map(|c| c + 1.0).collect();
    let low: Vec<f64> = close.iter().map(|c| c - 1.0).collect();
    let open: Vec<f64> = close.iter().map(|c| c - 0.3).collect();
    let vol: Vec<f64> = close.iter().map(|_| 10_000.0).collect();
    let mut engine = FormulaEngine::new();

    println!("\n=== zero-copy entry point (eval_zero_copy_inputs) @ 10k ===");
    for src in [
        "MA(CLOSE,20)",
        "BOLLMID(CLOSE,20)",
        "EMA(CLOSE,20)",
        "RSI(CLOSE,20)",
        "WMA(CLOSE,20)",
        "DEMA(CLOSE,20)",
        "TEMA(CLOSE,20)",
        "TRIMA(CLOSE,20)",
        "KAMA(CLOSE,20)",
        "SUM(CLOSE,20)",
    ] {
        let compiled = engine.compile(src).unwrap();
        for _ in 0..3 {
            let _ = engine.eval_zero_copy_inputs(&compiled, &open, &high, &low, &close, &vol, None);
        }
        let mut samples = Vec::with_capacity(ROUNDS);
        for _ in 0..ROUNDS {
            let t = Instant::now();
            for _ in 0..REPEATS {
                black_box(
                    engine
                        .eval_zero_copy_inputs(&compiled, &open, &high, &low, &close, &vol, None)
                        .unwrap(),
                );
            }
            samples.push(t.elapsed().as_secs_f64() * 1e6 / REPEATS as f64);
        }
        let note = if src.starts_with("SUM") {
            "  (not fast-pathed: general-executor baseline)"
        } else {
            ""
        };
        println!("  {src:<20}{:>10.2} us{note}", median(&mut samples));
    }

    let mut ctx = make_ctx(&close);
    let via_ctx = engine.eval("EMA(CLOSE,20)", &mut ctx).unwrap();
    let compiled = engine.compile("EMA(CLOSE,20)").unwrap();
    let via_slices = engine
        .eval_zero_copy_inputs(&compiled, &open, &high, &low, &close, &vol, None)
        .unwrap();
    let diff = via_ctx
        .iter()
        .zip(via_slices.iter())
        .filter(|(a, b)| a.to_bits() != b.to_bits())
        .count();
    println!("\n=== entry-point agreement ===");
    println!(
        "  eval vs eval_zero_copy_inputs  EMA(CLOSE,20)  bit-diff {diff}/{}",
        via_ctx.len()
    );
}
