use finkit::patterns::candlestick::evening_doji_star;
use finkit::talib_ffi::{TA_Initialize, TA_CDLEVENINGDOJISTAR, TA_SUCCESS};
use std::hint::black_box;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
struct Average10 {
    values: [f64; 10],
    sum: f64,
    next: usize,
    count: usize,
}

impl Average10 {
    fn new() -> Self {
        Self {
            values: [0.0; 10],
            sum: 0.0,
            next: 0,
            count: 0,
        }
    }

    fn push(&mut self, value: f64) {
        if self.count == 10 {
            self.sum -= self.values[self.next];
        } else {
            self.count += 1;
        }
        self.values[self.next] = value;
        self.sum += value;
        self.next += 1;
        if self.next == 10 {
            self.next = 0;
        }
    }

    fn average(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }

    fn push_full(&mut self, value: f64) {
        self.sum -= self.values[self.next];
        self.values[self.next] = value;
        self.sum += value;
        self.next += 1;
        if self.next == 10 {
            self.next = 0;
        }
    }

    fn average_full(&self) -> f64 {
        self.sum / 10.0
    }
}

fn body(open: f64, close: f64) -> f64 {
    (close - open).abs()
}

fn previous_loop(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Vec<i32> {
    let len = open.len();
    let mut output = vec![0; len];
    let mut body_long = Average10::new();
    let mut body_doji = Average10::new();
    let mut body_short = Average10::new();
    for i in 0..len {
        if i >= 3 {
            body_long.push(body(open[i - 3], close[i - 3]));
        }
        if i >= 2 {
            body_doji.push(high[i - 2] - low[i - 2]);
        }
        if i >= 3 {
            body_short.push(body(open[i - 1], close[i - 1]));
        }
        if i < 12 {
            continue;
        }
        let first_body = body(open[i - 2], close[i - 2]);
        let second_body = body(open[i - 1], close[i - 1]);
        let third_body = body(open[i], close[i]);
        let first_white = close[i - 2] >= open[i - 2];
        let current_black = close[i] < open[i];
        let gap_up = open[i - 1].min(close[i - 1]) > open[i - 2].max(close[i - 2]);
        if first_white
            && gap_up
            && current_black
            && first_body > body_long.average()
            && second_body <= body_doji.average() * 0.1
            && third_body > body_short.average()
            && close[i] < close[i - 2] - first_body * 0.3
        {
            output[i] = -100;
        }
    }
    output
}

#[inline(always)]
fn classify(
    output: &mut [i32],
    i: usize,
    open: &[f64],
    close: &[f64],
    body_long_avg: f64,
    body_doji_avg: f64,
    body_short_avg: f64,
) {
    let first_body = body(open[i - 2], close[i - 2]);
    let second_body = body(open[i - 1], close[i - 1]);
    let third_body = body(open[i], close[i]);
    let first_white = close[i - 2] >= open[i - 2];
    let current_black = close[i] < open[i];
    let gap_up = open[i - 1].min(close[i - 1]) > open[i - 2].max(close[i - 2]);
    if first_white
        && gap_up
        && current_black
        && first_body > body_long_avg
        && second_body <= body_doji_avg * 0.1
        && third_body > body_short_avg
        && close[i] < close[i - 2] - first_body * 0.3
    {
        output[i] = -100;
    }
}

fn split_loop(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Vec<i32> {
    let len = open.len();
    let mut output = vec![0; len];
    if len == 12 {
        return output;
    }
    let mut body_long = Average10::new();
    let mut body_doji = Average10::new();
    let mut body_short = Average10::new();

    for i in 0..9 {
        body_long.push(body(open[i], close[i]));
    }
    for i in 0..10 {
        body_doji.push(high[i] - low[i]);
    }
    for i in 2..11 {
        body_short.push(body(open[i], close[i]));
    }

    // Preserve the generic update sequence for the first full window. After
    // this point all three ten-sample windows remain full, so the hot loop can
    // update their sums without count checks.
    body_long.push(body(open[9], close[9]));
    body_doji.push(high[10] - low[10]);
    body_short.push(body(open[11], close[11]));
    classify(
        &mut output,
        12,
        open,
        close,
        body_long.average_full(),
        body_doji.average_full(),
        body_short.average_full(),
    );

    for i in 13..len {
        body_long.push_full(body(open[i - 3], close[i - 3]));
        body_doji.push_full(high[i - 2] - low[i - 2]);
        body_short.push_full(body(open[i - 1], close[i - 1]));
        classify(
            &mut output,
            i,
            open,
            close,
            body_long.average_full(),
            body_doji.average_full(),
            body_short.average_full(),
        );
    }
    output
}

fn inputs(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
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
        low.push(o.min(c) - 0.8 - (x * 0.019).cos().abs() * 0.2);
        close.push(c);
    }
    (open, high, low, close)
}

fn talib(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Vec<i32> {
    let mut raw = vec![0; close.len()];
    let (mut begin, mut count) = (0, 0);
    // SAFETY: contiguous equal-length inputs and valid output storage.
    let status = unsafe {
        TA_CDLEVENINGDOJISTAR(
            0,
            close.len() as i32 - 1,
            open.as_ptr(),
            high.as_ptr(),
            low.as_ptr(),
            close.as_ptr(),
            0.3,
            &mut begin,
            &mut count,
            raw.as_mut_ptr(),
        )
    };
    assert_eq!(status, TA_SUCCESS);
    let mut output = vec![0; close.len()];
    let begin = begin as usize;
    let count = count as usize;
    output[begin..begin + count].copy_from_slice(&raw[..count]);
    output
}

fn time(mut run: impl FnMut(), duration: Duration) -> f64 {
    let start = Instant::now();
    let mut calls = 0u64;
    while start.elapsed() < duration {
        run();
        calls += 1;
    }
    start.elapsed().as_secs_f64() * 1e6 / calls as f64
}

fn main() {
    assert_eq!(unsafe { TA_Initialize() }, 0);
    for len in [12usize, 13, 1_000, 100_000, 1_000_000] {
        let (open, high, low, close) = inputs(len);
        let previous = || previous_loop(&open, &high, &low, &close);
        let public = || {
            evening_doji_star(&open, &high, &low, &close, 0.1)
                .unwrap()
                .into_raw_vec_and_offset()
                .0
        };
        let candidate = || split_loop(&open, &high, &low, &close);
        let c = || talib(&open, &high, &low, &close);
        let expected = previous();
        let actual = public();
        assert_eq!(expected, actual, "optimized loop differs at n={len}");
        assert_eq!(actual, candidate(), "split loop differs at n={len}");
        assert_eq!(expected, c(), "TA-Lib mismatch at n={len}");
        let (mut old_samples, mut new_samples, mut c_samples) =
            (Vec::new(), Vec::new(), Vec::new());
        for turn in 0..5 {
            match turn % 3 {
                0 => {
                    old_samples.push(time(
                        || {
                            black_box(previous());
                        },
                        Duration::from_millis(100),
                    ));
                    new_samples.push(time(
                        || {
                            black_box(public());
                        },
                        Duration::from_millis(100),
                    ));
                    c_samples.push(time(
                        || {
                            black_box(c());
                        },
                        Duration::from_millis(100),
                    ));
                }
                1 => {
                    new_samples.push(time(
                        || {
                            black_box(public());
                        },
                        Duration::from_millis(100),
                    ));
                    c_samples.push(time(
                        || {
                            black_box(c());
                        },
                        Duration::from_millis(100),
                    ));
                    old_samples.push(time(
                        || {
                            black_box(previous());
                        },
                        Duration::from_millis(100),
                    ));
                }
                _ => {
                    c_samples.push(time(
                        || {
                            black_box(c());
                        },
                        Duration::from_millis(100),
                    ));
                    old_samples.push(time(
                        || {
                            black_box(previous());
                        },
                        Duration::from_millis(100),
                    ));
                    new_samples.push(time(
                        || {
                            black_box(public());
                        },
                        Duration::from_millis(100),
                    ));
                }
            }
        }
        for samples in [&mut old_samples, &mut new_samples, &mut c_samples] {
            samples.sort_by(f64::total_cmp);
        }
        println!("CDLEVENINGDOJISTAR n={len}: previous={:.2}us optimized={:.2}us TA-Lib={:.2}us speedup={:.3}x", old_samples[2], new_samples[2], c_samples[2], old_samples[2] / new_samples[2]);
    }
}
