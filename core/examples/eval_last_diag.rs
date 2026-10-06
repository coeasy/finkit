use finkit::formula::{FormulaContext, FormulaEngine};
use ndarray::Array1;

fn make_ctx(len: usize) -> FormulaContext {
    let close: Vec<f64> = (0..len)
        .map(|i| {
            let t = i as f64;
            100.0 + t * 0.01 + (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5
        })
        .collect();
    let high: Vec<f64> = close.iter().map(|c| c + 1.0).collect();
    let low: Vec<f64> = close.iter().map(|c| c - 1.0).collect();
    let open: Vec<f64> = close.iter().map(|c| c - 0.3).collect();
    let volume: Vec<f64> = close.iter().map(|_| 10_000.0).collect();
    FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
        None,
    )
}

fn main() {
    let mut engine = FormulaEngine::new();
    let compiled = engine.compile("RSI(CLOSE, 14)").unwrap();

    println!("=== same-length repeated eval_last (identical data) ===");
    for i in 0..8 {
        let ctx = make_ctx(250);
        let v = engine.eval_last(&compiled, &ctx).unwrap();
        println!("  call {i}: {v}");
    }

    println!("\n=== single ctx, repeated eval_last ===");
    let ctx = make_ctx(250);
    for i in 0..4 {
        let v = engine.eval_last(&compiled, &ctx).unwrap();
        println!("  call {i}: {v}");
    }

    println!("\n=== growing ctx (genuine append) ===");
    let mut e2 = FormulaEngine::new();
    let c2 = e2.compile("RSI(CLOSE, 14)").unwrap();
    for len in 245..=255 {
        let ctx = make_ctx(len);
        let v = e2.eval_last(&c2, &ctx).unwrap();
        println!("  len {len}: {v}");
    }

    println!("\n=== MA(CLOSE,20) same-length repeat ===");
    let mut e3 = FormulaEngine::new();
    let c3 = e3.compile("MA(CLOSE, 20)").unwrap();
    for i in 0..6 {
        let ctx = make_ctx(250);
        let v = e3.eval_last(&c3, &ctx).unwrap();
        println!("  call {i}: {v}");
    }
}
