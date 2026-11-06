use finkit::formula::{FormulaContext, FormulaDialect, FormulaEngine, FormulaExecutionMode};
use ndarray::Array1;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn columns(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let close: Vec<_> = (0..len)
        .map(|i| {
            let x = i as f64;
            100.0 + 0.006 * x + (x * 0.017).sin() * 2.1
        })
        .collect();
    let open = close.iter().map(|value| value - 0.15).collect();
    let high = close.iter().map(|value| value + 0.7).collect();
    let low = close.iter().map(|value| value - 0.8).collect();
    let volume = vec![1_000_000.0; len];
    (open, high, low, close, volume)
}

fn context(c: &(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)) -> FormulaContext {
    FormulaContext::new(
        Array1::from_vec(c.0.clone()),
        Array1::from_vec(c.1.clone()),
        Array1::from_vec(c.2.clone()),
        Array1::from_vec(c.3.clone()),
        Array1::from_vec(c.4.clone()),
        None,
    )
}

fn same_series(left: &[f64], right: &[f64]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(&a, &b)| (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits())
}

fn eval(
    mode: FormulaExecutionMode,
    source: &str,
    c: &(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>),
) -> (Vec<f64>, FormulaContext) {
    let mut context = context(c);
    let mut engine = FormulaEngine::new().with_execution_mode(mode);
    let primary = engine
        .eval_with_dialect(source, FormulaDialect::Pine, &mut context)
        .unwrap()
        .to_vec();
    (primary, context)
}

fn warm_time(
    mode: FormulaExecutionMode,
    source: &str,
    c: &(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>),
) -> f64 {
    let mut context = context(c);
    let mut engine = FormulaEngine::new().with_execution_mode(mode);
    for _ in 0..5 {
        black_box(
            engine
                .eval_with_dialect(source, FormulaDialect::Pine, &mut context)
                .unwrap(),
        );
    }
    let start = Instant::now();
    let mut count = 0_u64;
    while start.elapsed() < Duration::from_millis(120) {
        black_box(
            engine
                .eval_with_dialect(source, FormulaDialect::Pine, &mut context)
                .unwrap(),
        );
        count += 1;
    }
    start.elapsed().as_secs_f64() * 1e6 / count as f64
}

fn main() {
    let source = include_str!("../../tests/pine_corpus/cci.pine");
    for len in [250usize, 5_000] {
        let c = columns(len);
        let (tree_primary, tree_context) = eval(FormulaExecutionMode::Tree, source, &c);
        let (plan_primary, plan_context) = eval(FormulaExecutionMode::Plan, source, &c);
        let mut tree_variables = tree_context.variables.iter().collect::<Vec<_>>();
        let mut plan_variables = plan_context.variables.iter().collect::<Vec<_>>();
        tree_variables.sort_by(|a, b| a.0.cmp(b.0));
        plan_variables.sort_by(|a, b| a.0.cmp(b.0));
        let variables_equal = tree_variables.len() == plan_variables.len()
            && tree_variables.iter().zip(&plan_variables).all(
                |((tree_name, tree), (plan_name, plan))| {
                    tree_name == plan_name
                        && same_series(tree.as_slice().unwrap(), plan.as_slice().unwrap())
                },
            );
        let context_equal = variables_equal
            && tree_context.output_names == plan_context.output_names
            && tree_context.output_modifiers == plan_context.output_modifiers
            && tree_context.string_table == plan_context.string_table;
        assert!(
            same_series(&tree_primary, &plan_primary),
            "primary differs at n={len}"
        );
        assert!(context_equal, "published CCI context differs at n={len}");
        let mut tree_times = (0..5)
            .map(|_| warm_time(FormulaExecutionMode::Tree, source, &c))
            .collect::<Vec<_>>();
        let mut plan_times = (0..5)
            .map(|_| warm_time(FormulaExecutionMode::Plan, source, &c))
            .collect::<Vec<_>>();
        tree_times.sort_by(f64::total_cmp);
        plan_times.sort_by(f64::total_cmp);
        println!(
            "Pine CCI n={len}: parity=true tree={:.2}us plan={:.2}us speedup={:.3}x",
            tree_times[2],
            plan_times[2],
            tree_times[2] / plan_times[2]
        );
    }
}
