//! Formula engine throughput vs the native API vs TA-Lib C.
//!
//! Three columns answer three different questions:
//!
//! * **formula** — `FormulaEngine::eval`, the entry point a host application
//!   uses for user-authored source text. Includes parse-cache lookup, argument
//!   dispatch and the canonical kernel.
//! * **planned** — `FormulaEngine::execute` on an already compiled formula:
//!   the same run without the compile/cache step. The gap between this and
//!   *native* is the engine's own overhead.
//! * **native** — the direct Rust indicator call. No parsing, no dispatch.
//! * **talib-c** — TA-Lib 0.8.x through the C ABI (requires the `talib-c`
//!   feature).
//!
//! Run with the feature to get all four columns:
//!
//! ```text
//! cargo run --release --features talib-c --example formula_vs_talib -p finkit
//! ```
//!
//! # Methodology
//!
//! Per V4 §44.16, cross-run absolute readings on this machine swing ±20% with
//! a run-order artifact, so every number here comes from **one process,
//! interleaved rounds**: the implementations run back-to-back against the same
//! data, in alternating order (ABC then CBA), and the reported figure is the
//! median of the round times. That makes the *ratios* within a row meaningful
//! even when the absolute microsecond level drifts between sessions.

use finkit::formula::{FormulaContext, FormulaEngine};
use finkit::indicators::momentum::rsi;
use finkit::indicators::volatility::atr;
use finkit::math::moving_avg::{dema, ema, sma, tema, trima, wma};
use ndarray::Array1;

#[cfg(feature = "talib-c")]
use finkit::talib_ffi::{
    call_hlc, call_single_in, TA_ATR, TA_DEMA, TA_EMA, TA_KAMA, TA_RSI, TA_SMA, TA_TEMA, TA_TRIMA,
    TA_WMA,
};

/// Bars used per measurement. 10k matches the tracked Criterion suite; 100k
/// shows whether the engine overhead is per-bar or per-call.
const LENGTHS: [usize; 2] = [10_000, 100_000];
/// Interleaved rounds per (case, length).
const ROUNDS: usize = 15;
/// Repetitions inside one round, so a single round is long enough to time.
const REPEATS: usize = 20;

struct Data {
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
}

fn make_data(len: usize) -> Data {
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut volume = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f64;
        let noise = (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5 + (t * 3.71).sin() * 0.8;
        let price = 100.0 + t * 0.01 + noise;
        open.push(price - 0.3);
        high.push(price + 1.0 + (t * 0.7).sin().abs() * 0.5);
        low.push(price - 1.0 - (t * 0.5).cos().abs() * 0.5);
        close.push(price);
        volume.push(10_000.0 + (t * 10.0).sin() * 3_000.0 + 2_000.0 * (t * 2.3).cos().abs());
    }
    Data {
        open,
        high,
        low,
        close,
        volume,
    }
}

fn make_ctx(data: &Data) -> FormulaContext {
    FormulaContext::new(
        Array1::from_vec(data.open.clone()),
        Array1::from_vec(data.high.clone()),
        Array1::from_vec(data.low.clone()),
        Array1::from_vec(data.close.clone()),
        Array1::from_vec(data.volume.clone()),
        None,
    )
}

/// One measurement row: label, formula source, and the three closures.
struct Case {
    label: &'static str,
    source: &'static str,
    native: fn(&Data) -> Array1<f64>,
    #[cfg(feature = "talib-c")]
    talib: fn(&Data) -> Vec<f64>,
}

fn n_sma(d: &Data) -> Array1<f64> {
    sma(&d.close, 20).unwrap()
}
fn n_ema(d: &Data) -> Array1<f64> {
    ema(&d.close, 20).unwrap()
}
fn n_wma(d: &Data) -> Array1<f64> {
    wma(&d.close, 20).unwrap()
}
fn n_dema(d: &Data) -> Array1<f64> {
    dema(&d.close, 20).unwrap()
}
fn n_tema(d: &Data) -> Array1<f64> {
    tema(&d.close, 20).unwrap()
}
fn n_trima(d: &Data) -> Array1<f64> {
    trima(&d.close, 20).unwrap()
}
fn n_kama(d: &Data) -> Array1<f64> {
    // TA-Lib's KAMA defaults: period 30 with 2/30 fast/slow smoothing spans.
    finkit::math::fast_moving_avg::kama(&d.close, 20, 2, 30).unwrap()
}
fn n_rsi(d: &Data) -> Array1<f64> {
    rsi(&d.close, 14).unwrap()
}
fn n_atr(d: &Data) -> Array1<f64> {
    atr(&d.high, &d.low, &d.close, 14).unwrap()
}

#[cfg(feature = "talib-c")]
fn c_sma(d: &Data) -> Vec<f64> {
    call_single_in(TA_SMA, &d.close, 20)
}
#[cfg(feature = "talib-c")]
fn c_ema(d: &Data) -> Vec<f64> {
    call_single_in(TA_EMA, &d.close, 20)
}
#[cfg(feature = "talib-c")]
fn c_wma(d: &Data) -> Vec<f64> {
    call_single_in(TA_WMA, &d.close, 20)
}
#[cfg(feature = "talib-c")]
fn c_dema(d: &Data) -> Vec<f64> {
    call_single_in(TA_DEMA, &d.close, 20)
}
#[cfg(feature = "talib-c")]
fn c_tema(d: &Data) -> Vec<f64> {
    call_single_in(TA_TEMA, &d.close, 20)
}
#[cfg(feature = "talib-c")]
fn c_trima(d: &Data) -> Vec<f64> {
    call_single_in(TA_TRIMA, &d.close, 20)
}
#[cfg(feature = "talib-c")]
fn c_kama(d: &Data) -> Vec<f64> {
    call_single_in(TA_KAMA, &d.close, 20)
}
#[cfg(feature = "talib-c")]
fn c_rsi(d: &Data) -> Vec<f64> {
    call_single_in(TA_RSI, &d.close, 14)
}
#[cfg(feature = "talib-c")]
fn c_atr(d: &Data) -> Vec<f64> {
    call_hlc(TA_ATR, &d.high, &d.low, &d.close, 14)
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            label: "SMA(20)",
            source: "MA(CLOSE, 20)",
            native: n_sma,
            #[cfg(feature = "talib-c")]
            talib: c_sma,
        },
        Case {
            label: "EMA(20)",
            source: "EMA(CLOSE, 20)",
            native: n_ema,
            #[cfg(feature = "talib-c")]
            talib: c_ema,
        },
        Case {
            label: "WMA(20)",
            source: "WMA(CLOSE, 20)",
            native: n_wma,
            #[cfg(feature = "talib-c")]
            talib: c_wma,
        },
        Case {
            label: "DEMA(20)",
            source: "DEMA(CLOSE, 20)",
            native: n_dema,
            #[cfg(feature = "talib-c")]
            talib: c_dema,
        },
        Case {
            label: "TEMA(20)",
            source: "TEMA(CLOSE, 20)",
            native: n_tema,
            #[cfg(feature = "talib-c")]
            talib: c_tema,
        },
        Case {
            label: "TRIMA(20)",
            source: "TRIMA(CLOSE, 20)",
            native: n_trima,
            #[cfg(feature = "talib-c")]
            talib: c_trima,
        },
        Case {
            label: "KAMA(20)",
            source: "KAMA(CLOSE, 20)",
            native: n_kama,
            #[cfg(feature = "talib-c")]
            talib: c_kama,
        },
        Case {
            label: "RSI(14)",
            source: "RSI(CLOSE, 14)",
            native: n_rsi,
            #[cfg(feature = "talib-c")]
            talib: c_rsi,
        },
        Case {
            label: "ATR(14)",
            source: "ATR(HIGH, LOW, CLOSE, 14)",
            native: n_atr,
            #[cfg(feature = "talib-c")]
            talib: c_atr,
        },
    ]
}

/// Median of a small sample; avoids the outlier sensitivity of the mean when
/// the machine is shared.
fn median_us(samples: &mut [f64]) -> f64 {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    samples[samples.len() / 2]
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════════════");
    println!(" 公式引擎 / 原生直调 / TA-Lib C 执行效率对比");
    println!(" 同进程交错 {ROUNDS} 轮 × {REPEATS} 次，取每轮中位数");
    #[cfg(feature = "talib-c")]
    println!(" TA-Lib C 列：已启用 (--features talib-c)");
    #[cfg(not(feature = "talib-c"))]
    println!(" TA-Lib C 列：未启用（加 --features talib-c 以获取第四列）");
    println!("═══════════════════════════════════════════════════════════════════════");

    for len in LENGTHS {
        let data = make_data(len);
        println!("\n### {} bars\n", len);
        #[cfg(feature = "talib-c")]
        println!(
            "{:<12}{:>12}{:>12}{:>12}{:>12}{:>10}{:>10}",
            "指标", "formula", "planned", "native", "talib-C", "f/native", "f/talib"
        );
        #[cfg(not(feature = "talib-c"))]
        println!(
            "{:<12}{:>12}{:>12}{:>12}{:>10}",
            "指标", "formula", "planned", "native", "f/native"
        );
        println!("{}", "-".repeat(80));

        let mut sum_formula = 0.0;
        let mut sum_planned = 0.0;
        let mut sum_native = 0.0;
        #[cfg(feature = "talib-c")]
        let mut sum_talib = 0.0;
        let mut rows = 0usize;

        for case in cases() {
            let mut engine = FormulaEngine::new();
            let compiled = engine.compile(case.source).expect("compile formula");

            let mut formula_samples = Vec::with_capacity(ROUNDS);
            let mut planned_samples = Vec::with_capacity(ROUNDS);
            let mut native_samples = Vec::with_capacity(ROUNDS);
            #[cfg(feature = "talib-c")]
            let mut talib_samples = Vec::with_capacity(ROUNDS);

            // Warm-up: fill the plan/bytecode caches and touch the allocator
            // so the measured rounds do not include first-touch costs.
            for _ in 0..3 {
                let mut ctx = make_ctx(&data);
                let _ = engine.eval(case.source, &mut ctx);
                let mut ctx = make_ctx(&data);
                let _ = engine.execute(&compiled, &mut ctx);
                let _ = (case.native)(&data);
                #[cfg(feature = "talib-c")]
                let _ = (case.talib)(&data);
            }

            for round in 0..ROUNDS {
                // Alternate the order every round so a monotonic machine drift
                // cannot favour whichever implementation runs first.
                let forward = round % 2 == 0;
                let orders: [u8; 4] = if forward { [0, 1, 2, 3] } else { [3, 2, 1, 0] };
                for slot in orders {
                    match slot {
                        0 => {
                            let mut ctx = make_ctx(&data);
                            let start = std::time::Instant::now();
                            for _ in 0..REPEATS {
                                std::hint::black_box(
                                    engine.eval(case.source, &mut ctx).expect("eval"),
                                );
                            }
                            formula_samples
                                .push(start.elapsed().as_secs_f64() * 1e6 / REPEATS as f64);
                        }
                        1 => {
                            let mut ctx = make_ctx(&data);
                            let start = std::time::Instant::now();
                            for _ in 0..REPEATS {
                                std::hint::black_box(
                                    engine.execute(&compiled, &mut ctx).expect("execute"),
                                );
                            }
                            planned_samples
                                .push(start.elapsed().as_secs_f64() * 1e6 / REPEATS as f64);
                        }
                        2 => {
                            let start = std::time::Instant::now();
                            for _ in 0..REPEATS {
                                std::hint::black_box((case.native)(&data));
                            }
                            native_samples
                                .push(start.elapsed().as_secs_f64() * 1e6 / REPEATS as f64);
                        }
                        _ => {
                            #[cfg(feature = "talib-c")]
                            {
                                let start = std::time::Instant::now();
                                for _ in 0..REPEATS {
                                    std::hint::black_box((case.talib)(&data));
                                }
                                talib_samples
                                    .push(start.elapsed().as_secs_f64() * 1e6 / REPEATS as f64);
                            }
                        }
                    }
                }
            }

            let formula = median_us(&mut formula_samples);
            let planned = median_us(&mut planned_samples);
            let native = median_us(&mut native_samples);
            #[cfg(feature = "talib-c")]
            let talib = median_us(&mut talib_samples);

            sum_formula += formula;
            sum_planned += planned;
            sum_native += native;
            #[cfg(feature = "talib-c")]
            {
                sum_talib += talib;
            }
            rows += 1;

            #[cfg(feature = "talib-c")]
            println!(
                "{:<12}{:>10.2}µs{:>10.2}µs{:>10.2}µs{:>10.2}µs{:>9.2}x{:>9.2}x",
                case.label,
                formula,
                planned,
                native,
                talib,
                formula / native,
                talib / formula,
            );
            #[cfg(not(feature = "talib-c"))]
            println!(
                "{:<12}{:>10.2}µs{:>10.2}µs{:>10.2}µs{:>9.2}x",
                case.label,
                formula,
                planned,
                native,
                formula / native,
            );
        }

        #[cfg(feature = "talib-c")]
        println!(
            "{:<12}{:>10.2}µs{:>10.2}µs{:>10.2}µs{:>10.2}µs{:>9.2}x{:>9.2}x",
            "合计",
            sum_formula,
            sum_planned,
            sum_native,
            sum_talib,
            sum_formula / sum_native,
            sum_talib / sum_formula,
        );
        #[cfg(not(feature = "talib-c"))]
        println!(
            "{:<12}{:>10.2}µs{:>10.2}µs{:>10.2}µs{:>9.2}x",
            "合计",
            sum_formula,
            sum_planned,
            sum_native,
            sum_formula / sum_native,
        );
        let _ = rows;
    }

    println!("\n列含义：");
    println!("  formula = FormulaEngine::eval（用户源码入口，含编译缓存查询）");
    println!("  planned = 已编译公式 execute（去掉编译/缓存步骤后的纯执行）");
    println!("  native  = 直接调用 Rust 指标函数（无解析、无分派）");
    println!("  f/native = 引擎相对原生直调的开销倍数（越接近 1 越好）");
    println!("  f/talib  = TA-Lib C 耗时 / 公式引擎耗时（>1 表示引擎更快）");
}
