use finkit::formula::{FormulaContext, FormulaDialect, FormulaEngine, FormulaExecutionMode};
use ndarray::Array1;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn context(len: usize) -> FormulaContext {
    let mut seed: u64 = 0x1234_5678_AB_CD_EF_00;
    let mut rng = || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 33) as f64 / (u64::MAX as f64)
    };
    let (mut open, mut high, mut low, mut close, mut volume) = (
        Vec::with_capacity(len),
        Vec::with_capacity(len),
        Vec::with_capacity(len),
        Vec::with_capacity(len),
        Vec::with_capacity(len),
    );
    let mut previous_close = 100.0;
    for _ in 0..len {
        let delta = (rng() - 0.5) * 4.0;
        let current_close = previous_close + delta;
        let current_open = current_close - delta * 0.5;
        high.push(current_close.max(current_open) + rng() * 2.0 + 0.01);
        low.push(current_close.min(current_open) - rng() * 2.0 - 0.01);
        open.push(current_open);
        close.push(current_close);
        volume.push(1000.0 + rng() * 500.0);
        previous_close = current_close;
    }
    let mut context = FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
        None,
    );
    let hl2 = (0..len)
        .map(|i| (context.high[i] + context.low[i]) / 2.0)
        .collect();
    let hlc3 = (0..len)
        .map(|i| (context.high[i] + context.low[i] + context.close[i]) / 3.0)
        .collect();
    let ohlc4 = (0..len)
        .map(|i| (context.open[i] + context.high[i] + context.low[i] + context.close[i]) / 4.0)
        .collect();
    context.set_variable("HL2".to_string(), Array1::from_vec(hl2));
    context.set_variable("HLC3".to_string(), Array1::from_vec(hlc3));
    context.set_variable("OHLC4".to_string(), Array1::from_vec(ohlc4));
    context
}

fn same_series(left: &[f64], right: &[f64]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(&a, &b)| (a.is_nan() && b.is_nan()) || a == b || (a - b).abs() <= 1e-10)
}

fn eval(mode: FormulaExecutionMode, source: &str, len: usize) -> (Vec<f64>, FormulaContext) {
    let mut context = context(len);
    let mut engine = FormulaEngine::new().with_execution_mode(mode);
    let primary = engine
        .eval_with_dialect(source, FormulaDialect::Pine, &mut context)
        .unwrap()
        .to_vec();
    (primary, context)
}

fn warm_time(mode: FormulaExecutionMode, source: &str, len: usize) -> f64 {
    let mut context = context(len);
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
    let source = include_str!("../../tests/pine_corpus/roc.pine");
    for len in [120usize, 2_400] {
        let (tree_primary, tree_context) = eval(FormulaExecutionMode::Tree, source, len);
        let (plan_primary, plan_context) = eval(FormulaExecutionMode::Plan, source, len);
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
        assert!(context_equal, "published ROC context differs at n={len}");

        let mut tree_times = (0..5)
            .map(|_| warm_time(FormulaExecutionMode::Tree, source, len))
            .collect::<Vec<_>>();
        let mut plan_times = (0..5)
            .map(|_| warm_time(FormulaExecutionMode::Plan, source, len))
            .collect::<Vec<_>>();
        tree_times.sort_by(f64::total_cmp);
        plan_times.sort_by(f64::total_cmp);
        println!(
            "Pine ROC n={len}: parity=true tree={:.2}us plan={:.2}us speedup={:.3}x",
            tree_times[2],
            plan_times[2],
            tree_times[2] / plan_times[2]
        );
    }
}
