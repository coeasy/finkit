use finkit::indicators::chart::HeikinAshiResult;
use ndarray::Array1;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn push_reference(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> HeikinAshiResult {
    let len = open.len();
    let mut ha_open = Vec::with_capacity(len);
    let mut ha_high = Vec::with_capacity(len);
    let mut ha_low = Vec::with_capacity(len);
    let mut ha_close = Vec::with_capacity(len);
    let mut previous_open = f64::NAN;
    let mut previous_close = f64::NAN;
    for index in 0..len {
        let o = open[index];
        let h = high[index];
        let l = low[index];
        let c = close[index];
        if o.is_nan() || h.is_nan() || l.is_nan() || c.is_nan() {
            previous_open = f64::NAN;
            previous_close = f64::NAN;
            ha_open.push(f64::NAN);
            ha_high.push(f64::NAN);
            ha_low.push(f64::NAN);
            ha_close.push(f64::NAN);
            continue;
        }
        let current_close = (o + h + l + c) * 0.25;
        let current_open = if previous_open.is_nan() {
            (o + c) * 0.5
        } else {
            (previous_open + previous_close) * 0.5
        };
        ha_close.push(current_close);
        ha_open.push(current_open);
        ha_high.push(h.max(current_open).max(current_close));
        ha_low.push(l.min(current_open).min(current_close));
        previous_open = current_open;
        previous_close = current_close;
    }
    HeikinAshiResult {
        ha_open: Array1::from_vec(ha_open),
        ha_high: Array1::from_vec(ha_high),
        ha_low: Array1::from_vec(ha_low),
        ha_close: Array1::from_vec(ha_close),
    }
}

fn data(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    for i in 0..len {
        let x = i as f64;
        let c = 100.0 + 0.006 * x + (x * 0.017).sin() * 2.1;
        let o = c - 0.15 + (x * 0.031).sin() * 0.05;
        open.push(o);
        high.push(o.max(c) + 0.7 + (x * 0.023).sin().abs() * 0.3);
        low.push(c.min(o) - 0.8 - (x * 0.019).cos().abs() * 0.2);
        close.push(c);
    }
    (open, high, low, close)
}

fn indexed(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> HeikinAshiResult {
    let len = open.len();
    let mut ha_open = Vec::with_capacity(len);
    let mut ha_high = Vec::with_capacity(len);
    let mut ha_low = Vec::with_capacity(len);
    let mut ha_close = Vec::with_capacity(len);
    // SAFETY: every output slot is written in the loop before conversion.
    unsafe {
        ha_open.set_len(len);
        ha_high.set_len(len);
        ha_low.set_len(len);
        ha_close.set_len(len);
    }

    let mut previous_open = f64::NAN;
    let mut previous_close = f64::NAN;
    for index in 0..len {
        let o = open[index];
        let h = high[index];
        let l = low[index];
        let c = close[index];
        if o.is_nan() || h.is_nan() || l.is_nan() || c.is_nan() {
            previous_open = f64::NAN;
            previous_close = f64::NAN;
            ha_open[index] = f64::NAN;
            ha_high[index] = f64::NAN;
            ha_low[index] = f64::NAN;
            ha_close[index] = f64::NAN;
            continue;
        }
        let current_close = (o + h + l + c) * 0.25;
        let current_open = if previous_open.is_nan() {
            (o + c) * 0.5
        } else {
            (previous_open + previous_close) * 0.5
        };
        ha_close[index] = current_close;
        ha_open[index] = current_open;
        ha_high[index] = h.max(current_open).max(current_close);
        ha_low[index] = l.min(current_open).min(current_close);
        previous_open = current_open;
        previous_close = current_close;
    }
    HeikinAshiResult {
        ha_open: Array1::from_vec(ha_open),
        ha_high: Array1::from_vec(ha_high),
        ha_low: Array1::from_vec(ha_low),
        ha_close: Array1::from_vec(ha_close),
    }
}

fn iterated(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> HeikinAshiResult {
    let len = open.len();
    let mut ha_open = Vec::with_capacity(len);
    let mut ha_high = Vec::with_capacity(len);
    let mut ha_low = Vec::with_capacity(len);
    let mut ha_close = Vec::with_capacity(len);
    // SAFETY: every output slot is written in the zipped loop.
    unsafe {
        ha_open.set_len(len);
        ha_high.set_len(len);
        ha_low.set_len(len);
        ha_close.set_len(len);
    }
    let mut previous_open = f64::NAN;
    let mut previous_close = f64::NAN;
    for ((((o, h), l), c), (((out_open, out_high), out_low), out_close)) in
        open.iter().zip(high).zip(low).zip(close).zip(
            ha_open
                .iter_mut()
                .zip(ha_high.iter_mut())
                .zip(ha_low.iter_mut())
                .zip(ha_close.iter_mut()),
        )
    {
        let (o, h, l, c) = (*o, *h, *l, *c);
        if o.is_nan() || h.is_nan() || l.is_nan() || c.is_nan() {
            previous_open = f64::NAN;
            previous_close = f64::NAN;
            *out_open = f64::NAN;
            *out_high = f64::NAN;
            *out_low = f64::NAN;
            *out_close = f64::NAN;
            continue;
        }
        let current_close = (o + h + l + c) * 0.25;
        let current_open = if previous_open.is_nan() {
            (o + c) * 0.5
        } else {
            (previous_open + previous_close) * 0.5
        };
        *out_close = current_close;
        *out_open = current_open;
        *out_high = h.max(current_open).max(current_close);
        *out_low = l.min(current_open).min(current_close);
        previous_open = current_open;
        previous_close = current_close;
    }
    HeikinAshiResult {
        ha_open: Array1::from_vec(ha_open),
        ha_high: Array1::from_vec(ha_high),
        ha_low: Array1::from_vec(ha_low),
        ha_close: Array1::from_vec(ha_close),
    }
}

fn pointer_path(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> HeikinAshiResult {
    let len = open.len();
    let mut ha_open = Vec::<f64>::with_capacity(len);
    let mut ha_high = Vec::<f64>::with_capacity(len);
    let mut ha_low = Vec::<f64>::with_capacity(len);
    let mut ha_close = Vec::<f64>::with_capacity(len);
    // SAFETY: each output slot is assigned before the vectors are converted.
    unsafe {
        ha_open.set_len(len);
        ha_high.set_len(len);
        ha_low.set_len(len);
        ha_close.set_len(len);
    }
    let (open_ptr, high_ptr, low_ptr, close_ptr) =
        (open.as_ptr(), high.as_ptr(), low.as_ptr(), close.as_ptr());
    let (out_open, out_high, out_low, out_close) = (
        ha_open.as_mut_ptr(),
        ha_high.as_mut_ptr(),
        ha_low.as_mut_ptr(),
        ha_close.as_mut_ptr(),
    );
    let mut previous_open = f64::NAN;
    let mut previous_close = f64::NAN;
    for index in 0..len {
        // SAFETY: all input and output slices have the same length in this probe.
        let (o, h, l, c) = unsafe {
            (
                *open_ptr.add(index),
                *high_ptr.add(index),
                *low_ptr.add(index),
                *close_ptr.add(index),
            )
        };
        if o.is_nan() || h.is_nan() || l.is_nan() || c.is_nan() {
            previous_open = f64::NAN;
            previous_close = f64::NAN;
            // SAFETY: the output pointers were allocated for `len` elements.
            unsafe {
                out_open.add(index).write(f64::NAN);
                out_high.add(index).write(f64::NAN);
                out_low.add(index).write(f64::NAN);
                out_close.add(index).write(f64::NAN);
            }
            continue;
        }
        let current_close = (o + h + l + c) * 0.25;
        let current_open = if previous_open.is_nan() {
            (o + c) * 0.5
        } else {
            (previous_open + previous_close) * 0.5
        };
        // SAFETY: the output pointers were allocated for `len` elements.
        unsafe {
            out_open.add(index).write(current_open);
            out_high
                .add(index)
                .write(h.max(current_open).max(current_close));
            out_low
                .add(index)
                .write(l.min(current_open).min(current_close));
            out_close.add(index).write(current_close);
        }
        previous_open = current_open;
        previous_close = current_close;
    }
    HeikinAshiResult {
        ha_open: Array1::from_vec(ha_open),
        ha_high: Array1::from_vec(ha_high),
        ha_low: Array1::from_vec(ha_low),
        ha_close: Array1::from_vec(ha_close),
    }
}

fn time(mut f: impl FnMut() -> HeikinAshiResult, duration: Duration) -> f64 {
    let started = Instant::now();
    let mut calls = 0u64;
    while started.elapsed() < duration {
        black_box(f());
        calls += 1;
    }
    started.elapsed().as_secs_f64() * 1e6 / calls as f64
}

fn compare(left: &HeikinAshiResult, right: &HeikinAshiResult) {
    for (a, b) in [
        (&left.ha_open, &right.ha_open),
        (&left.ha_high, &right.ha_high),
        (&left.ha_low, &right.ha_low),
        (&left.ha_close, &right.ha_close),
    ] {
        for (index, (&x, &y)) in a.iter().zip(b.iter()).enumerate() {
            assert_eq!(x.is_nan(), y.is_nan(), "NaN mask at {index}");
            if !x.is_nan() {
                assert_eq!(x.to_bits(), y.to_bits(), "value at {index}");
            }
        }
    }
}

fn main() {
    let (mut open, mut high, mut low, mut close) = data(128);
    open[20] = f64::NAN;
    high[47] = f64::NAN;
    low[73] = f64::NAN;
    close[99] = f64::NAN;
    compare(
        &push_reference(&open, &high, &low, &close),
        &indexed(&open, &high, &low, &close),
    );
    compare(
        &push_reference(&open, &high, &low, &close),
        &iterated(&open, &high, &low, &close),
    );
    compare(
        &push_reference(&open, &high, &low, &close),
        &pointer_path(&open, &high, &low, &close),
    );

    for len in [1_000usize, 100_000, 1_000_000] {
        let (open, high, low, close) = data(len);
        let old = || push_reference(&open, &high, &low, &close);
        let new = || indexed(&open, &high, &low, &close);
        let iter = || iterated(&open, &high, &low, &close);
        let pointer = || pointer_path(&open, &high, &low, &close);
        compare(&old(), &new());
        compare(&old(), &iter());
        compare(&old(), &pointer());
        let (mut old_samples, mut new_samples, mut iter_samples, mut pointer_samples) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for turn in 0..7 {
            if turn % 2 == 0 {
                old_samples.push(time(&mut { || old() }, Duration::from_millis(120)));
                new_samples.push(time(&mut { || new() }, Duration::from_millis(120)));
                iter_samples.push(time(&mut { || iter() }, Duration::from_millis(120)));
                pointer_samples.push(time(&mut { || pointer() }, Duration::from_millis(120)));
            } else {
                pointer_samples.push(time(&mut { || pointer() }, Duration::from_millis(120)));
                iter_samples.push(time(&mut { || iter() }, Duration::from_millis(120)));
                new_samples.push(time(&mut { || new() }, Duration::from_millis(120)));
                old_samples.push(time(&mut { || old() }, Duration::from_millis(120)));
            }
        }
        old_samples.sort_by(f64::total_cmp);
        new_samples.sort_by(f64::total_cmp);
        iter_samples.sort_by(f64::total_cmp);
        pointer_samples.sort_by(f64::total_cmp);
        println!(
            "HA {len}: push={:.2}us indexed={:.2}us iter={:.2}us pointer={:.2}us speedup={:.3}x/{:.3}x/{:.3}x",
            old_samples[3],
            new_samples[3],
            iter_samples[3],
            pointer_samples[3],
            old_samples[3] / new_samples[3],
            old_samples[3] / iter_samples[3],
            old_samples[3] / pointer_samples[3]
        );
    }
}
