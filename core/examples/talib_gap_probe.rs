//! TA-Lib parity probe — the reference C function and our kernel in **one
//! process**, interleaved, both allocating the way their public entry point
//! allocates.
//!
//! ```text
//! TA_LIB_DIR=/p/ta-lib-build/install \
//!   cargo run --release -p finkit --example talib_gap_probe --features talib-c
//! ```
//!
//! ## What this is, and what it is not
//!
//! **Screening, not verdicts.** `benches/talib_c_comparison.rs` remains the
//! authoritative measurement: it is what `docs/BENCHMARK_REPORT.md` is
//! generated from, and it reproduces its own numbers across processes to
//! within a few percent (checked: `FTA_MAX_30` 16.06 µs against the report's
//! 16.08 µs, `FTA_ADX_14` 74.31 against 75.42). This probe exists to answer
//! the question Criterion cannot answer in seconds — *is variant X of this
//! kernel faster than variant Y* — and to put the C library in the same
//! binary so that question can be asked about the reference itself.
//!
//! ## The harness, and the way an earlier version of it lied
//!
//! The first revision timed **one call at a time** and took the minimum of
//! 100. That looked rigorous and was not: measured that way, `indicators::max`
//! came out at 35.80 µs where Criterion reports 16.06, and `indicators::adx`
//! at 121.90 where Criterion reports 74.31 — both inflated by a *position
//! dependent* factor, so the same function measured 30.70 µs in one section
//! and 48.10 µs in another **in the same process**. Two conclusions were drawn
//! from those numbers ("ADX is 0.59x", "MAX is 0.41x") and both were wrong;
//! Criterion refuted them outright.
//!
//! Two changes fix it, and both are about the timed region rather than the
//! kernel:
//!
//! 1. **Batch the timed region.** One call is 16–120 µs, which is close enough
//!    to the cost of `Instant::now()`, of a single interrupt, and of one
//!    allocator slow path that none of them average out. Each timed block runs
//!    [`BATCH`] iterations and the block's *mean* is the sample.
//! 2. **Interleave the two sides.** `ours` and `theirs` alternate block by
//!    block, and each keeps its own minimum. Nothing is gained by measuring
//!    one side's 12 blocks before the other's — that is exactly how a
//!    slowly-drifting machine turns a scheduling difference into a ratio.
//!
//! The two null controls run first *and* last, at opposite ends of the program,
//! so a reader can see the harness's own drift instead of taking it on trust.
//!
//! ## Kernel sections and what they are for
//!
//! * `LINREG_SLOPE recurrence` — the previous update line against TA-Lib's,
//!   with seeding, allocation and output identical, so the only variable is the
//!   recurrence. It ran because our kernel was losing by a margin large enough
//!   to be worth a rewrite, and the question was whether the *shape* of the
//!   update was the cause. It was: the orientation change measured 0.678x on
//!   the hoisted form and `new-shipped vs C` then measured **1.048x**, i.e. the
//!   rewritten kernel is ahead of the C library it had been behind. The three
//!   forms are kept so the attribution stays reproducible.
//! * `rolling MAX` — heap tables (what used to ship), fixed-size stack tables
//!   (TA-Lib uses `double local_sufHighest[30]`), and a monotonic index ring.
//!   The stack tier now ships: `heap vs stack tables` reads 0.870x, and the two
//!   variants differ in nothing but where the tables live. The ring is
//!   **rejected** at 0.213x and recorded so nobody rebuilds it.
//! * `public tier` — our public function against the C function, exactly the
//!   pairing `docs/BENCHMARK_REPORT.md` publishes.
//! * `ULTOSC ring layout` — one ring per term (what ships now) against one ring
//!   per window (what shipped before). The three windows read the same two
//!   per-bar terms at three different ages, so the old layout stored each term
//!   three times. TA-Lib 0.8.1's `ta_ULTOSC.c` keeps a single
//!   `local_term_closeMinusTrueLow[32]` and reads it back at three offsets, which
//!   is what prompted the question; the section measures whether that structure
//!   is why it was ahead.
//!
//! ## What the `public tier` section still gets wrong
//!
//! **It understates implementations that allocate more than once.** The one
//! cross-check that disagrees with Criterion is in this section: interleaved,
//! `adx_14` reads 0.742x (ours 104.27 µs, C 77.40 µs) while Criterion — with
//! `FTA_ADX_14` and `TALib_ADX_14` built through the *same* C wrapper shape —
//! reads 1.008x (74.28 against 74.87 µs). Both harnesses build the C side
//! identically, so neither reading is a stale-binary or drift artefact.
//!
//! The harness's own null controls point the same way. The allocation-free one
//! (`line slope` against a second copy of itself) reads 1.008x at the top and
//! 0.996x at the bottom — 1% resolution. The allocation-heavy one (`aroon`
//! against a second copy of itself, two buffers each) reads 0.963x at the top
//! and 0.887x at the bottom: the same code twice, 8 points apart, because the
//! process's allocator state had drifted between the two.
//!
//! The difference is the allocation *shape*. In the rounds below, our side made
//! three allocations per call where the C side made one, and the two shapes
//! alternate block by block. A long single-function run (Criterion) repeats one
//! request pattern and keeps the allocator's free lists hot; an interleaved run
//! forces the allocator to service two different size/free-list patterns
//! alternately, and the shape that asks for more pays for it. The effect is
//! real, repeatable, and **not a property of the kernel** — which is why the
//! kernel sections above compare `_into` forms that own a caller buffer and
//! allocate nothing.
//!
//! So: use this section for *kernel* questions, where both sides can be made
//! allocation-free. For a public-tier ratio, `docs/BENCHMARK_REPORT.md` decides.
//! (The `adx` gap that this section reported was real in one sense — our public
//! entry point really did allocate three buffers where one would do, and that
//! has since been fixed — but the 0.742x figure overstated the cost of it.)

#![allow(clippy::too_many_lines)]

use std::hint::black_box;
use std::time::Instant;

use finkit::indicators;
use finkit::talib_ffi::*;

const DATA_LEN: usize = 10_000;
const BATCH: usize = 16;
const ROUNDS: usize = 12;

#[allow(clippy::type_complexity)]
fn create_ohlcv_data(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut close = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut volume = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f64;
        let noise = (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5 + (t * 3.71).sin() * 0.8;
        let trend = t * 0.01;
        let price = 100.0 + trend + noise;
        open.push(price - 0.3);
        high.push(price + 1.0 + ((t * 0.7).sin().abs() * 0.5));
        low.push(price - 1.0 - ((t * 0.5).cos().abs() * 0.5));
        close.push(price);
        volume.push(10000.0 + (t * 10.0).sin() * 3000.0 + 2000.0 * (t * 2.3).cos().abs());
    }
    (open, high, low, close, volume)
}

/// One timed block: `BATCH` iterations, reported per iteration.
fn time_block<T>(mut f: impl FnMut() -> T) -> f64 {
    let start = Instant::now();
    for _ in 0..BATCH {
        black_box(f());
    }
    start.elapsed().as_secs_f64() * 1e6 / BATCH as f64
}

fn head(title: &str) {
    println!("\n{title}");
}

/// `speedup = theirs / ours`; `>= 1.0` means we are ahead, matching the report.
fn report(label: &str, ours: f64, theirs: f64) {
    let speedup = theirs / ours;
    let mark = if speedup >= 1.0 {
        "ahead"
    } else if speedup >= 0.95 {
        "parity"
    } else {
        "BEHIND"
    };
    println!(
        "  {:<32} ours {:>8.2} us   theirs {:>8.2} us   {:>6.3}x  {}",
        label, ours, theirs, speedup, mark
    );
}

/// Interleaved, batched A/B. Both sides are warmed, then alternate blocks; the
/// reported figure is each side's best block.
fn duel<A, B>(label: &str, mut ours: impl FnMut() -> A, mut theirs: impl FnMut() -> B) {
    for _ in 0..2 {
        black_box(ours());
        black_box(theirs());
    }
    let mut best_ours = f64::INFINITY;
    let mut best_theirs = f64::INFINITY;
    for _ in 0..ROUNDS {
        let o = time_block(&mut ours);
        if o < best_ours {
            best_ours = o;
        }
        let t = time_block(&mut theirs);
        if t < best_theirs {
            best_theirs = t;
        }
    }
    report(label, best_ours, best_theirs);
}

fn mismatches(a: &[f64], b: &[f64], tol: f64) -> usize {
    a.iter()
        .zip(b)
        .filter(|(x, y)| !(x.is_nan() && y.is_nan()) && (**x - **y).abs() > tol)
        .count()
}

// ===========================================================================
// TA-Lib C wrappers. Each allocates its output the way the C caller must, i.e.
// the same `vec![0.0; len]` the benchmark's helpers use.
// ===========================================================================

macro_rules! c_wrap_period {
    ($name:ident, $sym:ident) => {
        fn $name(data: &[f64], period: i32) -> Vec<f64> {
            let len = data.len();
            let mut out = vec![0.0f64; len];
            let mut beg = 0i32;
            let mut nb = 0i32;
            unsafe {
                $sym(
                    0,
                    (len - 1) as i32,
                    data.as_ptr(),
                    period,
                    &mut beg,
                    &mut nb,
                    out.as_mut_ptr(),
                );
            }
            out
        }
    };
}

c_wrap_period!(c_linearreg, TA_LINEARREG);
c_wrap_period!(c_linearreg_slope, TA_LINEARREG_SLOPE);
c_wrap_period!(c_linearreg_intercept, TA_LINEARREG_INTERCEPT);
c_wrap_period!(c_linearreg_angle, TA_LINEARREG_ANGLE);
c_wrap_period!(c_tsf, TA_TSF);
c_wrap_period!(c_wma, TA_WMA);
c_wrap_period!(c_trima, TA_TRIMA);
c_wrap_period!(c_max, TA_MAX);
c_wrap_period!(c_min, TA_MIN);

fn c_var(data: &[f64], period: i32, nbdev: f64) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_VAR(
            0,
            (len - 1) as i32,
            data.as_ptr(),
            period,
            nbdev,
            &mut beg,
            &mut nb,
            out.as_mut_ptr(),
        );
    }
    out
}

fn c_cos(data: &[f64]) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_COS(
            0,
            (len - 1) as i32,
            data.as_ptr(),
            &mut beg,
            &mut nb,
            out.as_mut_ptr(),
        );
    }
    out
}

macro_rules! c_wrap_hlc {
    ($name:ident, $sym:ident) => {
        fn $name(h: &[f64], l: &[f64], c: &[f64], period: i32) -> Vec<f64> {
            let len = c.len();
            let mut out = vec![0.0f64; len];
            let mut beg = 0i32;
            let mut nb = 0i32;
            unsafe {
                $sym(
                    0,
                    (len - 1) as i32,
                    h.as_ptr(),
                    l.as_ptr(),
                    c.as_ptr(),
                    period,
                    &mut beg,
                    &mut nb,
                    out.as_mut_ptr(),
                );
            }
            out
        }
    };
}

c_wrap_hlc!(c_adx, TA_ADX);
c_wrap_hlc!(c_adxr, TA_ADXR);

fn c_aroon(h: &[f64], l: &[f64], period: i32) -> Vec<f64> {
    let len = h.len();
    let mut down = vec![0.0f64; len];
    let mut up = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_AROON(
            0,
            (len - 1) as i32,
            h.as_ptr(),
            l.as_ptr(),
            period,
            &mut beg,
            &mut nb,
            down.as_mut_ptr(),
            up.as_mut_ptr(),
        );
    }
    up
}

fn c_aroonosc(h: &[f64], l: &[f64], period: i32) -> Vec<f64> {
    let len = h.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_AROONOSC(
            0,
            (len - 1) as i32,
            h.as_ptr(),
            l.as_ptr(),
            period,
            &mut beg,
            &mut nb,
            out.as_mut_ptr(),
        );
    }
    out
}

fn c_ultosc(h: &[f64], l: &[f64], c: &[f64], p1: i32, p2: i32, p3: i32) -> Vec<f64> {
    let len = c.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_ULTOSC(
            0,
            (len - 1) as i32,
            h.as_ptr(),
            l.as_ptr(),
            c.as_ptr(),
            p1,
            p2,
            p3,
            &mut beg,
            &mut nb,
            out.as_mut_ptr(),
        );
    }
    out
}

fn c_ad(h: &[f64], l: &[f64], c: &[f64], v: &[f64]) -> Vec<f64> {
    let len = c.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_AD(
            0,
            (len - 1) as i32,
            h.as_ptr(),
            l.as_ptr(),
            c.as_ptr(),
            v.as_ptr(),
            &mut beg,
            &mut nb,
            out.as_mut_ptr(),
        );
    }
    out
}

/// Kernel tier for the slope: C writes into the caller's slice, no allocation.
fn c_linearreg_slope_into(data: &[f64], period: i32, out: &mut [f64]) {
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_LINEARREG_SLOPE(
            0,
            (data.len() - 1) as i32,
            data.as_ptr(),
            period,
            &mut beg,
            &mut nb,
            out.as_mut_ptr(),
        );
    }
}

fn c_max_into(data: &[f64], period: i32, out: &mut [f64]) {
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_MAX(
            0,
            (data.len() - 1) as i32,
            data.as_ptr(),
            period,
            &mut beg,
            &mut nb,
            out.as_mut_ptr(),
        );
    }
}

// ===========================================================================
// Candidate kernels. Each writes every slot from `window - 1` on, and assumes
// fully finite input (the regime the shipped kernels dispatch to their fast
// path on).
// ===========================================================================

/// The `LINREG_SLOPE` recurrence as it shipped **before** this round,
/// transcribed from the then-current `simd_ops::linreg_slope_avx2`: weight
/// `period - 1` on the newest bar, `sum_xy` advanced with
/// `(period - 1) * new - (sum_y - old)`, and the `usize -> f64` cast written
/// inside the loop.
///
/// Kept because it is the *baseline* half of the A/B that justified the
/// rewrite: `slope_form_previous_hoisted` isolates the cast, and
/// `slope_form_talib` isolates the orientation. Neither variant is dead
/// evidence — the veto condition for the rewrite was "if hoisting alone closes
/// the gap, do not change the numerics", and the `previous vs hoisted` duel is
/// what answers that.
fn slope_form_previous(data: &[f64], period: usize, out: &mut [f64]) {
    let len = data.len();
    let p = period as f64;
    let sum_x = p * (p - 1.0) / 2.0;
    let sum_x2 = p * (p - 1.0) * (2.0 * p - 1.0) / 6.0;
    let inv_denom = 1.0 / (p * sum_x2 - sum_x * sum_x);

    let mut sum_y = 0.0f64;
    let mut sum_xy = 0.0f64;
    for (j, &val) in data[..period].iter().enumerate() {
        sum_y += val;
        sum_xy += j as f64 * val;
    }
    out[period - 1] = (p * sum_xy - sum_x * sum_y) * inv_denom;

    for i in period..len {
        let old = data[i - period];
        let new = data[i];
        sum_xy += (period - 1) as f64 * new - (sum_y - old);
        sum_y += new - old;
        out[i] = (p * sum_xy - sum_x * sum_y) * inv_denom;
    }
}

/// The previous recurrence with the cast hoisted. Identical arithmetic,
/// identical orientation — the only change is that `period - 1` is converted to
/// `f64` once instead of on every bar. This exists to attribute the A/B: if
/// hoisting alone recovers the gap, the orientation change is unnecessary and
/// (being numerics-changing) should not be made.
fn slope_form_previous_hoisted(data: &[f64], period: usize, out: &mut [f64]) {
    let len = data.len();
    let p = period as f64;
    let p1 = (period - 1) as f64;
    let sum_x = p * (p - 1.0) / 2.0;
    let sum_x2 = p * (p - 1.0) * (2.0 * p - 1.0) / 6.0;
    let inv_denom = 1.0 / (p * sum_x2 - sum_x * sum_x);

    let mut sum_y = 0.0f64;
    let mut sum_xy = 0.0f64;
    for (j, &val) in data[..period].iter().enumerate() {
        sum_y += val;
        sum_xy += j as f64 * val;
    }
    out[period - 1] = (p * sum_xy - sum_x * sum_y) * inv_denom;

    for i in period..len {
        let old = data[i - period];
        let new = data[i];
        sum_xy += p1 * new - (sum_y - old);
        sum_y += new - old;
        out[i] = (p * sum_xy - sum_x * sum_y) * inv_denom;
    }
}

/// **TA-Lib's** recurrence, and since this round the one that actually ships.
/// One change from the previous form: the weight runs the other way — oldest
/// bar carries `period - 1` — which makes the update
/// `sum_xy += sum_y - period * old` instead of
/// `sum_xy += (period - 1) * new - (sum_y - old)`. The new bar then enters the
/// regression product only through `sum_y`, and both accumulators carry a
/// two-add chain instead of one two-add chain plus a two-sub dependency on the
/// other.
///
/// The output is the same number: TA-Lib's `Divisor` is the negative of the
/// previous `denom` and its `SumXY` the reflection of ours, so the two sign
/// flips cancel. `main` reports both the count of slots differing by more than
/// `1e-12` **and** the largest such difference, so the reader can see how far
/// inside the 1e-9 golden tolerance the reassociation actually lands.
fn slope_form_talib(data: &[f64], period: usize, out: &mut [f64]) {
    let len = data.len();
    let p = period as f64;
    let p1 = (period - 1) as f64;
    let sum_x = p * p1 / 2.0;
    let sum_x2 = p * p1 * (2.0 * p - 1.0) / 6.0;
    // TA-Lib: Divisor = SumX * SumX - period * SumXSqr (the negative of ours).
    let inv_divisor = 1.0 / (sum_x * sum_x - p * sum_x2);

    let mut sum_y = 0.0f64;
    let mut sum_xy = 0.0f64;
    let mut w = p1;
    for &val in data[..period].iter() {
        sum_y += val;
        sum_xy += w * val;
        w -= 1.0;
    }
    out[period - 1] = (p * sum_xy - sum_x * sum_y) * inv_divisor;

    for i in period..len {
        let old = data[i - period];
        let new = data[i];
        sum_xy += sum_y - p * old;
        sum_y += new - old;
        out[i] = (p * sum_xy - sum_x * sum_y) * inv_divisor;
    }
}

/// Van Herk with **heap** tables — what ships.
fn max_vh_heap(data: &[f64], window: usize, out: &mut [f64]) {
    let len = data.len();
    let mut suffix = vec![0.0f64; window];
    let mut prefix = vec![0.0f64; window];
    let mut block_start = 0usize;
    let mut today = window - 1;
    while today < len {
        let block_end = block_start + window - 1;
        let mut extreme = data[block_end];
        suffix[window - 1] = extreme;
        let mut offset = window - 1;
        while offset > 0 {
            offset -= 1;
            let value = data[block_start + offset];
            if value > extreme {
                extreme = value;
            }
            suffix[offset] = extreme;
        }
        out[today] = suffix[0];

        let block_next = block_start + window;
        if block_next >= len {
            break;
        }
        let n_available = (len - block_next).min(window - 1);
        extreme = data[block_next];
        prefix[0] = extreme;
        let mut grown = 1usize;
        while grown < n_available {
            let value = data[block_next + grown];
            if value > extreme {
                extreme = value;
            }
            prefix[grown] = extreme;
            grown += 1;
        }
        let mut offset = 1usize;
        while offset <= n_available {
            let a = prefix[offset - 1];
            let b = suffix[offset];
            out[today + offset] = if a > b { a } else { b };
            offset += 1;
        }
        block_start += window;
        today += n_available + 1;
    }
}

/// Van Herk with **fixed-size stack tables**, the shape the C reference uses
/// (`double local_sufHighest[30]`). Valid for `window <= CAP`.
fn max_vh_stack(data: &[f64], window: usize, out: &mut [f64]) {
    const CAP: usize = 64;
    let len = data.len();
    let mut suffix = [0.0f64; CAP];
    let mut prefix = [0.0f64; CAP];
    let mut block_start = 0usize;
    let mut today = window - 1;
    while today < len {
        let block_end = block_start + window - 1;
        let mut extreme = data[block_end];
        suffix[window - 1] = extreme;
        let mut offset = window - 1;
        while offset > 0 {
            offset -= 1;
            let value = data[block_start + offset];
            if value > extreme {
                extreme = value;
            }
            suffix[offset] = extreme;
        }
        out[today] = suffix[0];

        let block_next = block_start + window;
        if block_next >= len {
            break;
        }
        let n_available = (len - block_next).min(window - 1);
        extreme = data[block_next];
        prefix[0] = extreme;
        let mut grown = 1usize;
        while grown < n_available {
            let value = data[block_next + grown];
            if value > extreme {
                extreme = value;
            }
            prefix[grown] = extreme;
            grown += 1;
        }
        let mut offset = 1usize;
        while offset <= n_available {
            let a = prefix[offset - 1];
            let b = suffix[offset];
            out[today + offset] = if a > b { a } else { b };
            offset += 1;
        }
        block_start += window;
        today += n_available + 1;
    }
}

/// Monotonic index ring: at most two comparisons per bar, no tables at all.
/// Valid for `window <= CAP` because the ring never holds more than `window`
/// indices.
fn max_ring(data: &[f64], window: usize, out: &mut [f64]) {
    const CAP: usize = 64;
    let len = data.len();
    let mut ring = [0usize; CAP];
    let mut head = 0usize;
    let mut count = 0usize;
    for i in 0..len {
        let value = data[i];
        while count > 0 {
            let back = ring[(head + count - 1) & (CAP - 1)];
            if data[back] <= value {
                count -= 1;
            } else {
                break;
            }
        }
        ring[(head + count) & (CAP - 1)] = i;
        count += 1;
        while ring[head] + window <= i {
            head = (head + 1) & (CAP - 1);
            count -= 1;
        }
        out[i] = data[ring[head]];
    }
}

/// The Ultimate Oscillator layout this round replaced: one ring per window, so
/// each bar's two terms are stored three times (six stores against two), plus
/// the `abs() > 1e-15` divide guard. Transcribed from the removed
/// `ultosc_default_7_14_28_into` so that section D compares the two layouts and
/// nothing else.
fn ultosc_three_rings(high: &[f64], low: &[f64], close: &[f64]) -> Vec<f64> {
    let mut bp1 = [0.0; 7];
    let mut tr1 = [0.0; 7];
    let mut bp2 = [0.0; 14];
    let mut tr2 = [0.0; 14];
    let mut bp3 = [0.0; 28];
    let mut tr3 = [0.0; 28];
    let (mut bp1_sum, mut tr1_sum) = (0.0, 0.0);
    let (mut bp2_sum, mut tr2_sum) = (0.0, 0.0);
    let (mut bp3_sum, mut tr3_sum) = (0.0, 0.0);
    let (mut pos1, mut pos2, mut pos3) = (0usize, 0usize, 0usize);
    let mut out = vec![f64::NAN; high.len()];

    for i in 0..high.len() {
        let (bp, tr) = if i == 0 {
            (0.0, 0.0)
        } else {
            let tl = low[i].min(close[i - 1]);
            (close[i] - tl, high[i].max(close[i - 1]) - tl)
        };
        let (idx1, idx2, idx3) = (pos1, pos2, pos3);
        bp1_sum += bp - bp1[idx1];
        tr1_sum += tr - tr1[idx1];
        bp2_sum += bp - bp2[idx2];
        tr2_sum += tr - tr2[idx2];
        bp3_sum += bp - bp3[idx3];
        tr3_sum += tr - tr3[idx3];
        bp1[idx1] = bp;
        tr1[idx1] = tr;
        bp2[idx2] = bp;
        tr2[idx2] = tr;
        bp3[idx3] = bp;
        tr3[idx3] = tr;
        pos1 += 1;
        if pos1 == 7 {
            pos1 = 0;
        }
        pos2 += 1;
        if pos2 == 14 {
            pos2 = 0;
        }
        pos3 += 1;
        if pos3 == 28 {
            pos3 = 0;
        }

        if i >= 28 {
            let avg1 = if tr1_sum.abs() > 1e-15 {
                bp1_sum / tr1_sum
            } else {
                0.0
            };
            let avg2 = if tr2_sum.abs() > 1e-15 {
                bp2_sum / tr2_sum
            } else {
                0.0
            };
            let avg3 = if tr3_sum.abs() > 1e-15 {
                bp3_sum / tr3_sum
            } else {
                0.0
            };
            out[i] = 100.0 * (4.0 * avg1 + 2.0 * avg2 + avg3) / 7.0;
        }
    }
    out
}

/// The one-ring-per-term layout with the **old** band guard and no empty-window
/// reseed. Section D needs it: the shipped kernel differs from the layout it
/// replaced in two ways at once (where the terms live, and how the divide is
/// guarded), and a single A/B of the two cannot attribute the difference.
fn ultosc_one_ring_band(high: &[f64], low: &[f64], close: &[f64]) -> Vec<f64> {
    const RING: usize = 32;
    let mask = RING - 1;
    let mut bp_ring = [0.0f64; RING];
    let mut tr_ring = [0.0f64; RING];
    let (mut bp1_sum, mut tr1_sum) = (0.0, 0.0);
    let (mut bp2_sum, mut tr2_sum) = (0.0, 0.0);
    let (mut bp3_sum, mut tr3_sum) = (0.0, 0.0);
    let mut out = vec![f64::NAN; high.len()];

    for i in 1..high.len() {
        let write = i & mask;
        let t1 = i.wrapping_sub(7) & mask;
        let t2 = i.wrapping_sub(14) & mask;
        let t3 = i.wrapping_sub(28) & mask;

        let true_low = low[i].min(close[i - 1]);
        let bp = close[i] - true_low;
        let tr = high[i].max(close[i - 1]) - true_low;

        bp1_sum += bp - bp_ring[t1];
        tr1_sum += tr - tr_ring[t1];
        bp2_sum += bp - bp_ring[t2];
        tr2_sum += tr - tr_ring[t2];
        bp3_sum += bp - bp_ring[t3];
        tr3_sum += tr - tr_ring[t3];
        bp_ring[write] = bp;
        tr_ring[write] = tr;

        if i >= 28 {
            let avg1 = if tr1_sum.abs() > 1e-15 {
                bp1_sum / tr1_sum
            } else {
                0.0
            };
            let avg2 = if tr2_sum.abs() > 1e-15 {
                bp2_sum / tr2_sum
            } else {
                0.0
            };
            let avg3 = if tr3_sum.abs() > 1e-15 {
                bp3_sum / tr3_sum
            } else {
                0.0
            };
            out[i] = 100.0 * (4.0 * avg1 + 2.0 * avg2 + avg3) / 7.0;
        }
    }
    out
}

/// The shipped guard (`> 0.0` plus the empty-window reseed) on a ring whose
/// length is a **compile-time constant**, with the term ring on the stack.
///
/// This exists only to price one thing: the shipped kernel reaches its ring
/// through `&mut [f64]`, so its `mask` is `bp_ring.len() - 1` — a value the
/// optimiser cannot fold, which costs address arithmetic in the loop. If this
/// variant is materially faster than `indicators::ultosc`, the fix is a
/// const-generic ring length, not a different algorithm.
fn ultosc_one_ring_exact_const(high: &[f64], low: &[f64], close: &[f64]) -> Vec<f64> {
    const RING: usize = 32;
    const MASK: usize = RING - 1;
    let mut bp_ring = [0.0f64; RING];
    let mut tr_ring = [0.0f64; RING];
    let (mut bp1_sum, mut tr1_sum) = (0.0, 0.0);
    let (mut bp2_sum, mut tr2_sum) = (0.0, 0.0);
    let (mut bp3_sum, mut tr3_sum) = (0.0, 0.0);
    let mut null_run = 0usize;
    let mut out = vec![f64::NAN; high.len()];

    for i in 1..high.len() {
        let write = i & MASK;
        let t1 = i.wrapping_sub(7) & MASK;
        let t2 = i.wrapping_sub(14) & MASK;
        let t3 = i.wrapping_sub(28) & MASK;

        let true_low = low[i].min(close[i - 1]);
        let bp = close[i] - true_low;
        let tr = high[i].max(close[i - 1]) - true_low;

        bp1_sum += bp - bp_ring[t1];
        tr1_sum += tr - tr_ring[t1];
        bp2_sum += bp - bp_ring[t2];
        tr2_sum += tr - tr_ring[t2];
        bp3_sum += bp - bp_ring[t3];
        tr3_sum += tr - tr_ring[t3];
        bp_ring[write] = bp;
        tr_ring[write] = tr;

        if bp == 0.0 && tr == 0.0 {
            null_run += 1;
        } else {
            null_run = 0;
        }
        if null_run >= 7 {
            if null_run >= 7 {
                bp1_sum = 0.0;
                tr1_sum = 0.0;
            }
            if null_run >= 14 {
                bp2_sum = 0.0;
                tr2_sum = 0.0;
            }
            if null_run >= 28 {
                bp3_sum = 0.0;
                tr3_sum = 0.0;
            }
        }

        if i >= 28 {
            let avg1 = if tr1_sum > 0.0 {
                bp1_sum / tr1_sum
            } else {
                0.0
            };
            let avg2 = if tr2_sum > 0.0 {
                bp2_sum / tr2_sum
            } else {
                0.0
            };
            let avg3 = if tr3_sum > 0.0 {
                bp3_sum / tr3_sum
            } else {
                0.0
            };
            out[i] = 100.0 * (4.0 * avg1 + 2.0 * avg2 + avg3) / 7.0;
        }
    }
    out
}

/// A series that moves and then goes perfectly flat, so eventually every window
/// fills with bars whose two Ultimate Oscillator terms are exactly zero. That is
/// the input that reaches a fixed-band divide guard.
/// The moving part uses an irrational-ish wobble on purpose: with hand-picked
/// short decimals the partial sums can stay exactly representable and the
/// emptied window lands on a clean `0.0`, which would hide the thing being
/// tested rather than exercise it.
fn flat_tail_series() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    const N: usize = 120;
    let mut high = vec![0.0; N];
    let mut low = vec![0.0; N];
    let mut close = vec![0.0; N];
    for i in 0..60 {
        let wobble = (i as f64 * 0.37).sin() * 3.0;
        high[i] = 100.0 + wobble + 1.2;
        low[i] = 100.0 + wobble - 1.4;
        close[i] = 100.0 + wobble + 0.3;
    }
    let price = close[59];
    for i in 60..N {
        // H == L == previous close, so both terms are exactly zero.
        high[i] = price;
        low[i] = price;
        close[i] = price;
    }
    (high, low, close)
}

/// Min and max of the finite entries, used to ask whether an oscillator stayed
/// inside its documented range.
fn finite_range(values: &[f64]) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for &v in values.iter().filter(|v| !v.is_nan()) {
        lo = lo.min(v);
        hi = hi.max(v);
    }
    (lo, hi)
}

fn main() {
    let (_open, high, low, close, volume) = create_ohlcv_data(DATA_LEN);
    // The math-transform group's own input: `|close| * 0.001 + 0.5`.
    let trig: Vec<f64> = close.iter().map(|v| v.abs() * 0.001 + 0.5).collect();
    let mut out = vec![0.0f64; DATA_LEN];
    let mut alt = vec![0.0f64; DATA_LEN];

    head("===== NULL CONTROLS (opening; same call twice, ratio = resolution) =====");
    duel(
        "line slope ours (a/b)",
        || finkit::math::linear::linreg_slope(&close, 14).unwrap(),
        || finkit::math::linear::linreg_slope(&close, 14).unwrap(),
    );
    duel(
        "sma proxy: aroon (a/b)",
        || indicators::aroon(&high, &low, 14).unwrap(),
        || indicators::aroon(&high, &low, 14).unwrap(),
    );

    head("===== A. LINREG_SLOPE recurrence, identical seeding and emit =====");
    let mut hoisted_out = vec![0.0f64; DATA_LEN];
    slope_form_previous_hoisted(&close, 14, &mut hoisted_out);
    slope_form_talib(&close, 14, &mut alt);
    // How far apart the two orientations actually are, not just how many slots
    // exceed a threshold. `DEFAULT_TOLERANCE` in the golden loader is 1e-9 and
    // the TA-Lib numeric contract is 1e-8; the point of printing the maximum is
    // that the reassociation has to be orders of magnitude inside both, and a
    // count alone does not show that.
    let worst = hoisted_out
        .iter()
        .zip(alt.iter())
        .filter(|(a, b)| !(a.is_nan() && b.is_nan()))
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f64, f64::max);
    println!(
        "    previous vs new-shipped: {} slots over 1e-12, worst {:.3e} \
         (golden tol 1e-9, TA-Lib contract 1e-8)",
        mismatches(&hoisted_out, &alt, 1e-12),
        worst
    );
    // Three duels, each isolating one difference:
    //   cast hoisted?  -> the `usize as f64` in the previous loop
    //   orientation?   -> the update line itself
    //   compiler?      -> the Rust transcript against the C library
    duel(
        "previous vs cast-hoisted",
        || slope_form_previous(black_box(&close), 14, &mut out),
        || slope_form_previous_hoisted(black_box(&close), 14, &mut alt),
    );
    duel(
        "hoisted vs new-shipped",
        || slope_form_previous_hoisted(black_box(&close), 14, &mut out),
        || slope_form_talib(black_box(&close), 14, &mut alt),
    );
    duel(
        "new-shipped vs C",
        || slope_form_talib(black_box(&close), 14, &mut out),
        || c_linearreg_slope_into(black_box(&close), 14, &mut alt),
    );

    head("===== B. rolling MAX, kernel tier (window 30) =====");
    max_vh_stack(&close, 30, &mut out);
    let mut stack_copy = vec![0.0f64; DATA_LEN];
    stack_copy.copy_from_slice(&out);
    max_ring(&close, 30, &mut alt);
    // Compare from `window - 1` on: below that the ring writes real values
    // while the block kernel leaves the warm-up prefix to its caller.
    println!(
        "    stack vs ring mismatches @0: {}",
        mismatches(&stack_copy[29..], &alt[29..], 0.0)
    );
    duel(
        "heap vs stack tables",
        || max_vh_heap(black_box(&close), 30, &mut out),
        || max_vh_stack(black_box(&close), 30, &mut alt),
    );
    duel(
        "ring vs stack tables",
        || max_ring(black_box(&close), 30, &mut out),
        || max_vh_stack(black_box(&close), 30, &mut alt),
    );
    duel(
        "stack vs C",
        || max_vh_stack(black_box(&close), 30, &mut out),
        || c_max_into(black_box(&close), 30, &mut alt),
    );

    head("===== C. PUBLIC TIER: the derived-quantity family =====");
    duel(
        "line slope",
        || finkit::math::linear::linreg_slope(&close, 14).unwrap(),
        || c_linearreg_slope(&close, 14),
    );
    duel(
        "line intercept",
        || finkit::math::linear::linreg_intercept(&close, 14).unwrap(),
        || c_linearreg_intercept(&close, 14),
    );
    duel(
        "line angle",
        || finkit::math::linear::linreg_angle(&close, 14).unwrap(),
        || c_linearreg_angle(&close, 14),
    );
    duel(
        "linear_reg",
        || indicators::linear_reg(&close, 14).unwrap(),
        || c_linearreg(&close, 14),
    );
    duel(
        "tsf",
        || indicators::tsf(&close, 14).unwrap(),
        || c_tsf(&close, 14),
    );
    duel(
        "max_30",
        || indicators::max(&close, 30).unwrap(),
        || c_max(&close, 30),
    );
    duel(
        "min_30",
        || indicators::min(&close, 30).unwrap(),
        || c_min(&close, 30),
    );
    duel(
        "var_20",
        || indicators::var(&close, 20, 1.0).unwrap(),
        || c_var(&close, 20, 1.0),
    );
    duel(
        "wma_20",
        || indicators::wma(&close, 20).unwrap(),
        || c_wma(&close, 20),
    );
    duel(
        "trima_20",
        || indicators::trima(&close, 20).unwrap(),
        || c_trima(&close, 20),
    );
    duel(
        "cos (trig input)",
        || indicators::cos(&trig).unwrap(),
        || c_cos(&trig),
    );
    duel(
        "adx_14",
        || indicators::adx(&high, &low, &close, 14).unwrap(),
        || c_adx(&high, &low, &close, 14),
    );
    duel(
        "adxr_14",
        || indicators::adxr(&high, &low, &close, 14).unwrap(),
        || c_adxr(&high, &low, &close, 14),
    );
    duel(
        "aroon_14",
        || indicators::aroon(&high, &low, 14).unwrap(),
        || c_aroon(&high, &low, 14),
    );
    duel(
        "aroonosc_14",
        || indicators::aroonosc(&high, &low, 14).unwrap(),
        || c_aroonosc(&high, &low, 14),
    );
    duel(
        "ultosc_7_14_28",
        || indicators::ultosc(&high, &low, &close, 7, 14, 28).unwrap(),
        || c_ultosc(&high, &low, &close, 7, 14, 28),
    );
    duel(
        "ad",
        || indicators::ad(&high, &low, &close, &volume).unwrap(),
        || c_ad(&high, &low, &close, &volume),
    );

    head("===== D. PUBLIC TIER: ULTOSC, one ring per term vs one per window =====");
    // The shipped kernel differs from the layout it replaced in **two** ways at
    // once -- where the per-bar terms live, and how the divide is guarded -- so
    // the pair is priced twice, with a variant that changes only the first.
    duel(
        "ultosc (null control, a/b)",
        || indicators::ultosc(&high, &low, &close, 7, 14, 28).unwrap(),
        || indicators::ultosc(&high, &low, &close, 7, 14, 28).unwrap(),
    );
    duel(
        "ultosc layout only: one ring vs three rings (band guard both sides)",
        || ultosc_one_ring_band(&high, &low, &close),
        || ultosc_three_rings(&high, &low, &close),
    );
    duel(
        "ultosc guard only: exact vs band (one-ring layout both sides)",
        || indicators::ultosc(&high, &low, &close, 7, 14, 28).unwrap(),
        || ultosc_one_ring_band(&high, &low, &close),
    );
    duel(
        "ultosc ring length: const (stack) vs runtime (slice), same guard",
        || ultosc_one_ring_exact_const(&high, &low, &close),
        || indicators::ultosc(&high, &low, &close, 7, 14, 28).unwrap(),
    );
    duel(
        "ultosc: shipped vs the layout it replaced (both changes)",
        || indicators::ultosc(&high, &low, &close, 7, 14, 28).unwrap(),
        || ultosc_three_rings(&high, &low, &close),
    );
    duel(
        "ultosc: shipped vs C",
        || indicators::ultosc(&high, &low, &close, 7, 14, 28).unwrap(),
        || c_ultosc(&high, &low, &close, 7, 14, 28),
    );
    {
        // A speed ratio between two computations that disagree is meaningless,
        // so the layouts are checked against each other before the ratios above
        // are read. The two accumulate the same values in the same order, so
        // this is an equality check rather than a tolerance.
        let shipped = indicators::ultosc(&high, &low, &close, 7, 14, 28).unwrap();
        let previous = ultosc_three_rings(&high, &low, &close);
        let worst = shipped
            .iter()
            .zip(previous.iter())
            .filter(|(a, b)| !(a.is_nan() && b.is_nan()))
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f64, f64::max);
        println!(
            "    layouts agree: {} slots over 1e-12, worst {:.3e}",
            mismatches(shipped.as_slice().unwrap(), &previous, 1e-12),
            worst
        );
    }
    {
        // Does the band actually fire? A window that has emptied holds rounding
        // residue, and the band test accepts it if the residue is still above
        // 1e-15. Whether that happens is a measurement, not a deduction, so both
        // layouts are run on a series built to reach the situation.
        let (fhigh, flow, fclose) = flat_tail_series();
        let previous = ultosc_three_rings(&fhigh, &flow, &fclose);
        let shipped = indicators::ultosc(&fhigh, &flow, &fclose, 7, 14, 28).unwrap();
        let (plo, phi) = finite_range(&previous);
        let (slo, shi) = finite_range(shipped.as_slice().unwrap());
        println!(
            "    flat window, output range: three-ring [band guard] [{plo:.4}, {phi:.4}], \
             shipped [exact guard] [{slo:.4}, {shi:.4}]  (defined on 0..100)"
        );
        if (plo, phi) == (slo, shi) {
            println!(
                "    -> the band did **not** fire on this series; the defect is latent here, \
                 not reproduced"
            );
        }
    }

    head("===== E. PUBLIC TIER: AROON / AROONOSC, rescan unroll (TA_UNROLL(4)) =====");
    // Both our `ExtremeTracker` and TA-Lib's `TA_AROON` use the identical
    // cached-extreme-index algorithm: a full-window rescan fires only when the
    // cached extreme leaves the window. The one concrete delta is TA-Lib's
    // `TA_UNROLL(4)` in that rescan, which our plain `for` lacked. Section E
    // measures whether the 4-wide unroll added to `rescan_extreme_window`
    // closes the ~17% gap (`aroon_14` 0.83x, `aroonosc_14` 0.81x in the report).
    duel(
        "aroon_14: ours vs C",
        || indicators::aroon(&high, &low, 14).unwrap(),
        || c_aroon(&high, &low, 14),
    );
    duel(
        "aroonosc_14: ours vs C",
        || indicators::aroonosc(&high, &low, 14).unwrap(),
        || c_aroonosc(&high, &low, 14),
    );

    head("===== NULL CONTROLS (closing; compare with the opening pair) =====");
    duel(
        "line slope ours (a/b)",
        || finkit::math::linear::linreg_slope(&close, 14).unwrap(),
        || finkit::math::linear::linreg_slope(&close, 14).unwrap(),
    );
    duel(
        "sma proxy: aroon (a/b)",
        || indicators::aroon(&high, &low, 14).unwrap(),
        || indicators::aroon(&high, &low, 14).unwrap(),
    );
}
