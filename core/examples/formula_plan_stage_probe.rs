//! Separates FormulaEngine Plan entry overhead from a precompiled executor run.

use finkit::formula::{
    parse_formula, unified_formula_executor, FormulaContext, FormulaDialect, FormulaEngine,
    FormulaExecutionMode, FormulaHotPlan,
};
use ndarray::Array1;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn columns(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let close: Vec<_> = (0..len)
        .map(|index| {
            let x = index as f64;
            100.0 + 0.006 * x + (x * 0.017).sin() * 2.1
        })
        .collect();
    let open: Vec<_> = close.iter().map(|value| value - 0.15).collect();
    let high: Vec<_> = close.iter().map(|value| value + 0.7).collect();
    let low: Vec<_> = close.iter().map(|value| value - 0.8).collect();
    let volume = vec![1_000_000.0; len];
    (open, high, low, close, volume)
}

fn context(columns: &(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)) -> FormulaContext {
    let (open, high, low, close, volume) = columns;
    FormulaContext::new(
        Array1::from_vec(open.clone()),
        Array1::from_vec(high.clone()),
        Array1::from_vec(low.clone()),
        Array1::from_vec(close.clone()),
        Array1::from_vec(volume.clone()),
        None,
    )
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn time_per_call(mut operation: impl FnMut(), duration: Duration) -> f64 {
    let started = Instant::now();
    let mut calls = 0_u64;
    while started.elapsed() < duration {
        operation();
        calls += 1;
    }
    started.elapsed().as_secs_f64() * 1e6 / calls as f64
}

fn measure(source: &str, len: usize, columns: &(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)) {
    let ast = parse_formula(source).expect("formula parses");
    let plan = FormulaHotPlan::compile(&ast).expect("formula plan compiles");
    let mut inputs = vec![&columns.3[..]; plan.input_bindings().len()];
    for binding in plan.input_bindings() {
        let values = match binding.name().to_ascii_uppercase().as_str() {
            "OPEN" => &columns.0,
            "HIGH" => &columns.1,
            "LOW" => &columns.2,
            "CLOSE" => &columns.3,
            "VOLUME" => &columns.4,
            name => panic!("unexpected input {name}"),
        };
        inputs[binding.slot().0] = values;
    }

    let mut executor = unified_formula_executor(&plan);
    let mut primary_executor = unified_formula_executor(&plan);
    let mut direct_samples = [0.0; 7];
    let mut direct_primary_samples = [0.0; 7];
    for sample in &mut direct_samples {
        *sample = time_per_call(
            || {
                black_box(executor.execute(black_box(&inputs)).unwrap());
            },
            Duration::from_millis(100),
        );
    }
    for sample in &mut direct_primary_samples {
        *sample = time_per_call(
            || {
                black_box(
                    primary_executor
                        .execute_primary(black_box(&inputs))
                        .unwrap(),
                );
            },
            Duration::from_millis(100),
        );
    }

    let mut ctx = context(columns);
    let mut engine = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
    let mut engine_samples = [0.0; 7];
    for sample in &mut engine_samples {
        *sample = time_per_call(
            || {
                black_box(
                    engine
                        .eval_with_dialect(source, FormulaDialect::TongDaXin, &mut ctx)
                        .unwrap(),
                );
            },
            Duration::from_millis(100),
        );
    }

    let mut tree = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Tree);
    let mut tree_samples = [0.0; 7];
    for sample in &mut tree_samples {
        *sample = time_per_call(
            || {
                black_box(
                    tree.eval_with_dialect(source, FormulaDialect::TongDaXin, &mut ctx)
                        .unwrap(),
                );
            },
            Duration::from_millis(100),
        );
    }

    let direct_us = median(&mut direct_samples);
    let direct_primary_us = median(&mut direct_primary_samples);
    let engine_us = median(&mut engine_samples);
    let tree_us = median(&mut tree_samples);
    println!(
        "formula={source:?} n={len} hot_nodes={} outputs={} direct={direct_us:.3}us direct_primary={direct_primary_us:.3}us engine_plan={engine_us:.3}us tree={tree_us:.3}us plan_tree={:.3}x wrapper_delta={:.3}us ratio={:.3}x",
        plan.hot().nodes().len(),
        plan.hot().output_layout().len(),
        engine_us / tree_us,
        engine_us - direct_us,
        engine_us / direct_us,
    );
}

fn main() {
    for len in [250, 5_000] {
        let columns = columns(len);
        for source in [
            "MA(CLOSE,14)",
            "ATR(HIGH,LOW,CLOSE,14)",
            "RSI(CLOSE,14)",
            "CLOSE*2+1",
            "SMA(CLOSE,14)+ROC(CLOSE,10)",
            "ZLEMA20:=ZLEMA(CLOSE,20);",
            "CPK:=COPPOCK(CLOSE,10,14,11);",
        ] {
            measure(source, len, &columns);
        }
    }
}
