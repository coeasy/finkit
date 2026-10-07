//! Contract test: every kernel the formula fast path can select is a **full
//! writer** — it overwrites every slot of the output buffer.
//!
//! `FormulaEngine`'s fast paths allocate their result buffer with
//! `uninit_output` (no `f64::NAN` pre-fill), which is only sound while this
//! holds. If a kernel is ever changed to leave a slot untouched — an early
//! return, a shorter write loop, an added `if` — the engine would read
//! uninitialised memory. This test fails first, naming the kernel.
//!
//! Method: poison the output slice with a signalling-NaN sentinel that no
//! kernel produces (`0x7FF0_0000_0000_0001`; the kernels emit the quiet NaN
//! `0x7FF8_0000_0000_0000`) and count survivors after the call.

use finkit::formula::{FormulaContext, FormulaEngine};
use finkit::indicators::volatility;
use finkit::math::{fast_moving_avg, moving_avg, simd_kernels, volume_kernels};
use ndarray::Array1;

const LEN: usize = 4_096;
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

/// Run `kernel` on a poisoned buffer and assert it left nothing behind.
fn assert_full_writer(name: &str, kernel: impl FnOnce(&mut [f64])) {
    let mut out = vec![sentinel(); LEN];
    kernel(&mut out);
    let left = survivors(&out);
    assert_eq!(
        left, 0,
        "{name} left {left}/{LEN} output slots untouched; the fast path's \
         uninit_output buffer would read uninitialised memory"
    );
}

#[test]
fn every_fast_path_kernel_writes_every_slot() {
    let input = series(LEN);
    let high = series(LEN);
    let low: Vec<f64> = high.iter().map(|v| v - 1.0).collect();
    let close = series(LEN);

    assert_full_writer("simd_kernels::sma_simd_into (MA/BOLLMID)", |out| {
        simd_kernels::sma_simd_into(&input, PERIOD, out);
    });
    assert_full_writer("fast_moving_avg::ema_into (EMA, into path)", |out| {
        let _ = fast_moving_avg::ema_into(&input, PERIOD, out);
    });
    assert_full_writer("simd_kernels::ema_simd_into (EMA, slices path)", |out| {
        simd_kernels::ema_simd_into(&input, PERIOD, out);
    });
    assert_full_writer("simd_kernels::rsi_simd_into (RSI)", |out| {
        simd_kernels::rsi_simd_into(&input, PERIOD, out);
    });
    assert_full_writer("moving_avg::wma_into (WMA)", |out| {
        let _ = moving_avg::wma_into(&input, PERIOD, out);
    });
    assert_full_writer("moving_avg::dema_into (DEMA)", |out| {
        let _ = moving_avg::dema_into(&input, PERIOD, out);
    });
    assert_full_writer("moving_avg::tema_into (TEMA)", |out| {
        let _ = moving_avg::tema_into(&input, PERIOD, out);
    });
    assert_full_writer("moving_avg::trima_into (TRIMA)", |out| {
        let _ = moving_avg::trima_into(&input, PERIOD, out);
    });
    assert_full_writer("moving_avg::kama_into (KAMA)", |out| {
        let _ = moving_avg::kama_into(&input, PERIOD, 2, 30, out);
    });
    assert_full_writer("volatility::atr_into (ATR, multi-input)", |out| {
        let _ = volatility::atr_into(&high, &low, &close, PERIOD, out);
    });
}

/// `indicators::ad` allocates its result with `uninit_output` -- it does **not**
/// pre-fill -- and the only thing making that sound is `ad_into` overwriting
/// every slot. The justification used to be a source comment with no test
/// behind it, which is exactly the kind of invariant that rots silently. ADOSC
/// keeps the same option open, and it has two kernels (a 3/10 fast path and a
/// general path), so both are pinned here too.
#[test]
fn uninit_allocated_volume_kernels_write_every_slot() {
    let high = series(LEN);
    let low: Vec<f64> = high.iter().map(|v| v - 1.0).collect();
    let close = series(LEN);
    let volume: Vec<f64> = (0..LEN).map(|i| 10_000.0 + (i as f64) * 3.0).collect();

    assert_full_writer("volume_kernels::ad_into (AD)", |out| {
        let _ = volume_kernels::ad_into(&high, &low, &close, &volume, out);
    });
    assert_full_writer(
        "volume_kernels::adosc_into (ADOSC, 3/10 fast path)",
        |out| {
            let _ = volume_kernels::adosc_into(&high, &low, &close, &volume, 3, 10, out);
        },
    );
    assert_full_writer("volume_kernels::adosc_into (ADOSC, general path)", |out| {
        let _ = volume_kernels::adosc_into(&high, &low, &close, &volume, 5, 20, out);
    });
}

fn ctx_from(close: &[f64]) -> FormulaContext {
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

/// The context path (`eval`) and the zero-copy slices path
/// (`eval_zero_copy_inputs`) must agree bit-for-bit for every name either fast
/// path serves. This is what caught the EMA split: the slices path used
/// `simd_kernels::ema_simd_into` while the context path used `ema_into`, and
/// the two differ by 16/10000 bits.
#[test]
fn fast_paths_agree_bit_for_bit_across_entry_points() {
    let close = series(LEN);
    let high: Vec<f64> = close.iter().map(|c| c + 1.0).collect();
    let low: Vec<f64> = close.iter().map(|c| c - 1.0).collect();
    let open: Vec<f64> = close.iter().map(|c| c - 0.3).collect();
    let volume: Vec<f64> = close.iter().map(|_| 10_000.0).collect();

    let mut engine = FormulaEngine::new();
    for source in [
        "MA(CLOSE,20)",
        "BOLLMID(CLOSE,20)",
        "EMA(CLOSE,20)",
        "RSI(CLOSE,20)",
        "WMA(CLOSE,20)",
        "DEMA(CLOSE,20)",
        "TEMA(CLOSE,20)",
        "TRIMA(CLOSE,20)",
        "KAMA(CLOSE,20)",
    ] {
        let compiled = engine.compile(source).expect("compile");
        let mut ctx = ctx_from(&close);
        let via_ctx = engine.eval(source, &mut ctx).expect("eval");
        let via_slices = engine
            .eval_zero_copy_inputs(&compiled, &open, &high, &low, &close, &volume, None)
            .expect("zero-copy eval");
        let diff = via_ctx
            .iter()
            .zip(via_slices.iter())
            .filter(|(a, b)| a.to_bits() != b.to_bits())
            .count();
        assert_eq!(
            diff,
            0,
            "{source}: eval and eval_zero_copy_inputs disagree on {diff}/{} values",
            via_ctx.len()
        );
    }
}
