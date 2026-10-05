//! How much of a plan-path `eval_plan` call is *cache overhead* rather than
//! execution, and why the `FormulaPlanCache` hands out `Arc` handles.
//!
//! Run: `cargo run --release -p finkit --example plan_cache_probe`
//!
//! The third column is the cost of the **public** `compile_plan` entry point,
//! which still clones by design — it returns an owned plan to its caller. The
//! internal execution path does not pay it; that is the whole point of the
//! measurement. Compare `eval_plan` against `cold` to see what the cache is
//! worth (~1.7 ms to compile, i.e. >200x at screening length).
//!
//! Screening evaluates one formula across thousands of short series, so the
//! 250-bar block is the row that matters in production.

use finkit::formula::{FormulaContext, FormulaEngine};
use ndarray::Array1;
use std::time::Instant;

const ITERS: usize = 300;
const ROUNDS: usize = 5;

/// Minimum of `ROUNDS` timing rounds.
///
/// These are sub-microsecond measurements taken in one process, so the mean is
/// contaminated by turbo/frequency drift and by whichever allocator state the
/// previous block happened to leave behind. The minimum is the stable estimator:
/// it is the run that had the least interference, which is what we are trying
/// to compare.
fn time_best<F: FnMut()>(mut f: F) -> f64 {
    (0..ROUNDS)
        .map(|_| time(&mut f))
        .fold(f64::INFINITY, f64::min)
}

fn data(
    len: usize,
) -> (
    Array1<f64>,
    Array1<f64>,
    Array1<f64>,
    Array1<f64>,
    Array1<f64>,
) {
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut volume = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f64;
        let noise = (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5;
        let price = 100.0 + t * 0.01 + noise;
        open.push(price - 0.3);
        high.push(price + 1.0);
        low.push(price - 1.0);
        close.push(price);
        volume.push(10_000.0 + (t * 10.0).sin() * 3_000.0);
    }
    (
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
    )
}

fn time<F: FnMut()>(mut f: F) -> f64 {
    for _ in 0..20 {
        f();
    }
    let start = Instant::now();
    for _ in 0..ITERS {
        f();
    }
    start.elapsed().as_secs_f64() * 1e6 / ITERS as f64
}

fn main() {
    let formulas = [
        ("MACD-ish", "EMA(CLOSE,12)-EMA(CLOSE,26)"),
        ("RSI", "RSI(CLOSE,14)"),
        ("BOLL", "(CLOSE-MA(CLOSE,20))/STD(CLOSE,20)*100"),
        (
            "COMPOUND",
            "MA5:=MA(CLOSE,5); MA20:=MA(CLOSE,20); OUT: MA5-MA20;",
        ),
    ];

    for &len in &[250usize, 2_000, 10_000] {
        println!("--- series length {len} ---");
        for (name, src) in formulas {
            let (o, h, l, c, v) = data(len);
            let mut ctx = FormulaContext::new(o.clone(), h, l, c, v, None);
            let engine = FormulaEngine::new();
            engine
                .eval_plan(src, &mut ctx)
                .expect("warm-up eval failed");

            let hit = time_best(|| {
                engine
                    .compile_plan_default(src)
                    .expect("cached plan lookup failed");
            });
            let total = time_best(|| {
                engine.eval_plan(src, &mut ctx).expect("eval failed");
            });
            let cold = time(|| {
                let fresh = FormulaEngine::new();
                fresh.eval_plan(src, &mut ctx).expect("cold eval failed");
            });
            println!(
                "{name:>9} | public compile_plan {hit:8.2} us | eval_plan {total:9.2} us \
                 | hit overhead {:5.1}% | cold(compile+eval) {cold:9.2} us",
                hit / total * 100.0
            );

            // The tree-walker is the *default* backend, and its cache
            // (`FormulaCache`) clones the compiled AST on every hit through
            // `get_cloned`. Measured here so the cost is on record.
            let mut engine = FormulaEngine::new();
            engine.eval(src, &mut ctx).expect("tree warm-up failed");
            let tree = time_best(|| {
                engine.eval(src, &mut ctx).expect("tree eval failed");
            });
            println!("{name:>9} | tree eval (default backend) {tree:9.2} us");
        }
    }
}
