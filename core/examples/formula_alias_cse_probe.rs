//! Focused check for CSE across equivalent OHLCV variable aliases.

use finkit::formula::{
    parse_formula, FormulaContext, FormulaDialect, FormulaEngine, FormulaExecutionMode,
    FormulaHotPlan,
};
use ndarray::Array1;
use std::hint::black_box;
use std::time::Instant;

const SOURCE: &str = "振幅H:=(H-L)/C*100;振幅FULL:=(HIGH-LOW)/CLOSE*100;差值:=振幅H-振幅FULL;";
const OUTPUTS: [&str; 3] = ["振幅H", "振幅FULL", "差值"];

fn context() -> FormulaContext {
    let n = 250;
    let close: Vec<f64> = (0..n)
        .map(|index| 100.0 + index as f64 * 0.17 + (index as f64 * 0.13).sin())
        .collect();
    let high: Vec<f64> = close.iter().map(|value| value + 1.5).collect();
    let low: Vec<f64> = close.iter().map(|value| value - 2.5).collect();
    let open = close.clone();
    let volume = vec![1000.0; n];
    FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
        None,
    )
}

fn evaluate(mode: FormulaExecutionMode) -> (Vec<f64>, Vec<(String, Vec<f64>)>) {
    let mut context = context();
    let mut engine = FormulaEngine::new().with_execution_mode(mode);
    let primary = engine
        .eval_with_dialect(SOURCE, FormulaDialect::TongDaXin, &mut context)
        .expect("alias formula evaluates")
        .to_vec();
    let outputs = OUTPUTS
        .iter()
        .map(|name| {
            (
                (*name).to_string(),
                context
                    .get_variable(name)
                    .expect("named assignment is published")
                    .to_vec(),
            )
        })
        .collect();
    (primary, outputs)
}

fn warm_time(mode: FormulaExecutionMode) -> f64 {
    let mut context = context();
    let mut engine = FormulaEngine::new().with_execution_mode(mode);
    for _ in 0..5 {
        black_box(
            engine
                .eval_with_dialect(SOURCE, FormulaDialect::TongDaXin, &mut context)
                .expect("alias formula evaluates"),
        );
    }
    let started = Instant::now();
    for _ in 0..100 {
        black_box(
            engine
                .eval_with_dialect(SOURCE, FormulaDialect::TongDaXin, &mut context)
                .expect("alias formula evaluates"),
        );
    }
    started.elapsed().as_secs_f64() * 1e6 / 100.0
}

fn main() {
    let ast = parse_formula(SOURCE).expect("alias formula parses");
    let plan = FormulaHotPlan::compile(&ast).expect("alias formula compiles");
    let (tree_primary, tree_outputs) = evaluate(FormulaExecutionMode::Tree);
    let (plan_primary, plan_outputs) = evaluate(FormulaExecutionMode::Plan);
    let exact = tree_primary == plan_primary
        && tree_outputs == plan_outputs
        && tree_outputs
            .iter()
            .all(|(_, values)| values.iter().all(|value| value.is_finite()));
    assert!(
        exact,
        "Tree and Plan primary/named values must match exactly"
    );

    let mut tree_runs = Vec::with_capacity(5);
    let mut plan_runs = Vec::with_capacity(5);
    for _ in 0..5 {
        tree_runs.push(warm_time(FormulaExecutionMode::Tree));
        plan_runs.push(warm_time(FormulaExecutionMode::Plan));
    }
    tree_runs.sort_by(f64::total_cmp);
    plan_runs.sort_by(f64::total_cmp);
    println!("formula={SOURCE}");
    println!("hot_plan_nodes={}", plan.hot().nodes().len());
    println!("exact_primary_and_named_output_parity={exact}");
    println!("tree_warm_median_us={:.3}", tree_runs[2]);
    println!("plan_warm_median_us={:.3}", plan_runs[2]);
}
