//! Prove that every kernel reachable from the formula fast path overwrites
//! **every** output slot.
//!
//! The fast-path result buffer is currently allocated with
//! `Array1::from_elem(len, f64::NAN)`, which costs a full-length store pass
//! (~10 us / 10k bars, more on longer series). Swapping that for
//! `uninit_output` removes the pass, but is only sound if each kernel writes
//! every slot — otherwise the caller reads uninitialised memory.
//!
//! Method: poison the output slice with a signalling-NaN sentinel that no
//! kernel ever produces (`0x7FF0_0000_0000_0001`; the kernels emit the quiet
//! NaN `0x7FF8_0000_0000_0000`), run the kernel, and count survivors.
//! `0` survivors == the kernel is a full writer and `uninit_output` is safe.

use finkit::indicators::volatility;
use finkit::math::{fast_moving_avg, moving_avg, simd_kernels};

const LEN: usize = 10_000;
const PERIOD: usize = 20;
const SENTINEL: u64 = 0x7FF0_0000_0000_0001;

fn sentinel() -> f64 {
    f64::from_bits(SENTINEL)
}

fn survivors(out: &[f64]) -> usize {
    out.iter().filter(|v| v.to_bits() == SENTINEL).count()
}

fn series(len: usize) -> Vec<f64> {
    (0..len)
        .map(|i| {
            let i = i as f64;
            100.0 + (i * 0.37).sin() * 5.0 + (i * 0.011).cos() * 2.0 + i * 0.0003
        })
        .collect()
}

fn report(name: &str, out: &[f64], note: &str) {
    let left = survivors(out);
    let verdict = if left == 0 {
        "FULL-WRITER -> uninit_output OK"
    } else {
        "PARTIAL -> must keep NaN fill"
    };
    println!(
        "{name:<28} survivors={left:>6} / {:<6}  {verdict}  {note}",
        out.len()
    );
}

fn main() {
    let input = series(LEN);
    let high = series(LEN);
    let low: Vec<f64> = high.iter().map(|v| v - 1.0).collect();
    let close = series(LEN);

    println!("== formula fast-path kernels: output-slot coverage (sentinel probe) ==");
    println!("sentinels are signalling NaN {SENTINEL:#x}; a full writer leaves 0\n");

    // --- single-input kernels, called exactly as the engine calls them ---
    let mut out = vec![sentinel(); LEN];
    simd_kernels::sma_simd_into(&input, PERIOD, &mut out);
    report("MA/BOLLMID sma_simd_into", &out, "");

    let mut out = vec![sentinel(); LEN];
    let _ = fast_moving_avg::ema_into(&input, PERIOD, &mut out);
    report("EMA fast_moving_avg::ema_into", &out, "into-path");

    let mut out = vec![sentinel(); LEN];
    simd_kernels::ema_simd_into(&input, PERIOD, &mut out);
    report("EMA simd_kernels::ema_simd_into", &out, "slices-path");

    let mut out = vec![sentinel(); LEN];
    simd_kernels::rsi_simd_into(&input, PERIOD, &mut out);
    report("RSI rsi_simd_into", &out, "");

    let mut out = vec![sentinel(); LEN];
    let _ = moving_avg::wma_into(&input, PERIOD, &mut out);
    report("WMA moving_avg::wma_into", &out, "");

    let mut out = vec![sentinel(); LEN];
    let _ = moving_avg::dema_into(&input, PERIOD, &mut out);
    report("DEMA moving_avg::dema_into", &out, "");

    let mut out = vec![sentinel(); LEN];
    let _ = moving_avg::tema_into(&input, PERIOD, &mut out);
    report("TEMA moving_avg::tema_into", &out, "");

    let mut out = vec![sentinel(); LEN];
    let _ = moving_avg::trima_into(&input, PERIOD, &mut out);
    report("TRIMA moving_avg::trima_into", &out, "");

    let mut out = vec![sentinel(); LEN];
    let _ = moving_avg::kama_into(&input, PERIOD, 2, 30, &mut out);
    report("KAMA moving_avg::kama_into", &out, "");

    // --- multi-input kernel: ATR(HIGH, LOW, CLOSE, N) ---
    let mut out = vec![sentinel(); LEN];
    let _ = volatility::atr_into(&high, &low, &close, PERIOD, &mut out);
    report("ATR volatility::atr_into", &out, "multi-input");

    println!("\nAny PARTIAL row means the buffer must stay NaN-filled (or the");
    println!("kernel must be extended to write the full span) before uninit_output.");
}
