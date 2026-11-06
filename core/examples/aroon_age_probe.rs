use finkit::indicators::{aroon, aroonosc};
use std::hint::black_box;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
struct IndexTracker<const WANT_MAX: bool> {
    extreme: f64,
    index: usize,
    found: bool,
}

#[derive(Clone, Copy)]
struct BranchlessAgeTracker<const WANT_MAX: bool> {
    extreme: f64,
    age: usize,
    found: bool,
}

impl<const WANT_MAX: bool> BranchlessAgeTracker<WANT_MAX> {
    #[inline(always)]
    fn new() -> Self {
        Self {
            extreme: if WANT_MAX {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            },
            age: 0,
            found: false,
        }
    }

    #[inline(always)]
    fn advance(&mut self, values: &[f64], i: usize, value: f64, period: usize) {
        self.age += self.found as usize;
        let wins = if WANT_MAX {
            value >= self.extreme
        } else {
            value <= self.extreme
        };
        if wins {
            self.extreme = value;
            self.age = 0;
            self.found = true;
        } else if self.found && self.age > period {
            let (extreme, index, found) = old_rescan::<WANT_MAX>(values, i - period, i);
            self.extreme = extreme;
            self.age = if found { i - index } else { 0 };
            self.found = found;
        }
    }
}

impl<const WANT_MAX: bool> IndexTracker<WANT_MAX> {
    #[inline(always)]
    fn new() -> Self {
        Self {
            extreme: if WANT_MAX {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            },
            index: 0,
            found: false,
        }
    }

    #[inline(always)]
    fn advance(&mut self, values: &[f64], i: usize, value: f64, period: usize) {
        let wins = if WANT_MAX {
            value >= self.extreme
        } else {
            value <= self.extreme
        };
        if wins {
            self.extreme = value;
            self.index = i;
            self.found = true;
        } else if self.found && self.index + period < i {
            let (extreme, index, found) = old_rescan::<WANT_MAX>(values, i - period, i);
            self.extreme = extreme;
            self.index = index;
            self.found = found;
        }
    }
}

#[inline(always)]
fn old_rescan<const WANT_MAX: bool>(
    values: &[f64],
    start: usize,
    end: usize,
) -> (f64, usize, bool) {
    let mut best = if WANT_MAX {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };
    let mut best_index = start;
    let mut found = false;
    let mut index = start;
    while index + 3 <= end {
        let c0 = values[index];
        let wins0 = if WANT_MAX { c0 >= best } else { c0 <= best };
        if wins0 {
            best = c0;
            best_index = index;
            found = true;
        }
        let c1 = values[index + 1];
        let wins1 = if WANT_MAX { c1 >= best } else { c1 <= best };
        if wins1 {
            best = c1;
            best_index = index + 1;
            found = true;
        }
        let c2 = values[index + 2];
        let wins2 = if WANT_MAX { c2 >= best } else { c2 <= best };
        if wins2 {
            best = c2;
            best_index = index + 2;
            found = true;
        }
        let c3 = values[index + 3];
        let wins3 = if WANT_MAX { c3 >= best } else { c3 <= best };
        if wins3 {
            best = c3;
            best_index = index + 3;
            found = true;
        }
        index += 4;
    }
    while index <= end {
        let candidate = values[index];
        let wins = if WANT_MAX {
            candidate >= best
        } else {
            candidate <= best
        };
        if wins {
            best = candidate;
            best_index = index;
            found = true;
        }
        index += 1;
    }
    (best, best_index, found)
}

fn old_aroon(high: &[f64], low: &[f64], period: usize) -> (Vec<f64>, Vec<f64>) {
    let scale = 100.0 / period as f64;
    let mut highs = IndexTracker::<true>::new();
    let mut lows = IndexTracker::<false>::new();
    let mut up = Vec::with_capacity(high.len());
    let mut down = Vec::with_capacity(high.len());
    // Mirror the old caller-owned kernel's uninitialized output allocation;
    // both slots are written exactly once on every loop iteration below.
    unsafe {
        up.set_len(high.len());
        down.set_len(high.len());
    }
    for (i, (((&high_bar, &low_bar), up_slot), down_slot)) in high
        .iter()
        .zip(low.iter())
        .zip(up.iter_mut())
        .zip(down.iter_mut())
        .enumerate()
    {
        highs.advance(high, i, high_bar, period);
        lows.advance(low, i, low_bar, period);
        if i < period {
            *up_slot = f64::NAN;
            *down_slot = f64::NAN;
        } else {
            *up_slot = if highs.found {
                (period - (i - highs.index)) as f64 * scale
            } else {
                f64::NAN
            };
            *down_slot = if lows.found {
                (period - (i - lows.index)) as f64 * scale
            } else {
                f64::NAN
            };
        }
    }
    (up, down)
}

#[expect(clippy::uninit_vec)]
fn old_aroonosc(high: &[f64], low: &[f64], period: usize) -> Vec<f64> {
    let scale = 100.0 / period as f64;
    let mut highs = IndexTracker::<true>::new();
    let mut lows = IndexTracker::<false>::new();
    let mut output = Vec::with_capacity(high.len());
    // The loop below writes every output slot, including the warm-up prefix.
    unsafe {
        output.set_len(high.len());
    }
    for (i, ((&high_bar, &low_bar), slot)) in high
        .iter()
        .zip(low.iter())
        .zip(output.iter_mut())
        .enumerate()
    {
        highs.advance(high, i, high_bar, period);
        lows.advance(low, i, low_bar, period);
        *slot = if i >= period && highs.found && lows.found {
            (highs.index as f64 - lows.index as f64) * scale
        } else {
            f64::NAN
        };
    }
    output
}

fn branchless_age_aroon(high: &[f64], low: &[f64], period: usize) -> (Vec<f64>, Vec<f64>) {
    let scale = 100.0 / period as f64;
    let mut highs = BranchlessAgeTracker::<true>::new();
    let mut lows = BranchlessAgeTracker::<false>::new();
    let mut up = Vec::with_capacity(high.len());
    let mut down = Vec::with_capacity(high.len());
    for (i, (&h, &l)) in high.iter().zip(low).enumerate() {
        highs.advance(high, i, h, period);
        lows.advance(low, i, l, period);
        up.push(if i >= period && highs.found {
            (period - highs.age) as f64 * scale
        } else {
            f64::NAN
        });
        down.push(if i >= period && lows.found {
            (period - lows.age) as f64 * scale
        } else {
            f64::NAN
        });
    }
    (up, down)
}

fn branchless_age_aroonosc(high: &[f64], low: &[f64], period: usize) -> Vec<f64> {
    let scale = 100.0 / period as f64;
    let mut highs = BranchlessAgeTracker::<true>::new();
    let mut lows = BranchlessAgeTracker::<false>::new();
    let mut output = Vec::with_capacity(high.len());
    for (i, (&h, &l)) in high.iter().zip(low).enumerate() {
        highs.advance(high, i, h, period);
        lows.advance(low, i, l, period);
        output.push(if i >= period && highs.found && lows.found {
            (lows.age as isize - highs.age as isize) as f64 * scale
        } else {
            f64::NAN
        });
    }
    output
}

fn duration_per_call(mut run: impl FnMut(), target: Duration) -> Duration {
    for _ in 0..3 {
        run();
    }
    let started = Instant::now();
    let mut count = 0;
    while started.elapsed() < target {
        run();
        count += 1;
    }
    started.elapsed() / count
}

fn same(actual: &[f64], expected: &[f64]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(a, b)| (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits())
}

fn main() {
    for len in [1_000, 10_000, 100_000, 1_000_000] {
        let high: Vec<f64> = (0..len)
            .map(|i| {
                let i = i as f64;
                100.0 + i * 0.01 + (i * 0.37).sin() * 2.0 + (i * 1.13).cos() * 1.5
            })
            .collect();
        let low: Vec<f64> = high
            .iter()
            .enumerate()
            .map(|(i, value)| value - 1.0 - ((i as f64 * 0.5).cos().abs() * 0.5))
            .collect();
        let expected = old_aroon(&high, &low, 14);
        let actual = aroon(&high, &low, 14).unwrap();
        let parity = same(actual.aroon_up.as_slice().unwrap(), &expected.0)
            && same(actual.aroon_down.as_slice().unwrap(), &expected.1);
        let expected_osc = old_aroonosc(&high, &low, 14);
        let actual_osc = aroonosc(&high, &low, 14).unwrap();
        let parity_osc = same(actual_osc.as_slice().unwrap(), &expected_osc);
        let branchless_expected = branchless_age_aroon(&high, &low, 14);
        let branchless_actual_osc = branchless_age_aroonosc(&high, &low, 14);
        let branchless_parity = same(actual.aroon_up.as_slice().unwrap(), &branchless_expected.0)
            && same(
                actual.aroon_down.as_slice().unwrap(),
                &branchless_expected.1,
            );
        let branchless_parity_osc = same(actual_osc.as_slice().unwrap(), &branchless_actual_osc);
        let mut edge_high = high.clone();
        let mut edge_low = low.clone();
        if len >= 32 {
            edge_high[5] = f64::NAN;
            edge_low[8] = f64::NAN;
            edge_high[17] = f64::INFINITY;
            edge_low[20] = f64::NEG_INFINITY;
            edge_high[31] = edge_high[30];
            edge_low[30] = edge_low[29];
        }
        let edge_expected = old_aroon(&edge_high, &edge_low, 14);
        let edge_actual = aroon(&edge_high, &edge_low, 14).unwrap();
        let edge_parity = same(edge_actual.aroon_up.as_slice().unwrap(), &edge_expected.0)
            && same(edge_actual.aroon_down.as_slice().unwrap(), &edge_expected.1);
        let edge_expected_osc = old_aroonosc(&edge_high, &edge_low, 14);
        let edge_actual_osc = aroonosc(&edge_high, &edge_low, 14).unwrap();
        let edge_parity_osc = same(edge_actual_osc.as_slice().unwrap(), &edge_expected_osc);
        let rust_time = duration_per_call(
            || {
                black_box(aroon(black_box(&high), black_box(&low), 14).unwrap());
            },
            Duration::from_millis(500),
        );
        let old_time = duration_per_call(
            || {
                black_box(old_aroon(black_box(&high), black_box(&low), 14));
            },
            Duration::from_millis(500),
        );
        let osc_time = duration_per_call(
            || {
                black_box(aroonosc(black_box(&high), black_box(&low), 14).unwrap());
            },
            Duration::from_millis(500),
        );
        let old_osc_time = duration_per_call(
            || {
                black_box(old_aroonosc(black_box(&high), black_box(&low), 14));
            },
            Duration::from_millis(500),
        );
        let branchless_time = duration_per_call(
            || {
                black_box(branchless_age_aroon(black_box(&high), black_box(&low), 14));
            },
            Duration::from_millis(500),
        );
        let branchless_osc_time = duration_per_call(
            || {
                black_box(branchless_age_aroonosc(
                    black_box(&high),
                    black_box(&low),
                    14,
                ));
            },
            Duration::from_millis(500),
        );
        println!(
            "n={len} parity={parity}/{parity_osc} edge_parity={edge_parity}/{edge_parity_osc} branchless_parity={branchless_parity}/{branchless_parity_osc} AROON age/index={}/{} ns ({:.3}x), branchless={} ns ({:.3}x) AROONOSC age/index={}/{} ns ({:.3}x), branchless={} ns ({:.3}x)",
            rust_time.as_nanos(),
            old_time.as_nanos(),
            old_time.as_secs_f64() / rust_time.as_secs_f64(),
            branchless_time.as_nanos(),
            rust_time.as_secs_f64() / branchless_time.as_secs_f64(),
            osc_time.as_nanos(),
            old_osc_time.as_nanos(),
            old_osc_time.as_secs_f64() / osc_time.as_secs_f64(),
            branchless_osc_time.as_nanos(),
            osc_time.as_secs_f64() / branchless_osc_time.as_secs_f64(),
        );
    }
}
