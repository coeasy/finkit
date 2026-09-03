use finkit::formula::{parse_formula, FormulaContext, FormulaEngine, FormulaExecutor};
use ndarray::Array1;
use std::collections::BTreeSet;

fn make_context(len: usize) -> FormulaContext {
    let open = Array1::from_vec((0..len).map(|i| 100.0 + i as f64 * 0.3).collect());
    let high = Array1::from_vec((0..len).map(|i| 102.0 + i as f64 * 0.4).collect());
    let low = Array1::from_vec((0..len).map(|i| 98.0 + i as f64 * 0.2).collect());
    let close = Array1::from_vec(
        (0..len)
            .map(|i| 100.0 + i as f64 * 0.35 + ((i % 7) as f64 - 3.0) * 0.2)
            .collect(),
    );
    let volume = Array1::from_vec(
        (0..len)
            .map(|i| 10_000.0 + i as f64 * 75.0 + (i % 5) as f64 * 20.0)
            .collect(),
    );
    FormulaContext::new(open, high, low, close, volume, None)
}

fn assert_array_equivalent(left: &Array1<f64>, right: &Array1<f64>, label: &str) {
    assert_eq!(left.len(), right.len(), "length mismatch for {label}");
    for (index, (&lhs, &rhs)) in left.iter().zip(right).enumerate() {
        if lhs.is_nan() && rhs.is_nan() {
            continue;
        }
        let scale = lhs.abs().max(rhs.abs()).max(1.0);
        let error = (lhs - rhs).abs();
        assert!(
            error <= 1e-10 * scale,
            "value mismatch for {label} at {index}: {lhs} vs {rhs} (error {error})"
        );
    }
}

fn assert_execution_equivalent(source: &str) {
    let ast = parse_formula(source).unwrap();
    let executor = FormulaExecutor::new();
    let mut raw_context = make_context(96);
    let raw_result = executor.execute(&ast, &mut raw_context).unwrap();

    let mut engine = FormulaEngine::new();
    let compiled = engine.compile(source).unwrap();
    let mut optimized_context = make_context(96);
    let optimized_result = engine.execute(&compiled, &mut optimized_context).unwrap();

    assert_array_equivalent(&raw_result, &optimized_result, "formula result");

    let raw_names: BTreeSet<String> = raw_context
        .variables
        .keys()
        .map(|name| name.to_string())
        .collect();
    let optimized_names: BTreeSet<String> = optimized_context
        .variables
        .keys()
        .map(|name| name.to_string())
        .collect();
    assert_eq!(
        raw_names, optimized_names,
        "optimizer changed observable assignment/output variables"
    );

    for name in raw_names {
        assert_array_equivalent(
            &raw_context.variables[name.as_str()],
            &optimized_context.variables[name.as_str()],
            &format!("variable {name}"),
        );
    }
}

#[test]
fn execution_optimizer_preserves_bollinger_assignments() {
    assert_execution_equivalent(
        "MID:=MA(CLOSE,20);STD_VAL:=STD(CLOSE,20);UPPER:=MID+2*STD_VAL;LOWER:=MID-2*STD_VAL;UPPER;",
    );
}

#[test]
fn execution_optimizer_preserves_rsi_side_variables() {
    assert_execution_equivalent(
        "CHANGE:=CLOSE-REF(CLOSE,1);UP:=IF(CHANGE>0,CHANGE,0);DOWN:=IF(CHANGE<0,-CHANGE,0);RS:=SUM(UP,6)/SUM(DOWN,6);OVERBOUGHT:=RS>3;OVERSOLD:=RS<0.333333333333;OVERBOUGHT;",
    );
}

#[test]
fn execution_optimizer_preserves_buy_and_sell_assignments() {
    assert_execution_equivalent(
        "FAST:=MA(CLOSE,5);SLOW:=MA(CLOSE,20);BUY:=CROSS(FAST,SLOW);SELL:=CROSS(SLOW,FAST);BUY;",
    );
}

#[test]
fn execution_optimizer_preserves_named_outputs() {
    assert_execution_equivalent("MA5:=MA(CLOSE,5);UPPER:MA5+2;LOWER:MA5-2;UPPER;");
}
