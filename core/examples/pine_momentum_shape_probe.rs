use finkit::formula::{
    FormulaContext, FormulaDialect, FormulaEngine, FormulaExecutionMode, ParamValues,
};
use ndarray::Array1;
use std::collections::BTreeSet;

fn context(len: usize) -> FormulaContext {
    let close: Vec<_> = (0..len)
        .map(|i| 100.0 + i as f64 * 0.006 + (i as f64 * 0.017).sin() * 2.1)
        .collect();
    let open: Vec<_> = close.iter().map(|v| v - 0.15).collect();
    let high: Vec<_> = close.iter().map(|v| v + 0.8).collect();
    let low: Vec<_> = close.iter().map(|v| v - 0.8).collect();
    let volume = vec![1_000_000.0; len];
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
    let source = include_str!("../../tests/pine_corpus/momentum.pine");
    let mut tree_context = context(5_000);
    let mut plan_context = context(5_000);
    let mut tree = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Tree);
    let mut plan = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
    let tree_result = tree
        .eval_with_dialect(source, FormulaDialect::Pine, &mut tree_context)
        .unwrap();
    let plan_result = plan
        .eval_with_dialect(source, FormulaDialect::Pine, &mut plan_context)
        .unwrap();
    assert_eq!(tree_result.len(), plan_result.len());
    for (i, (a, b)) in tree_result.iter().zip(plan_result.iter()).enumerate() {
        assert!(
            if a.is_nan() {
                b.is_nan()
            } else {
                a.to_bits() == b.to_bits()
            },
            "result {i}: {a} != {b}"
        );
    }
    let tree_names: BTreeSet<_> = tree_context
        .variables
        .keys()
        .map(|key| key.as_ref().to_owned())
        .collect();
    let plan_names: BTreeSet<_> = plan_context
        .variables
        .keys()
        .map(|key| key.as_ref().to_owned())
        .collect();
    assert_eq!(tree_names, plan_names, "published variable names");
    for name in &tree_names {
        let a = tree_context.variables.get(name.as_str()).unwrap();
        let b = plan_context.variables.get(name.as_str()).unwrap();
        assert_eq!(a.len(), b.len(), "variable length {name}");
        for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
            assert!(
                if x.is_nan() {
                    y.is_nan()
                } else {
                    x.to_bits() == y.to_bits()
                },
                "variable {name}[{i}]: {x} != {y}"
            );
        }
    }
    assert_eq!(
        tree_context.output_names, plan_context.output_names,
        "output names"
    );
    assert_eq!(
        tree_context.output_modifiers.len(),
        plan_context.output_modifiers.len(),
        "output modifier count"
    );
    for (name, modifier) in &tree_context.output_modifiers {
        assert_eq!(
            format!("{modifier:?}"),
            format!("{:?}", plan_context.output_modifiers.get(name).unwrap()),
            "output modifier {name}"
        );
    }
    println!(
        "Pine Momentum primary and variables match exactly: {} variables, outputs={:?}",
        tree_names.len(),
        plan_context.output_names
    );

    let engine = FormulaEngine::new();
    let plan = engine
        .compile_plan(source, FormulaDialect::Pine, &ParamValues::new())
        .unwrap();
    println!(
        "shape inputs={} outputs={} vars={} layout={:?}",
        plan.input_bindings().len(),
        plan.outputs().len(),
        plan.variables().len(),
        plan.hot().output_layout().outputs()
    );
    for x in plan.input_bindings() {
        println!("input {} slot {}", x.name(), x.slot().0);
    }
    for x in plan.outputs() {
        println!(
            "output {} slot {} level={}",
            x.name(),
            x.slot().0,
            x.is_level_marker()
        );
    }
    for x in plan.variables() {
        println!("var {} slot {}", x.name(), x.slot().0);
    }
}
