//! ULTOSC kernel probe: decompose the ~10 us gap to TA-Lib on the 10k-bar series.
//!
//! The shipped kernel sits at 0.85x (ours 66.9 us vs C 57.1 us @ 10k). The
//! candidates are all structural, so this probe measures them against the
//! shipped shape in-process, ours-vs-ours, with interleaved batched blocks:
//!
//! - `shipped`     : one loop `1..len` with a per-bar `if i >= max_period`.
//! - `split`       : warm-up loop `1..max_period` (sums + ring only), then a
//!                   main loop `max_period..len` with no watermark branch --
//!                   the two-loop shape `ta_ULTOSC.c` uses.
//! - `split_nodiv` : `split` with the three guarded divides replaced by a
//!                   constant. Diagnostic only: quantifies the divide share.
//! - `split_recip` : `split` with the final `/ 7.0` turned into `* (1.0/7.0)`.
//!                   Diagnostic only: NOT bit-identical, so not shippable.
//!
//! Run: `cargo run --release --example ultosc_probe`

use std::hint::black_box;
use std::time::Instant;

const BATCH: usize = 64;
const ROUNDS: usize = 9;
const DATA_LEN: usize = 10_000;
const P1: usize = 7;
const P2: usize = 14;
const P3: usize = 28;

fn bars(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f64;
        let noise = (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5 + (t * 3.71).sin() * 0.8;
        let price = 100.0 + t * 0.01 + noise;
        high.push(price + 1.0 + (t * 0.7).sin().abs() * 0.5);
        low.push(price - 1.0 - (t * 0.5).cos().abs() * 0.5);
        close.push(price);
    }
    (high, low, close)
}

fn time_block<T>(mut f: impl FnMut() -> T) -> f64 {
    let start = Instant::now();
    for _ in 0..BATCH {
        black_box(f());
    }
    start.elapsed().as_secs_f64() * 1e6 / BATCH as f64
}

fn duel<A, B>(label: &str, mut base: impl FnMut() -> A, mut cand: impl FnMut() -> B) {
    for _ in 0..2 {
        black_box(base());
        black_box(cand());
    }
    let (mut best_base, mut best_cand) = (f64::INFINITY, f64::INFINITY);
    for _ in 0..ROUNDS {
        let b = time_block(&mut base);
        if b < best_base {
            best_base = b;
        }
        let c = time_block(&mut cand);
        if c < best_cand {
            best_cand = c;
        }
    }
    let delta = (best_cand - best_base) / best_base * 100.0;
    println!(
        "  {:<14} base {:>7.2} us   cand {:>7.2} us   {:>+6.2}%",
        label, best_base, best_cand, delta
    );
}

#[inline(always)]
fn dx_like_guards(
    bp1: f64,
    tr1: f64,
    bp2: f64,
    tr2: f64,
    bp3: f64,
    tr3: f64,
) -> f64 {
    let avg1 = if tr1 > 0.0 { bp1 / tr1 } else { 0.0 };
    let avg2 = if tr2 > 0.0 { bp2 / tr2 } else { 0.0 };
    let avg3 = if tr3 > 0.0 { bp3 / tr3 } else { 0.0 };
    100.0 * (4.0 * avg1 + 2.0 * avg2 + avg3) / 7.0
}

/// The shipped shape: one loop, per-bar watermark branch.
fn shipped(high: &[f64], low: &[f64], close: &[f64], out: &mut [f64]) {
    let max_period = P1.max(P2).max(P3);
    let ring = (max_period + 1).next_power_of_two();
    let mask = ring - 1;
    let mut bp_ring = vec![0.0f64; ring];
    let mut tr_ring = vec![0.0f64; ring];
    let mut bp1 = 0.0;
    let mut tr1 = 0.0;
    let mut bp2 = 0.0;
    let mut tr2 = 0.0;
    let mut bp3 = 0.0;
    let mut tr3 = 0.0;
    let mut null_run = 0usize;
    let shortest = P1.min(P2).min(P3);
    for i in 1..high.len() {
        let write = i & mask;
        let t1 = i.wrapping_sub(P1) & mask;
        let t2 = i.wrapping_sub(P2) & mask;
        let t3 = i.wrapping_sub(P3) & mask;
        let true_low = low[i].min(close[i - 1]);
        let bp = close[i] - true_low;
        let tr = high[i].max(close[i - 1]) - true_low;
        bp1 += bp - bp_ring[t1];
        tr1 += tr - tr_ring[t1];
        bp2 += bp - bp_ring[t2];
        tr2 += tr - tr_ring[t2];
        bp3 += bp - bp_ring[t3];
        tr3 += tr - tr_ring[t3];
        bp_ring[write] = bp;
        tr_ring[write] = tr;
        if bp == 0.0 && tr == 0.0 {
            null_run += 1;
        } else {
            null_run = 0;
        }
        if null_run >= shortest {
            if null_run >= P1 {
                bp1 = 0.0;
                tr1 = 0.0;
            }
            if null_run >= P2 {
                bp2 = 0.0;
                tr2 = 0.0;
            }
            if null_run >= P3 {
                bp3 = 0.0;
                tr3 = 0.0;
            }
        }
        if i >= max_period {
            out[i] = dx_like_guards(bp1, tr1, bp2, tr2, bp3, tr3);
        }
    }
}

/// Two loops: the warm-up carries no watermark branch and no divides.
fn split(high: &[f64], low: &[f64], close: &[f64], out: &mut [f64]) {
    let max_period = P1.max(P2).max(P3);
    let ring = (max_period + 1).next_power_of_two();
    let mask = ring - 1;
    let mut bp_ring = vec![0.0f64; ring];
    let mut tr_ring = vec![0.0f64; ring];
    let mut bp1 = 0.0;
    let mut tr1 = 0.0;
    let mut bp2 = 0.0;
    let mut tr2 = 0.0;
    let mut bp3 = 0.0;
    let mut tr3 = 0.0;
    let mut null_run = 0usize;
    let shortest = P1.min(P2).min(P3);
    let len = high.len();
    let warm_end = max_period.min(len);

    for i in 1..warm_end {
        let write = i & mask;
        let t1 = i.wrapping_sub(P1) & mask;
        let t2 = i.wrapping_sub(P2) & mask;
        let t3 = i.wrapping_sub(P3) & mask;
        let true_low = low[i].min(close[i - 1]);
        let bp = close[i] - true_low;
        let tr = high[i].max(close[i - 1]) - true_low;
        bp1 += bp - bp_ring[t1];
        tr1 += tr - tr_ring[t1];
        bp2 += bp - bp_ring[t2];
        tr2 += tr - tr_ring[t2];
        bp3 += bp - bp_ring[t3];
        tr3 += tr - tr_ring[t3];
        bp_ring[write] = bp;
        tr_ring[write] = tr;
        if bp == 0.0 && tr == 0.0 {
            null_run += 1;
        } else {
            null_run = 0;
        }
        if null_run >= shortest {
            if null_run >= P1 {
                bp1 = 0.0;
                tr1 = 0.0;
            }
            if null_run >= P2 {
                bp2 = 0.0;
                tr2 = 0.0;
            }
            if null_run >= P3 {
                bp3 = 0.0;
                tr3 = 0.0;
            }
        }
    }
    for i in warm_end..len {
        let write = i & mask;
        let t1 = i.wrapping_sub(P1) & mask;
        let t2 = i.wrapping_sub(P2) & mask;
        let t3 = i.wrapping_sub(P3) & mask;
        let true_low = low[i].min(close[i - 1]);
        let bp = close[i] - true_low;
        let tr = high[i].max(close[i - 1]) - true_low;
        bp1 += bp - bp_ring[t1];
        tr1 += tr - tr_ring[t1];
        bp2 += bp - bp_ring[t2];
        tr2 += tr - tr_ring[t2];
        bp3 += bp - bp_ring[t3];
        tr3 += tr - tr_ring[t3];
        bp_ring[write] = bp;
        tr_ring[write] = tr;
        if bp == 0.0 && tr == 0.0 {
            null_run += 1;
        } else {
            null_run = 0;
        }
        if null_run >= shortest {
            if null_run >= P1 {
                bp1 = 0.0;
                tr1 = 0.0;
            }
            if null_run >= P2 {
                bp2 = 0.0;
                tr2 = 0.0;
            }
            if null_run >= P3 {
                bp3 = 0.0;
                tr3 = 0.0;
            }
        }
        out[i] = dx_like_guards(bp1, tr1, bp2, tr2, bp3, tr3);
    }
}

/// Diagnostic: `split` with the divides removed entirely.
fn split_nodiv(high: &[f64], low: &[f64], close: &[f64], out: &mut [f64]) {
    let max_period = P1.max(P2).max(P3);
    let ring = (max_period + 1).next_power_of_two();
    let mask = ring - 1;
    let mut bp_ring = vec![0.0f64; ring];
    let mut tr_ring = vec![0.0f64; ring];
    let mut bp1 = 0.0;
    let mut tr1 = 0.0;
    let mut bp2 = 0.0;
    let mut tr2 = 0.0;
    let mut bp3 = 0.0;
    let mut tr3 = 0.0;
    let mut null_run = 0usize;
    let shortest = P1.min(P2).min(P3);
    let len = high.len();
    let warm_end = max_period.min(len);
    for i in 1..warm_end {
        let write = i & mask;
        let t1 = i.wrapping_sub(P1) & mask;
        let t2 = i.wrapping_sub(P2) & mask;
        let t3 = i.wrapping_sub(P3) & mask;
        let true_low = low[i].min(close[i - 1]);
        let bp = close[i] - true_low;
        let tr = high[i].max(close[i - 1]) - true_low;
        bp1 += bp - bp_ring[t1];
        tr1 += tr - tr_ring[t1];
        bp2 += bp - bp_ring[t2];
        tr2 += tr - tr_ring[t2];
        bp3 += bp - bp_ring[t3];
        tr3 += tr - tr_ring[t3];
        bp_ring[write] = bp;
        tr_ring[write] = tr;
        if bp == 0.0 && tr == 0.0 {
            null_run += 1;
        } else {
            null_run = 0;
        }
        if null_run >= shortest {
            if null_run >= P1 {
                bp1 = 0.0;
                tr1 = 0.0;
            }
            if null_run >= P2 {
                bp2 = 0.0;
                tr2 = 0.0;
            }
            if null_run >= P3 {
                bp3 = 0.0;
                tr3 = 0.0;
            }
        }
    }
    for i in warm_end..len {
        let write = i & mask;
        let t1 = i.wrapping_sub(P1) & mask;
        let t2 = i.wrapping_sub(P2) & mask;
        let t3 = i.wrapping_sub(P3) & mask;
        let true_low = low[i].min(close[i - 1]);
        let bp = close[i] - true_low;
        let tr = high[i].max(close[i - 1]) - true_low;
        bp1 += bp - bp_ring[t1];
        tr1 += tr - tr_ring[t1];
        bp2 += bp - bp_ring[t2];
        tr2 += tr - tr_ring[t2];
        bp3 += bp - bp_ring[t3];
        tr3 += tr - tr_ring[t3];
        bp_ring[write] = bp;
        tr_ring[write] = tr;
        if bp == 0.0 && tr == 0.0 {
            null_run += 1;
        } else {
            null_run = 0;
        }
        if null_run >= shortest {
            if null_run >= P1 {
                bp1 = 0.0;
                tr1 = 0.0;
            }
            if null_run >= P2 {
                bp2 = 0.0;
                tr2 = 0.0;
            }
            if null_run >= P3 {
                bp3 = 0.0;
                tr3 = 0.0;
            }
        }
        out[i] = bp1 + bp2 + bp3 + tr1 + tr2 + tr3;
    }
}

/// Diagnostic: `split` with the final `/ 7.0` as a reciprocal multiply.
fn split_recip(high: &[f64], low: &[f64], close: &[f64], out: &mut [f64]) {
    let max_period = P1.max(P2).max(P3);
    let ring = (max_period + 1).next_power_of_two();
    let mask = ring - 1;
    let mut bp_ring = vec![0.0f64; ring];
    let mut tr_ring = vec![0.0f64; ring];
    let mut bp1 = 0.0;
    let mut tr1 = 0.0;
    let mut bp2 = 0.0;
    let mut tr2 = 0.0;
    let mut bp3 = 0.0;
    let mut tr3 = 0.0;
    let mut null_run = 0usize;
    let shortest = P1.min(P2).min(P3);
    let len = high.len();
    let warm_end = max_period.min(len);
    for i in 1..warm_end {
        let write = i & mask;
        let t1 = i.wrapping_sub(P1) & mask;
        let t2 = i.wrapping_sub(P2) & mask;
        let t3 = i.wrapping_sub(P3) & mask;
        let true_low = low[i].min(close[i - 1]);
        let bp = close[i] - true_low;
        let tr = high[i].max(close[i - 1]) - true_low;
        bp1 += bp - bp_ring[t1];
        tr1 += tr - tr_ring[t1];
        bp2 += bp - bp_ring[t2];
        tr2 += tr - tr_ring[t2];
        bp3 += bp - bp_ring[t3];
        tr3 += tr - tr_ring[t3];
        bp_ring[write] = bp;
        tr_ring[write] = tr;
        if bp == 0.0 && tr == 0.0 {
            null_run += 1;
        } else {
            null_run = 0;
        }
        if null_run >= shortest {
            if null_run >= P1 {
                bp1 = 0.0;
                tr1 = 0.0;
            }
            if null_run >= P2 {
                bp2 = 0.0;
                tr2 = 0.0;
            }
            if null_run >= P3 {
                bp3 = 0.0;
                tr3 = 0.0;
            }
        }
    }
    for i in warm_end..len {
        let write = i & mask;
        let t1 = i.wrapping_sub(P1) & mask;
        let t2 = i.wrapping_sub(P2) & mask;
        let t3 = i.wrapping_sub(P3) & mask;
        let true_low = low[i].min(close[i - 1]);
        let bp = close[i] - true_low;
        let tr = high[i].max(close[i - 1]) - true_low;
        bp1 += bp - bp_ring[t1];
        tr1 += tr - tr_ring[t1];
        bp2 += bp - bp_ring[t2];
        tr2 += tr - tr_ring[t2];
        bp3 += bp - bp_ring[t3];
        tr3 += tr - tr_ring[t3];
        bp_ring[write] = bp;
        tr_ring[write] = tr;
        if bp == 0.0 && tr == 0.0 {
            null_run += 1;
        } else {
            null_run = 0;
        }
        if null_run >= shortest {
            if null_run >= P1 {
                bp1 = 0.0;
                tr1 = 0.0;
            }
            if null_run >= P2 {
                bp2 = 0.0;
                tr2 = 0.0;
            }
            if null_run >= P3 {
                bp3 = 0.0;
                tr3 = 0.0;
            }
        }
        let avg1 = if tr1 > 0.0 { bp1 / tr1 } else { 0.0 };
        let avg2 = if tr2 > 0.0 { bp2 / tr2 } else { 0.0 };
        let avg3 = if tr3 > 0.0 { bp3 / tr3 } else { 0.0 };
        out[i] = 100.0 * (4.0 * avg1 + 2.0 * avg2 + avg3) * (1.0 / 7.0);
    }
}

/// Bit-parity check against the shipped shape.
fn parity(a: &[f64], b: &[f64], label: &str) {
    let mut diff = 0usize;
    let mut max_abs = 0.0f64;
    let mut max_rel = 0.0f64;
    for (x, y) in a.iter().zip(b.iter()) {
        if x.to_bits() != y.to_bits() {
            diff += 1;
        }
        if x.is_finite() && y.is_finite() {
            let d = (x - y).abs();
            if d > max_abs {
                max_abs = d;
            }
            let scale = x.abs().max(y.abs());
            if scale > 0.0 {
                let r = d / scale;
                if r > max_rel {
                    max_rel = r;
                }
            }
        }
    }
    println!(
        "  parity {label}: {diff} / {} bit-differs; max_abs {max_abs:.3e}; max_rel {max_rel:.3e}",
        a.len()
    );
}

fn main() {
    let (high, low, close) = bars(DATA_LEN);
    let mut a = vec![f64::NAN; DATA_LEN];
    let mut b = vec![f64::NAN; DATA_LEN];
    let mut c = vec![f64::NAN; DATA_LEN];
    let mut d = vec![f64::NAN; DATA_LEN];

    println!("ULTOSC kernel probe, {DATA_LEN} bars, p=({P1},{P2},{P3})");
    println!("  (base = shipped shape; negative % = candidate faster)\n");

    duel(
        "split",
        || shipped(&high, &low, &close, &mut a),
        || split(&high, &low, &close, &mut b),
    );
    duel(
        "split_nodiv",
        || shipped(&high, &low, &close, &mut a),
        || split_nodiv(&high, &low, &close, &mut c),
    );
    duel(
        "split_recip",
        || shipped(&high, &low, &close, &mut a),
        || split_recip(&high, &low, &close, &mut d),
    );

    println!();
    split(&high, &low, &close, &mut b);
    shipped(&high, &low, &close, &mut a);
    parity(&a, &b, "split vs shipped");
    split_recip(&high, &low, &close, &mut d);
    parity(&a, &d, "split_recip vs shipped");
    let sum = |v: &[f64]| v.iter().filter(|x| x.is_finite()).sum::<f64>();
    println!(
        "  sum check: shipped {:.6} split {:.6} nodiv {:.6} recip {:.6}",
        sum(&a),
        sum(&b),
        sum(&c),
        sum(&d)
    );
}
