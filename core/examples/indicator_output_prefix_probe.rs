use finkit::indicators::momentum_ext::coppock;
use finkit::indicators::talib_ext::{cvi, efi, eri, smi, EriResult};
use finkit::math::moving_avg::ema;
use ndarray::Array1;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn data(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    for i in 0..len {
        let x = i as f64;
        let value = 100.0 + 0.006 * x + (x * 0.017).sin() * 2.1;
        close.push(value);
        high.push(value + 0.7 + (x * 0.023).sin().abs() * 0.3);
        low.push(value - 0.8 - (x * 0.019).cos().abs() * 0.2);
    }
    (high, low, close)
}

fn old_cvi(high: &[f64], low: &[f64], p: usize, roc: usize) -> Array1<f64> {
    let range: Vec<f64> = high.iter().zip(low).map(|(h, l)| h - l).collect();
    let e = ema(&range, p).unwrap();
    let mut out = Array1::from_elem(high.len(), f64::NAN);
    for i in p - 1 + roc..high.len() {
        let prev = e[i - roc];
        out[i] = if prev == 0.0 {
            0.0
        } else {
            100.0 * (e[i] - prev) / prev
        };
    }
    out
}

fn old_eri(high: &[f64], low: &[f64], close: &[f64], p: usize) -> EriResult {
    let e = ema(close, p).unwrap();
    let mut bull = Array1::from_elem(high.len(), f64::NAN);
    let mut bear = Array1::from_elem(high.len(), f64::NAN);
    for i in p - 1..high.len() {
        bull[i] = high[i] - e[i];
        bear[i] = low[i] - e[i];
    }
    EriResult {
        bullpower: bull,
        bearpower: bear,
    }
}

fn old_efi(close: &[f64], volume: &[f64], p: usize) -> Array1<f64> {
    let mut force = vec![0.0; close.len()];
    for i in 1..close.len() {
        force[i] = (close[i] - close[i - 1]) * volume[i];
    }
    let mut out = Array1::from_elem(close.len(), f64::NAN);
    if p == 1 {
        for i in 1..close.len() {
            out[i] = force[i];
        }
        return out;
    }
    let mut value = force[1..=p].iter().sum::<f64>() / p as f64;
    out[p] = value;
    let alpha = 2.0 / (p as f64 + 1.0);
    for i in p + 1..close.len() {
        value += (force[i] - value) * alpha;
        out[i] = value;
    }
    out
}

fn old_coppock(input: &[f64], wma_period: usize, long_roc: usize, short_roc: usize) -> Array1<f64> {
    let len = input.len();
    let roc_start = long_roc.max(short_roc);
    let mut combined = Array1::from_elem(len, f64::NAN);
    for i in roc_start..len {
        let long_base = input[i - long_roc];
        let short_base = input[i - short_roc];
        let long_value = if long_base != 0.0 {
            (input[i] - long_base) / long_base * 100.0
        } else {
            f64::NAN
        };
        let short_value = if short_base != 0.0 {
            (input[i] - short_base) / short_base * 100.0
        } else {
            f64::NAN
        };
        if !long_value.is_nan() && !short_value.is_nan() {
            combined[i] = long_value + short_value;
        }
    }

    let mut output = Array1::from_elem(len, f64::NAN);
    let inverse_weight_sum = 1.0 / (wma_period * (wma_period + 1) / 2) as f64;
    let wma_first = roc_start + wma_period - 1;
    let win_start = wma_first + 1 - wma_period;
    let combined = combined.as_slice().unwrap();
    let mut window_sum: f64 = combined[win_start..=wma_first].iter().sum();
    let mut weighted_sum: f64 = combined[win_start..=wma_first]
        .iter()
        .enumerate()
        .map(|(j, &value)| (j + 1) as f64 * value)
        .sum();
    output[wma_first] = weighted_sum * inverse_weight_sum;
    let mut dirty = window_sum.is_nan();

    for i in wma_first + 1..len {
        let old = combined[i - wma_period];
        let new = combined[i];
        if dirty || old.is_nan() || new.is_nan() {
            let start = i + 1 - wma_period;
            let window = &combined[start..=i];
            window_sum = window.iter().sum();
            weighted_sum = window
                .iter()
                .enumerate()
                .map(|(j, &value)| (j + 1) as f64 * value)
                .sum();
            dirty = window_sum.is_nan();
        } else {
            weighted_sum += wma_period as f64 * new - window_sum;
            window_sum += new - old;
        }
        output[i] = weighted_sum * inverse_weight_sum;
    }
    output
}

fn old_smi(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    p: usize,
    fast: usize,
    slow: usize,
    signal: usize,
) -> (Array1<f64>, Array1<f64>) {
    let len = high.len();
    let (hi, lo) = finkit::indicators::talib_ext::rolling_minmax(high, low, p);
    let mut n = vec![f64::NAN; len];
    let mut den = vec![f64::NAN; len];
    for i in p - 1..len {
        n[i] = close[i] - (hi[i] + lo[i]) * 0.5;
        den[i] = hi[i] - lo[i];
    }
    let ns = ema(&n[p - 1..], slow).unwrap();
    let ds = ema(&den[p - 1..], slow).unwrap();
    let nsv = ns.to_vec();
    let dsv = ds.to_vec();
    let nf = ema(&nsv[slow - 1..], fast).unwrap();
    let df = ema(&dsv[slow - 1..], fast).unwrap();
    let base = p - 1 + slow - 1 + fast - 1;
    let mut raw = vec![f64::NAN; len];
    for i in fast - 1..nf.len() {
        let idx = base + i - (fast - 1);
        if idx >= len {
            break;
        }
        let denominator = df[i] * 0.5;
        raw[idx] = if denominator == 0.0 {
            0.0
        } else {
            100.0 * nf[i] / denominator
        };
    }
    let sig = ema(&raw[base..], signal).unwrap();
    let mut value = Array1::from_elem(len, f64::NAN);
    let mut line = Array1::from_elem(len, f64::NAN);
    for i in signal - 1..sig.len() {
        let idx = base + i;
        if idx < len {
            line[idx] = sig[i];
            value[idx] = raw[idx];
        }
    }
    (value, line)
}

fn time<F: FnMut() -> T, T>(mut f: F, duration: Duration) -> f64 {
    let start = Instant::now();
    let mut count = 0u64;
    while start.elapsed() < duration {
        black_box(f());
        count += 1;
    }
    start.elapsed().as_secs_f64() * 1e6 / count as f64
}

fn main() {
    for len in [1_000usize, 100_000, 1_000_000] {
        let (high, low, close) = data(len);
        let volume: Vec<_> = (0..len)
            .map(|i| 1_000_000.0 + (i as f64 * 0.013).sin() * 20_000.0)
            .collect();
        let old_efi_fn = || old_efi(&close, &volume, 13);
        let new_efi_fn = || efi(&close, &volume, 13).unwrap();
        let old_efi = old_efi_fn();
        let new_efi = new_efi_fn();
        let efi_max_diff = old_efi
            .iter()
            .zip(new_efi.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert_eq!(
            old_efi.iter().map(|v| v.is_nan()).collect::<Vec<_>>(),
            new_efi.iter().map(|v| v.is_nan()).collect::<Vec<_>>()
        );
        assert!(efi_max_diff <= 1e-8, "EFI max diff={efi_max_diff}");
        for _ in 0..2 {
            black_box(old_efi_fn());
            black_box(new_efi_fn());
        }
        let mut old_efi_samples = Vec::new();
        let mut new_efi_samples = Vec::new();
        for turn in 0..5 {
            if turn % 2 == 0 {
                old_efi_samples.push(time(&mut { || old_efi_fn() }, Duration::from_millis(120)));
                new_efi_samples.push(time(&mut { || new_efi_fn() }, Duration::from_millis(120)));
            } else {
                new_efi_samples.push(time(&mut { || new_efi_fn() }, Duration::from_millis(120)));
                old_efi_samples.push(time(&mut { || old_efi_fn() }, Duration::from_millis(120)));
            }
        }
        old_efi_samples.sort_by(f64::total_cmp);
        new_efi_samples.sort_by(f64::total_cmp);
        println!(
            "EFI {len}: old={:.2}us new={:.2}us speedup={:.3}x max_abs={efi_max_diff:.3e}",
            old_efi_samples[2],
            new_efi_samples[2],
            old_efi_samples[2] / new_efi_samples[2]
        );

        let old = || old_cvi(&high, &low, 10, 10);
        let new = || cvi(&high, &low, 10, 10).unwrap();
        let expected = old();
        let actual = new();
        let max_diff = expected
            .iter()
            .zip(actual.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert_eq!(
            expected.iter().map(|v| v.is_nan()).collect::<Vec<_>>(),
            actual.iter().map(|v| v.is_nan()).collect::<Vec<_>>()
        );
        assert!(max_diff <= 1e-8, "max diff={max_diff}");
        for _ in 0..2 {
            black_box(old());
            black_box(new());
        }
        let mut old_samples = Vec::new();
        let mut new_samples = Vec::new();
        for turn in 0..5 {
            if turn % 2 == 0 {
                old_samples.push(time(&mut { || old() }, Duration::from_millis(120)));
                new_samples.push(time(&mut { || new() }, Duration::from_millis(120)));
            } else {
                new_samples.push(time(&mut { || new() }, Duration::from_millis(120)));
                old_samples.push(time(&mut { || old() }, Duration::from_millis(120)));
            }
        }
        old_samples.sort_by(f64::total_cmp);
        new_samples.sort_by(f64::total_cmp);
        let old_us = old_samples[2];
        let new_us = new_samples[2];
        println!(
            "CVI {len}: old={old_us:.2}us new={new_us:.2}us speedup={:.3}x max_abs={max_diff:.3e}",
            old_us / new_us
        );

        let old_eri_fn = || old_eri(&high, &low, &close, 13);
        let new_eri_fn = || eri(&high, &low, &close, 13).unwrap();
        let old = old_eri_fn();
        let new = new_eri_fn();
        let max_diff = old
            .bullpower
            .iter()
            .zip(new.bullpower.iter())
            .chain(old.bearpower.iter().zip(new.bearpower.iter()))
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert_eq!(
            old.bullpower.iter().map(|v| v.is_nan()).collect::<Vec<_>>(),
            new.bullpower.iter().map(|v| v.is_nan()).collect::<Vec<_>>()
        );
        assert_eq!(
            old.bearpower.iter().map(|v| v.is_nan()).collect::<Vec<_>>(),
            new.bearpower.iter().map(|v| v.is_nan()).collect::<Vec<_>>()
        );
        assert!(max_diff <= 1e-8, "max diff={max_diff}");
        for _ in 0..2 {
            black_box(old_eri_fn());
            black_box(new_eri_fn());
        }
        let mut old_samples = Vec::new();
        let mut new_samples = Vec::new();
        for turn in 0..5 {
            if turn % 2 == 0 {
                old_samples.push(time(&mut { || old_eri_fn() }, Duration::from_millis(120)));
                new_samples.push(time(&mut { || new_eri_fn() }, Duration::from_millis(120)));
            } else {
                new_samples.push(time(&mut { || new_eri_fn() }, Duration::from_millis(120)));
                old_samples.push(time(&mut { || old_eri_fn() }, Duration::from_millis(120)));
            }
        }
        old_samples.sort_by(f64::total_cmp);
        new_samples.sort_by(f64::total_cmp);
        let old_us = old_samples[2];
        let new_us = new_samples[2];
        println!(
            "ERI {len}: old={old_us:.2}us new={new_us:.2}us speedup={:.3}x max_abs={max_diff:.3e}",
            old_us / new_us
        );

        let old_coppock_fn = || old_coppock(&close, 11, 14, 10);
        let new_coppock_fn = || coppock(&close, 11, 14, 10).unwrap();
        let old = old_coppock_fn();
        let new = new_coppock_fn();
        let max_diff = old
            .iter()
            .zip(new.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert_eq!(
            old.iter().map(|v| v.is_nan()).collect::<Vec<_>>(),
            new.iter().map(|v| v.is_nan()).collect::<Vec<_>>()
        );
        assert!(max_diff <= 1e-8, "COPPOCK max diff={max_diff}");
        for _ in 0..2 {
            black_box(old_coppock_fn());
            black_box(new_coppock_fn());
        }
        let mut old_samples = Vec::new();
        let mut new_samples = Vec::new();
        for turn in 0..5 {
            if turn % 2 == 0 {
                old_samples.push(time(
                    &mut { || old_coppock_fn() },
                    Duration::from_millis(120),
                ));
                new_samples.push(time(
                    &mut { || new_coppock_fn() },
                    Duration::from_millis(120),
                ));
            } else {
                new_samples.push(time(
                    &mut { || new_coppock_fn() },
                    Duration::from_millis(120),
                ));
                old_samples.push(time(
                    &mut { || old_coppock_fn() },
                    Duration::from_millis(120),
                ));
            }
        }
        old_samples.sort_by(f64::total_cmp);
        new_samples.sort_by(f64::total_cmp);
        let old_us = old_samples[2];
        let new_us = new_samples[2];
        println!(
            "COPPOCK {len}: old={old_us:.2}us new={new_us:.2}us speedup={:.3}x max_abs={max_diff:.3e}",
            old_us / new_us
        );

        let old_smi_fn = || old_smi(&high, &low, &close, 13, 2, 25, 9);
        let new_smi_fn = || {
            let result = smi(&high, &low, &close, 13, 2, 25, 9).unwrap();
            (result.smi, result.smisignal)
        };
        let (old_value, old_line) = old_smi_fn();
        let (new_value, new_line) = new_smi_fn();
        let max_diff = old_value
            .iter()
            .zip(new_value.iter())
            .chain(old_line.iter().zip(new_line.iter()))
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert_eq!(
            old_value.iter().map(|v| v.is_nan()).collect::<Vec<_>>(),
            new_value.iter().map(|v| v.is_nan()).collect::<Vec<_>>()
        );
        assert_eq!(
            old_line.iter().map(|v| v.is_nan()).collect::<Vec<_>>(),
            new_line.iter().map(|v| v.is_nan()).collect::<Vec<_>>()
        );
        assert!(max_diff <= 1e-8, "SMI max diff={max_diff}");
        for _ in 0..2 {
            black_box(old_smi_fn());
            black_box(new_smi_fn());
        }
        let mut old_samples = Vec::new();
        let mut new_samples = Vec::new();
        for turn in 0..5 {
            if turn % 2 == 0 {
                old_samples.push(time(&mut { || old_smi_fn() }, Duration::from_millis(120)));
                new_samples.push(time(&mut { || new_smi_fn() }, Duration::from_millis(120)));
            } else {
                new_samples.push(time(&mut { || new_smi_fn() }, Duration::from_millis(120)));
                old_samples.push(time(&mut { || old_smi_fn() }, Duration::from_millis(120)));
            }
        }
        old_samples.sort_by(f64::total_cmp);
        new_samples.sort_by(f64::total_cmp);
        let old_us = old_samples[2];
        let new_us = new_samples[2];
        println!(
            "SMI {len}: old={old_us:.2}us new={new_us:.2}us speedup={:.3}x max_abs={max_diff:.3e}",
            old_us / new_us
        );
    }
}
