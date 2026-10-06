//! Kernel A/B probe: the *previous* and *current* body of a rewritten kernel,
//! side by side in one binary.
//!
//! Both bodies run interleaved in the same process, so they see the same
//! thermal state, the same allocator and the same page cache. Sequential runs
//! of two separate Criterion invocations cannot do that: this machine moves a
//! single indicator's measured cost by up to 40% between processes (see
//! `docs/BENCHMARK_REPORT.md` and `docs/talib-efficiency-deep-dive-zh.md` §1),
//! and that is larger than several of the differences worth chasing.
//!
//! This is the evidence behind every "faster"/"slower" verdict in
//! `docs/talib-efficiency-deep-dive-zh.md` §4 and §6. Reproduce with:
//!
//! ```text
//! cargo run --release -p finkit --example kernel_hotspot_probe
//! ```
//!
//! ## Why "previous" is a *port* and not a call
//!
//! Each `*_old` function below is the body that shipped before the rewrite,
//! transcribed so it can live in the same binary as its replacement. Both
//! sides therefore perform the same allocation the shipped function performs.
//! An earlier revision of this probe benched the old body into a
//! *preallocated* buffer while benching the new one through the allocating
//! public function; that charged one 80 KB malloc/free pair to the new code and
//! made three micro-rewrites look like regressions. The trap is worth keeping
//! visible, because it produced exactly the kind of confident, wrong conclusion
//! this whole document set exists to avoid.
//!
//! ## The null controls
//!
//! `AVGDEV` and `LINREG_SLOPE` are *reverted* — the shipped body is the one the
//! probe compares against, so both sides are the same algorithm. Their ratios
//! are therefore this probe's own noise floor, and they bracket what counts as
//! a real effect: 0.995x in the best case, 1.116x in the worst. Any smaller
//! difference is not evidence of anything.

use std::hint::black_box;
use std::time::Instant;

use finkit::indicators;

const DATA_LEN: usize = 10_000;
const REPS: usize = 150;

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

fn bench_min<T>(reps: usize, mut f: impl FnMut() -> T) -> f64 {
    for _ in 0..(reps / 4).max(2) {
        black_box(f());
    }
    let mut best = f64::INFINITY;
    for _ in 0..reps {
        let start = Instant::now();
        black_box(f());
        let elapsed = start.elapsed().as_secs_f64() * 1e6;
        if elapsed < best {
            best = elapsed;
        }
    }
    best
}

fn head(title: &str) {
    println!("\n{title}");
}

fn row(label: &str, us: f64) {
    println!(
        "  {:<38} {:>9.2} us   {:>7.3} ns/bar",
        label,
        us,
        us * 1000.0 / DATA_LEN as f64
    );
}

fn verdict(label: &str, old: f64, new: f64) {
    println!(
        "    -> {:<30} {:>6.3}x {}",
        label,
        old / new,
        if new < old { "faster" } else { "SLOWER" }
    );
}

fn mismatches(a: &[f64], b: &[f64], tol: f64) -> usize {
    a.iter()
        .zip(b)
        .filter(|(x, y)| {
            let (x, y) = (**x, **y);
            !(x.is_nan() && y.is_nan()) && (x - y).abs() > tol
        })
        .count()
}

// ===========================================================================
// PERCENTRANK — previous body: sorted window + branchless binary search +
// `copy_within`, allocating its output the way the shipped function did.
// ===========================================================================

fn lower_bound(sorted: &[f64], value: f64) -> usize {
    let mut base = 0usize;
    let mut remaining = sorted.len();
    while remaining > 1 {
        let half = remaining / 2;
        base += if sorted[base + half - 1] < value {
            half
        } else {
            0
        };
        remaining -= half;
    }
    base + usize::from(!sorted.is_empty() && sorted[base] < value)
}

fn percent_rank_old(input: &[f64], timeperiod: usize) -> Vec<f64> {
    let len = input.len();
    let mut output = vec![f64::NAN; len];
    let mut sorted: Vec<f64> = input[..timeperiod].to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    for i in timeperiod..len {
        let current = input[i];
        let count_less = lower_bound(&sorted, current);
        output[i] = (count_less as f64 / timeperiod as f64) * 100.0;
        let evicted = input[i - timeperiod];
        let evicted_pos = lower_bound(&sorted, evicted);
        let insert_pos = count_less - usize::from(evicted < current);
        if evicted_pos <= insert_pos {
            sorted.copy_within(evicted_pos + 1..=insert_pos, evicted_pos);
        } else {
            sorted.copy_within(insert_pos..evicted_pos, insert_pos + 1);
        }
        sorted[insert_pos] = current;
    }
    output
}

// ===========================================================================
// AVGDEV — NULL CONTROL. The shipped body was restored, so this is the same
// algorithm on both sides and the ratio is pure measurement noise.
// ===========================================================================

fn avgdev_old(input: &[f64], timeperiod: usize) -> Vec<f64> {
    let len = input.len();
    let mut output = vec![f64::NAN; len];
    let n = timeperiod as f64;
    let inv_n = 1.0 / n;
    let mut buf: Vec<f64> = input[..timeperiod].to_vec();
    let mut sum: f64 = buf.iter().sum();
    let dev_sum = |buf: &[f64], mean: f64| -> f64 { buf.iter().map(|&x| (x - mean).abs()).sum() };
    let mean = sum * inv_n;
    output[timeperiod - 1] = dev_sum(&buf, mean) * inv_n;
    for i in timeperiod..len {
        let newest = input[i];
        sum += newest - input[i - timeperiod];
        buf.remove(0);
        buf.push(newest);
        let mean = sum * inv_n;
        output[i] = dev_sum(&buf, mean) * inv_n;
    }
    output
}

// ===========================================================================
// LINREG_SLOPE — NULL CONTROL (also restored).
// ===========================================================================

fn linreg_slope_old(input: &[f64], period: usize) -> Vec<f64> {
    let len = input.len();
    let mut output = vec![f64::NAN; len];
    let p = period as f64;
    let sum_x = p * (p - 1.0) / 2.0;
    let sum_x2 = p * (p - 1.0) * (2.0 * p - 1.0) / 6.0;
    let inv_denom = 1.0 / (p * sum_x2 - sum_x * sum_x);
    let mut sum_y: f64 = input[..period].iter().sum();
    let mut sum_xy = 0.0f64;
    for (j, &v) in input[..period].iter().enumerate() {
        sum_xy += j as f64 * v;
    }
    output[period - 1] = (p * sum_xy - sum_x * sum_y) * inv_denom;
    for i in period..len {
        let old_val = input[i - period];
        let new_val = input[i];
        sum_xy += (period - 1) as f64 * new_val - (sum_y - old_val);
        sum_y += new_val - old_val;
        output[i] = (p * sum_xy - sum_x * sum_y) * inv_denom;
    }
    output
}

// ===========================================================================
// AROON — previous body at HEAD: two independent pointer-based cached-index
// scans (the function was called `aroon_with_deques`, but the body is the
// pointer scan), allocating both columns.
// ===========================================================================

fn aroon_old_into(
    high: &[f64],
    low: &[f64],
    period: usize,
    up_out: &mut [f64],
    dn_out: &mut [f64],
) {
    let len = high.len();
    up_out[..period].fill(f64::NAN);
    dn_out[..period].fill(f64::NAN);
    let inv_period = 100.0 / period as f64;
    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();
    unsafe {
        let mut highest_idx = 0usize;
        let mut lowest_idx = 0usize;
        let mut highest = *high_ptr;
        let mut lowest = *low_ptr;
        for k in 1..=period {
            let h = *high_ptr.add(k);
            let l = *low_ptr.add(k);
            if h >= highest {
                highest = h;
                highest_idx = k;
            }
            if l <= lowest {
                lowest = l;
                lowest_idx = k;
            }
        }
        *up_out.get_unchecked_mut(period) = highest_idx as f64 * inv_period;
        *dn_out.get_unchecked_mut(period) = lowest_idx as f64 * inv_period;
        for i in (period + 1)..len {
            let ws = i - period;
            let new_h = *high_ptr.add(i);
            let new_l = *low_ptr.add(i);
            if highest_idx < ws {
                highest = *high_ptr.add(ws);
                highest_idx = ws;
                let mut k = ws + 1;
                while k <= i {
                    let h = *high_ptr.add(k);
                    if h >= highest {
                        highest = h;
                        highest_idx = k;
                    }
                    k += 1;
                }
            } else if new_h >= highest {
                highest = new_h;
                highest_idx = i;
            }
            if lowest_idx < ws {
                lowest = *low_ptr.add(ws);
                lowest_idx = ws;
                let mut k = ws + 1;
                while k <= i {
                    let l = *low_ptr.add(k);
                    if l <= lowest {
                        lowest = l;
                        lowest_idx = k;
                    }
                    k += 1;
                }
            } else if new_l <= lowest {
                lowest = new_l;
                lowest_idx = i;
            }
            *up_out.get_unchecked_mut(i) = (period - (i - highest_idx)) as f64 * inv_period;
            *dn_out.get_unchecked_mut(i) = (period - (i - lowest_idx)) as f64 * inv_period;
        }
    }
}

fn aroon_old(high: &[f64], low: &[f64], period: usize) -> (Vec<f64>, Vec<f64>) {
    let len = high.len();
    let mut up_out = vec![f64::NAN; len];
    let mut dn_out = vec![f64::NAN; len];
    aroon_old_into(high, low, period, &mut up_out, &mut dn_out);
    (up_out, dn_out)
}

// ===========================================================================
// AROONOSC — previous body at HEAD: both legs in one fused pointer loop.
// ===========================================================================

fn aroonosc_old(high: &[f64], low: &[f64], period: usize) -> Vec<f64> {
    let len = high.len();
    let mut output = vec![f64::NAN; len];
    let inv_period = 100.0 / period as f64;
    let window = period + 1;
    let mut highest = f64::NEG_INFINITY;
    let mut highest_idx = 0usize;
    let mut has_high = false;
    let mut lowest = f64::INFINITY;
    let mut lowest_idx = 0usize;
    let mut has_low = false;
    for i in 0..len {
        let new_high = high[i];
        let new_low = low[i];
        if new_high >= highest {
            highest = new_high;
            highest_idx = i;
            has_high = true;
        } else if has_high && highest_idx + window <= i {
            let mut best = f64::NEG_INFINITY;
            let mut best_index = i + 1 - window;
            let mut found = false;
            for index in (i + 1 - window)..=i {
                if high[index] >= best {
                    best = high[index];
                    best_index = index;
                    found = true;
                }
            }
            highest = best;
            highest_idx = best_index;
            has_high = found;
        }
        if new_low <= lowest {
            lowest = new_low;
            lowest_idx = i;
            has_low = true;
        } else if has_low && lowest_idx + window <= i {
            let mut best = f64::INFINITY;
            let mut best_index = i + 1 - window;
            let mut found = false;
            for index in (i + 1 - window)..=i {
                if low[index] <= best {
                    best = low[index];
                    best_index = index;
                    found = true;
                }
            }
            lowest = best;
            lowest_idx = best_index;
            has_low = found;
        }
        if i + 1 >= window {
            output[i] = if has_high && has_low {
                (highest_idx as f64 - lowest_idx as f64) * inv_period
            } else {
                f64::NAN
            };
        }
    }
    output
}

// ===========================================================================
// LINREG_SLOPE loop body — the only question worth asking about the one ❌ in
// the report. Identical seeding, identical allocation; the *only* difference
// is indexed access vs pointer cursors. `linreg_slope_14` is 0.76x against
// TA-Lib, and V4 §42.8 attributes the gap to exactly this.
// ===========================================================================

fn linreg_body_indexed(input: &[f64], period: usize, output: &mut [f64]) {
    let len = input.len();
    let p = period as f64;
    let sum_x = p * (p - 1.0) / 2.0;
    let sum_x2 = p * (p - 1.0) * (2.0 * p - 1.0) / 6.0;
    let inv_denom = 1.0 / (p * sum_x2 - sum_x * sum_x);
    let mut sum_y: f64 = input[..period].iter().sum();
    let mut sum_xy = 0.0f64;
    for (j, &v) in input[..period].iter().enumerate() {
        sum_xy += j as f64 * v;
    }
    output[period - 1] = (p * sum_xy - sum_x * sum_y) * inv_denom;
    for i in period..len {
        let old_val = input[i - period];
        let new_val = input[i];
        sum_xy += (period - 1) as f64 * new_val - (sum_y - old_val);
        sum_y += new_val - old_val;
        output[i] = (p * sum_xy - sum_x * sum_y) * inv_denom;
    }
}

fn linreg_body_cursors(input: &[f64], period: usize, output: &mut [f64]) {
    let len = input.len();
    let p = period as f64;
    let sum_x = p * (p - 1.0) / 2.0;
    let sum_x2 = p * (p - 1.0) * (2.0 * p - 1.0) / 6.0;
    let inv_denom = 1.0 / (p * sum_x2 - sum_x * sum_x);
    let mut sum_y: f64 = input[..period].iter().sum();
    let mut sum_xy = 0.0f64;
    for (j, &v) in input[..period].iter().enumerate() {
        sum_xy += j as f64 * v;
    }
    output[period - 1] = (p * sum_xy - sum_x * sum_y) * inv_denom;
    let last_x = (period - 1) as f64;
    unsafe {
        let mut new = input.as_ptr().add(period);
        let mut old = input.as_ptr();
        let mut out = output.as_mut_ptr().add(period);
        let end = input.as_ptr().add(len);
        while new < end {
            let new_val = *new;
            let old_val = *old;
            sum_xy += last_x * new_val - (sum_y - old_val);
            sum_y += new_val - old_val;
            *out = (p * sum_xy - sum_x * sum_y) * inv_denom;
            new = new.add(1);
            old = old.add(1);
            out = out.add(1);
        }
    }
}

fn main() {
    let (_open, high, low, close, _volume) = create_ohlcv_data(DATA_LEN);
    println!("kernel A/B probe — {DATA_LEN} bars, best of {REPS} runs, both bodies in-process");
    println!("both sides allocate their output; AVGDEV/LINREG are null controls");

    // ---- PERCENTRANK ------------------------------------------------------
    head("PERCENTRANK_30");
    let mut pr_old_out = vec![f64::NAN; DATA_LEN];
    let pr_old = bench_min(REPS, || {
        let v = percent_rank_old(&close, 30);
        black_box(&v);
        v
    });
    let pr_new = bench_min(REPS, || indicators::percent_rank(&close, 30).unwrap());
    pr_old_out.copy_from_slice(percent_rank_old(&close, 30).as_slice());
    row("previous: sorted window + memmove", pr_old);
    row("current:  avx2 direct count", pr_new);
    verdict("percent_rank_30", pr_old, pr_new);
    println!(
        "  bit-identical slots: {}/{}",
        DATA_LEN
            - mismatches(
                &pr_old_out,
                indicators::percent_rank(&close, 30)
                    .unwrap()
                    .as_slice()
                    .unwrap(),
                0.0
            ),
        DATA_LEN
    );

    // ---- AVGDEV (null control) -------------------------------------------
    head("AVGDEV_14  [null control — reverted, both sides identical]");
    let mut ad_old_out = vec![f64::NAN; DATA_LEN];
    let ad_old = bench_min(REPS, || {
        let v = avgdev_old(&close, 14);
        black_box(&v);
        v
    });
    let ad_new = bench_min(REPS, || indicators::avgdev(&close, 14).unwrap());
    ad_old_out.copy_from_slice(avgdev_old(&close, 14).as_slice());
    row("control: shipped body, copied", ad_old);
    row("shipped: indicators::avgdev", ad_new);
    verdict("avgdev_14 (expect ~1.0)", ad_old, ad_new);
    println!(
        "  mismatches @1e-10: {}",
        mismatches(
            &ad_old_out,
            indicators::avgdev(&close, 14).unwrap().as_slice().unwrap(),
            1e-10
        )
    );

    // ---- LINREG_SLOPE (null control) -------------------------------------
    head("LINREG_SLOPE_14  [null control — reverted, both sides identical]");
    let mut ls_old_out = vec![f64::NAN; DATA_LEN];
    let ls_old = bench_min(REPS, || {
        let v = linreg_slope_old(&close, 14);
        black_box(&v);
        v
    });
    let ls_new = bench_min(REPS, || {
        finkit::math::linear::linreg_slope(&close, 14).unwrap()
    });
    ls_old_out.copy_from_slice(linreg_slope_old(&close, 14).as_slice());
    row("control: shipped body, copied", ls_old);
    row("shipped: linear::linreg_slope", ls_new);
    verdict("linreg_slope_14 (expect ~1.0)", ls_old, ls_new);
    println!(
        "  mismatches @1e-12: {}",
        mismatches(
            &ls_old_out,
            finkit::math::linear::linreg_slope(&close, 14)
                .unwrap()
                .as_slice()
                .unwrap(),
            1e-12
        )
    );

    // ---- LINREG_SLOPE loop body ------------------------------------------
    head("LINREG_SLOPE_14  [loop body only — same seeding, same allocation]");
    let mut lb_indexed = vec![f64::NAN; DATA_LEN];
    let mut lb_cursors = vec![f64::NAN; DATA_LEN];
    let lb_idx = bench_min(REPS, || {
        linreg_body_indexed(&close, 14, &mut lb_indexed);
    });
    let lb_cur = bench_min(REPS, || {
        linreg_body_cursors(&close, 14, &mut lb_cursors);
    });
    row("indexed  (shipped body)", lb_idx);
    row("pointer cursors (candidate)", lb_cur);
    verdict("linreg body", lb_idx, lb_cur);
    println!(
        "  mismatches @1e-12: {}",
        mismatches(&lb_indexed, &lb_cursors, 1e-12)
    );

    // ---- AROON ------------------------------------------------------------
    head("AROON_14");
    let ar_old = bench_min(REPS, || aroon_old(&high, &low, 14));
    let ar_new = bench_min(REPS, || indicators::aroon(&high, &low, 14).unwrap());
    row("previous: two pointer scans", ar_old);
    row("current:  fused ExtremeTracker", ar_new);
    verdict("aroon_14 (allocating)", ar_old, ar_new);

    // Kernel-level: same pair with output storage handed in, so no allocator
    // noise at all. This is the path FFI and the formula engine actually walk.
    let mut up_old = vec![f64::NAN; DATA_LEN];
    let mut dn_old = vec![f64::NAN; DATA_LEN];
    let mut up_new = vec![f64::NAN; DATA_LEN];
    let mut dn_new = vec![f64::NAN; DATA_LEN];
    let ar_old_into = bench_min(REPS, || {
        aroon_old_into(&high, &low, 14, &mut up_old, &mut dn_old);
    });
    let ar_new_into = bench_min(REPS, || {
        indicators::aroon_into(&high, &low, 14, &mut up_new, &mut dn_new).unwrap();
    });
    row("previous: aroon_into (caller buffer)", ar_old_into);
    row("current:  aroon_into (caller buffer)", ar_new_into);
    verdict("aroon_into_14 (no alloc)", ar_old_into, ar_new_into);
    let shipped = indicators::aroon(&high, &low, 14).unwrap();
    println!(
        "  aroon_up   mismatches @1e-12: {}",
        mismatches(shipped.aroon_up.as_slice().unwrap(), &up_old, 1e-12)
    );
    println!(
        "  aroon_down mismatches @1e-12: {}",
        mismatches(shipped.aroon_down.as_slice().unwrap(), &dn_old, 1e-12)
    );

    // ---- AROONOSC ---------------------------------------------------------
    head("AROONOSC_14");
    let ao_old = bench_min(REPS, || aroonosc_old(&high, &low, 14));
    let ao_new = bench_min(REPS, || indicators::aroonosc(&high, &low, 14).unwrap());
    row("previous: fused pointer loop", ao_old);
    row("current:  fused ExtremeTracker", ao_new);
    verdict("aroonosc_14", ao_old, ao_new);
    println!(
        "  aroonosc mismatches @1e-12: {}",
        mismatches(
            indicators::aroonosc(&high, &low, 14)
                .unwrap()
                .as_slice()
                .unwrap(),
            &aroonosc_old(&high, &low, 14),
            1e-12
        )
    );
}
