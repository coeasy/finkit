use finkit::indicators::{aroon, aroonosc};
use finkit::talib_ffi::{TA_AROON, TA_AROONOSC};
use std::hint::black_box;
use std::time::Instant;

const CAP: usize = 32;

#[derive(Clone, Copy)]
struct Deque {
    indices: [usize; CAP],
    head: usize,
    len: usize,
}

impl Deque {
    fn new() -> Self {
        Self {
            indices: [0; CAP],
            head: 0,
            len: 0,
        }
    }

    fn back(&self) -> Option<usize> {
        (self.len != 0).then(|| self.indices[(self.head + self.len - 1) % CAP])
    }

    fn front(&self) -> Option<usize> {
        (self.len != 0).then(|| self.indices[self.head])
    }

    fn pop_back(&mut self) {
        self.len -= 1;
    }

    fn pop_front(&mut self) {
        self.head = (self.head + 1) % CAP;
        self.len -= 1;
    }

    fn push_back(&mut self, index: usize) {
        debug_assert!(self.len < CAP);
        self.indices[(self.head + self.len) % CAP] = index;
        self.len += 1;
    }
}

#[inline]
fn monotone_core(
    high: &[f64],
    low: &[f64],
    period: usize,
    mut up: Option<&mut [f64]>,
    mut down: Option<&mut [f64]>,
    mut oscillator: Option<&mut [f64]>,
) {
    assert_eq!(high.len(), low.len());
    assert!(period + 1 <= CAP);
    assert!(up.as_ref().is_none_or(|values| values.len() == high.len()));
    assert!(down
        .as_ref()
        .is_none_or(|values| values.len() == high.len()));
    assert!(oscillator
        .as_ref()
        .is_none_or(|values| values.len() == high.len()));

    let scale = 100.0 / period as f64;
    let mut highs = Deque::new();
    let mut lows = Deque::new();
    for index in 0..high.len() {
        let first_valid = index.saturating_sub(period);
        while highs.front().is_some_and(|old| old < first_valid) {
            highs.pop_front();
        }
        while lows.front().is_some_and(|old| old < first_valid) {
            lows.pop_front();
        }

        if !high[index].is_nan() {
            while highs.back().is_some_and(|old| high[old] <= high[index]) {
                highs.pop_back();
            }
            highs.push_back(index);
        }
        if !low[index].is_nan() {
            while lows.back().is_some_and(|old| low[old] >= low[index]) {
                lows.pop_back();
            }
            lows.push_back(index);
        }

        let (up_value, down_value, osc_value) = if index >= period {
            let up_age = highs.front().map(|extreme| index - extreme);
            let down_age = lows.front().map(|extreme| index - extreme);
            match (up_age, down_age) {
                (Some(up_age), Some(down_age)) => (
                    (period - up_age) as f64 * scale,
                    (period - down_age) as f64 * scale,
                    (down_age as isize - up_age as isize) as f64 * scale,
                ),
                (up_age, down_age) => (
                    up_age.map_or(f64::NAN, |age| (period - age) as f64 * scale),
                    down_age.map_or(f64::NAN, |age| (period - age) as f64 * scale),
                    f64::NAN,
                ),
            }
        } else {
            (f64::NAN, f64::NAN, f64::NAN)
        };
        if let Some(values) = up.as_deref_mut() {
            values[index] = up_value;
        }
        if let Some(values) = down.as_deref_mut() {
            values[index] = down_value;
        }
        if let Some(values) = oscillator.as_deref_mut() {
            values[index] = osc_value;
        }
    }
}

fn monotone_aroon(
    high: &[f64],
    low: &[f64],
    period: usize,
    up: &mut [f64],
    down: &mut [f64],
    oscillator: &mut [f64],
) {
    monotone_core(high, low, period, Some(up), Some(down), Some(oscillator));
}

fn uninit_output(len: usize) -> Vec<f64> {
    let mut output = Vec::with_capacity(len);
    // SAFETY: the candidate kernel writes every element before reading it.
    unsafe { output.set_len(len) };
    output
}

fn monotone_aroon_allocating(high: &[f64], low: &[f64], period: usize) {
    let len = high.len();
    let mut up = uninit_output(len);
    let mut down = uninit_output(len);
    let mut oscillator = uninit_output(len);
    monotone_aroon(high, low, period, &mut up, &mut down, &mut oscillator);
    black_box((up, down, oscillator));
}

fn monotone_pair_allocating(high: &[f64], low: &[f64], period: usize) {
    let len = high.len();
    let mut up = uninit_output(len);
    let mut down = uninit_output(len);
    monotone_core(high, low, period, Some(&mut up), Some(&mut down), None);
    black_box((up, down));
}

fn monotone_osc_allocating(high: &[f64], low: &[f64], period: usize) {
    let mut oscillator = uninit_output(high.len());
    monotone_core(high, low, period, None, None, Some(&mut oscillator));
    black_box(oscillator);
}

fn inputs(len: usize) -> (Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    for i in 0..len {
        let x = i as f64;
        let close = 100.0 + x * 0.006 + (x * 0.017).sin() * 2.1;
        let open = close - 0.15 + (x * 0.031).sin() * 0.05;
        high.push(open.max(close) + 0.7 + (x * 0.023).sin().abs() * 0.3);
        low.push(open.min(close) - 0.8 - (x * 0.019).cos().abs() * 0.2);
    }
    (high, low)
}

fn c_outputs(high: &[f64], low: &[f64], period: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let count = high.len() - period;
    let mut down = vec![0.0; count];
    let mut up = vec![0.0; count];
    let mut osc = vec![0.0; count];
    let mut begin = 0;
    let mut elements = 0;
    // SAFETY: TA-Lib writes `count` dense outputs and both input arrays cover
    // the complete requested range.
    let status = unsafe {
        TA_AROON(
            0,
            (high.len() - 1) as i32,
            high.as_ptr(),
            low.as_ptr(),
            period as i32,
            &mut begin,
            &mut elements,
            down.as_mut_ptr(),
            up.as_mut_ptr(),
        )
    };
    assert_eq!(status, 0);
    assert_eq!(begin, period as i32);
    assert_eq!(elements, count as i32);

    begin = 0;
    elements = 0;
    // SAFETY: TA-Lib writes exactly `count` dense oscillator outputs.
    let status = unsafe {
        TA_AROONOSC(
            0,
            (high.len() - 1) as i32,
            high.as_ptr(),
            low.as_ptr(),
            period as i32,
            &mut begin,
            &mut elements,
            osc.as_mut_ptr(),
        )
    };
    assert_eq!(status, 0);
    assert_eq!(begin, period as i32);
    assert_eq!(elements, count as i32);
    (up, down, osc)
}

fn c_aroon_only(high: &[f64], low: &[f64], period: usize) -> (Vec<f64>, Vec<f64>) {
    let count = high.len() - period;
    let mut down = vec![0.0; count];
    let mut up = vec![0.0; count];
    let mut begin = 0;
    let mut elements = 0;
    // SAFETY: TA-Lib writes `count` dense outputs into the two valid buffers.
    let status = unsafe {
        TA_AROON(
            0,
            (high.len() - 1) as i32,
            high.as_ptr(),
            low.as_ptr(),
            period as i32,
            &mut begin,
            &mut elements,
            down.as_mut_ptr(),
            up.as_mut_ptr(),
        )
    };
    assert_eq!(status, 0);
    assert_eq!(begin, period as i32);
    assert_eq!(elements, count as i32);
    (up, down)
}

fn c_aroonosc_only(high: &[f64], low: &[f64], period: usize) -> Vec<f64> {
    let count = high.len() - period;
    let mut output = vec![0.0; count];
    let mut begin = 0;
    let mut elements = 0;
    // SAFETY: TA-Lib writes `count` dense oscillator outputs.
    let status = unsafe {
        TA_AROONOSC(
            0,
            (high.len() - 1) as i32,
            high.as_ptr(),
            low.as_ptr(),
            period as i32,
            &mut begin,
            &mut elements,
            output.as_mut_ptr(),
        )
    };
    assert_eq!(status, 0);
    assert_eq!(begin, period as i32);
    assert_eq!(elements, count as i32);
    output
}

fn elapsed_us(mut body: impl FnMut()) -> f64 {
    let started = Instant::now();
    body();
    started.elapsed().as_secs_f64() * 1e6
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn run(len: usize, period: usize) {
    let (high, low) = inputs(len);
    let mut up = vec![0.0; len];
    let mut down = vec![0.0; len];
    let mut osc = vec![0.0; len];
    monotone_aroon(&high, &low, period, &mut up, &mut down, &mut osc);
    let public = aroon(&high, &low, period).unwrap();
    let public_osc = aroonosc(&high, &low, period).unwrap();
    let c = c_outputs(&high, &low, period);
    let mut mismatches = [0usize; 3];
    let mut public_mismatches = [0usize; 3];
    let mut max_abs = [0.0f64; 3];
    let public_lanes = [
        (public.aroon_up.as_slice().unwrap(), &c.0),
        (public.aroon_down.as_slice().unwrap(), &c.1),
        (public_osc.as_slice().unwrap(), &c.2),
    ];
    for (lane, (actual, expected)) in [(&up, &c.0), (&down, &c.1), (&osc, &c.2)]
        .into_iter()
        .enumerate()
    {
        for (index, (actual, expected)) in actual.iter().skip(period).zip(expected).enumerate() {
            let difference = (actual - expected).abs();
            max_abs[lane] = max_abs[lane].max(difference);
            if difference > 1e-10 + 1e-10 * actual.abs().max(expected.abs()) {
                mismatches[lane] += 1;
            }
            if actual.is_nan() || expected.is_nan() {
                if !(actual.is_nan() && expected.is_nan()) {
                    mismatches[lane] += 1;
                }
            }
            let _ = index;
        }
    }
    for (lane, (actual, expected)) in public_lanes.into_iter().enumerate() {
        for (actual, expected) in actual.iter().skip(period).zip(expected) {
            let tolerance = 1e-10 + 1e-10 * actual.abs().max(expected.abs());
            if (actual - expected).abs() > tolerance || actual.is_nan() != expected.is_nan() {
                public_mismatches[lane] += 1;
            }
        }
    }
    assert_eq!(public_mismatches, [0, 0, 0]);

    let mut candidate = [0.0; 9];
    let mut current = [0.0; 9];
    let mut c_samples = [0.0; 9];
    for index in 0..candidate.len() {
        match index % 3 {
            0 => {
                candidate[index] = elapsed_us(|| {
                    monotone_aroon_allocating(black_box(&high), black_box(&low), period);
                });
                current[index] = elapsed_us(|| {
                    black_box(aroon(black_box(&high), black_box(&low), period).unwrap());
                    black_box(aroonosc(black_box(&high), black_box(&low), period).unwrap());
                });
                c_samples[index] = elapsed_us(|| {
                    black_box(c_outputs(black_box(&high), black_box(&low), period));
                });
            }
            1 => {
                current[index] = elapsed_us(|| {
                    black_box(aroon(black_box(&high), black_box(&low), period).unwrap());
                    black_box(aroonosc(black_box(&high), black_box(&low), period).unwrap());
                });
                c_samples[index] = elapsed_us(|| {
                    black_box(c_outputs(black_box(&high), black_box(&low), period));
                });
                candidate[index] = elapsed_us(|| {
                    monotone_aroon_allocating(black_box(&high), black_box(&low), period);
                });
            }
            _ => {
                c_samples[index] = elapsed_us(|| {
                    black_box(c_outputs(black_box(&high), black_box(&low), period));
                });
                candidate[index] = elapsed_us(|| {
                    monotone_aroon_allocating(black_box(&high), black_box(&low), period);
                });
                current[index] = elapsed_us(|| {
                    black_box(aroon(black_box(&high), black_box(&low), period).unwrap());
                    black_box(aroonosc(black_box(&high), black_box(&low), period).unwrap());
                });
            }
        }
    }
    let candidate_us = median(&mut candidate);
    let current_us = median(&mut current);
    let c_us = median(&mut c_samples);
    let mut pair_candidate = [0.0; 9];
    let mut pair_current = [0.0; 9];
    let mut pair_c = [0.0; 9];
    let mut osc_candidate = [0.0; 9];
    let mut osc_current = [0.0; 9];
    let mut osc_c = [0.0; 9];
    for index in 0..9 {
        match index % 3 {
            0 => {
                pair_candidate[index] = elapsed_us(|| {
                    monotone_pair_allocating(&high, &low, period);
                });
                pair_current[index] = elapsed_us(|| {
                    black_box(aroon(&high, &low, period).unwrap());
                });
                pair_c[index] = elapsed_us(|| {
                    black_box(c_aroon_only(&high, &low, period));
                });
                osc_candidate[index] = elapsed_us(|| {
                    monotone_osc_allocating(&high, &low, period);
                });
                osc_current[index] = elapsed_us(|| {
                    black_box(aroonosc(&high, &low, period).unwrap());
                });
                osc_c[index] = elapsed_us(|| {
                    black_box(c_aroonosc_only(&high, &low, period));
                });
            }
            1 => {
                pair_current[index] = elapsed_us(|| {
                    black_box(aroon(&high, &low, period).unwrap());
                });
                pair_c[index] = elapsed_us(|| {
                    black_box(c_aroon_only(&high, &low, period));
                });
                pair_candidate[index] = elapsed_us(|| {
                    monotone_pair_allocating(&high, &low, period);
                });
                osc_current[index] = elapsed_us(|| {
                    black_box(aroonosc(&high, &low, period).unwrap());
                });
                osc_c[index] = elapsed_us(|| {
                    black_box(c_aroonosc_only(&high, &low, period));
                });
                osc_candidate[index] = elapsed_us(|| {
                    monotone_osc_allocating(&high, &low, period);
                });
            }
            _ => {
                pair_c[index] = elapsed_us(|| {
                    black_box(c_aroon_only(&high, &low, period));
                });
                pair_candidate[index] = elapsed_us(|| {
                    monotone_pair_allocating(&high, &low, period);
                });
                pair_current[index] = elapsed_us(|| {
                    black_box(aroon(&high, &low, period).unwrap());
                });
                osc_c[index] = elapsed_us(|| {
                    black_box(c_aroonosc_only(&high, &low, period));
                });
                osc_candidate[index] = elapsed_us(|| {
                    monotone_osc_allocating(&high, &low, period);
                });
                osc_current[index] = elapsed_us(|| {
                    black_box(aroonosc(&high, &low, period).unwrap());
                });
            }
        }
    }
    let pair_candidate_us = median(&mut pair_candidate);
    let pair_current_us = median(&mut pair_current);
    let pair_c_us = median(&mut pair_c);
    let osc_candidate_us = median(&mut osc_candidate);
    let osc_current_us = median(&mut osc_current);
    let osc_c_us = median(&mut osc_c);
    println!(
        "AROON monotone n={len} p={period} candidate_mismatch={:?} public_mismatch={:?} max_abs={:?} both={candidate_us:.2}/{current_us:.2}/{c_us:.2}us pair={pair_candidate_us:.2}/{pair_current_us:.2}/{pair_c_us:.2}us osc={osc_candidate_us:.2}/{osc_current_us:.2}/{osc_c_us:.2}us (cand/current/C)",
        mismatches,
        public_mismatches,
        max_abs,
    );
}

fn check_special_inputs() {
    let mut high = Vec::with_capacity(257);
    let mut low = Vec::with_capacity(257);
    for index in 0..257 {
        let mut high_value = (index % 7) as f64;
        let mut low_value = -((index % 5) as f64);
        if index % 17 == 0 {
            high_value = f64::NAN;
        }
        if index % 19 == 0 {
            low_value = f64::NAN;
        }
        if index % 31 == 4 {
            high_value = f64::INFINITY;
        }
        if index % 37 == 6 {
            low_value = f64::NEG_INFINITY;
        }
        high.push(high_value);
        low.push(low_value);
    }
    for period in [2, 5, 7, 10, 14, 20, 31] {
        let mut up = uninit_output(high.len());
        let mut down = uninit_output(high.len());
        let mut osc = uninit_output(high.len());
        monotone_aroon(&high, &low, period, &mut up, &mut down, &mut osc);
        let expected = aroon(&high, &low, period).unwrap();
        let expected_osc = aroonosc(&high, &low, period).unwrap();
        for (label, actual, expected) in [
            ("up", up.as_slice(), expected.aroon_up.as_slice().unwrap()),
            (
                "down",
                down.as_slice(),
                expected.aroon_down.as_slice().unwrap(),
            ),
            ("osc", osc.as_slice(), expected_osc.as_slice().unwrap()),
        ] {
            for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
                assert!(
                    (actual.is_nan() && expected.is_nan())
                        || actual.to_bits() == expected.to_bits(),
                    "special-input {label} mismatch p={period} i={index}: {actual} vs {expected}"
                );
            }
        }
    }
}

fn main() {
    check_special_inputs();
    for period in [2, 5, 7, 10, 14, 20, 31] {
        for len in [1_000, 100_000, 1_000_000] {
            run(len, period);
        }
    }
}
