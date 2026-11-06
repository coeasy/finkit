use finkit::formula::{
    parse_formula, FormulaContext, FormulaDialect, FormulaEngine, FormulaExecutionMode,
    FormulaHotPlan, ParamValues,
};
use ndarray::Array1;

fn main() {
    let ast = parse_formula("MA(CLOSE,14)").unwrap();
    let literal_plan = FormulaHotPlan::compile(&ast).unwrap();
    println!(
        "[ma-literal] hot_nodes={}",
        literal_plan.hot().nodes().len()
    );
    for node in literal_plan.hot().nodes() {
        println!(
            "  kernel={:#018x} params={:?}",
            node.kernel.0,
            literal_plan.hot().parameter_arena().range(node.parameters)
        );
    }

    let ast = parse_formula("CPK:=COPPOCK(CLOSE,10,14,11);").unwrap();
    let coppock_plan = FormulaHotPlan::compile(&ast).unwrap();
    println!(
        "[coppock-assignment] semantic_nodes={} hot_nodes={} inputs={} outputs={} vars={}",
        coppock_plan.semantic().plan().len(),
        coppock_plan.hot().nodes().len(),
        coppock_plan.input_bindings().len(),
        coppock_plan.outputs().len(),
        coppock_plan.variables().len()
    );
    for &id in coppock_plan.semantic().plan().execution_order() {
        let node = coppock_plan.semantic().plan().node(id).unwrap();
        println!(
            "  semantic={} id={} dependencies={:?}",
            node.operation,
            id.0,
            node.dependencies
                .iter()
                .map(|dependency| dependency.0)
                .collect::<Vec<_>>(),
        );
    }

    let ast = parse_formula("ZLEMA20:=ZLEMA(CLOSE,20);").unwrap();
    let zlema_plan = FormulaHotPlan::compile(&ast).unwrap();
    println!(
        "[zlema-assignment] root={} semantic_nodes={} hot_nodes={} inputs={} outputs={} vars={}",
        zlema_plan.semantic().root().0,
        zlema_plan.semantic().plan().len(),
        zlema_plan.hot().nodes().len(),
        zlema_plan.input_bindings().len(),
        zlema_plan.outputs().len(),
        zlema_plan.variables().len()
    );
    for &id in zlema_plan.semantic().plan().execution_order() {
        let node = zlema_plan.semantic().plan().node(id).unwrap();
        println!(
            "  semantic={} id={} dependencies={:?}",
            node.operation,
            id.0,
            node.dependencies
                .iter()
                .map(|dependency| dependency.0)
                .collect::<Vec<_>>(),
        );
    }
    let close = Array1::from_vec(
        (0..256)
            .map(|index| 100.0 + (index as f64 * 0.03).sin() * 4.0 + index as f64 * 0.02)
            .collect(),
    );
    let context = || {
        FormulaContext::new(
            close.clone(),
            close.clone(),
            close.clone(),
            close.clone(),
            Array1::from_elem(close.len(), 1000.0),
            None,
        )
    };
    let mut tree_context = context();
    let mut plan_context = context();
    let mut tree = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Tree);
    let mut plan = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
    let tree_values = tree
        .eval("ZLEMA20:=ZLEMA(CLOSE,20);", &mut tree_context)
        .unwrap();
    let plan_values = plan
        .eval("ZLEMA20:=ZLEMA(CLOSE,20);", &mut plan_context)
        .unwrap();
    let tree_variable = tree_context.variables.get("ZLEMA20").unwrap();
    let plan_variable = plan_context.variables.get("ZLEMA20").unwrap();
    let values_equal = tree_values
        .iter()
        .zip(plan_values.iter())
        .all(|(left, right)| left == right || (left.is_nan() && right.is_nan()));
    let variables_equal = tree_variable
        .iter()
        .zip(plan_variable.iter())
        .all(|(left, right)| left == right || (left.is_nan() && right.is_nan()));
    println!(
        "[zlema-assignment-parity] result_equal={values_equal} variable_equal={variables_equal}"
    );

    let engine = FormulaEngine::new();
    for (name, source) in [
        ("atr", include_str!("../../tests/pine_corpus/atr.pine")),
        ("cci", include_str!("../../tests/pine_corpus/cci.pine")),
        (
            "sma_overlay",
            include_str!("../../tests/pine_corpus/sma_overlay.pine"),
        ),
        ("obv", include_str!("../../tests/pine_corpus/obv.pine")),
        ("vwma", include_str!("../../tests/pine_corpus/vwma.pine")),
        (
            "williams_r",
            include_str!("../../tests/pine_corpus/williams_r.pine"),
        ),
        ("vwap", include_str!("../../tests/pine_corpus/vwap.pine")),
        ("roc", include_str!("../../tests/pine_corpus/roc.pine")),
        (
            "momentum",
            include_str!("../../tests/pine_corpus/momentum.pine"),
        ),
        ("trix", include_str!("../../tests/pine_corpus/trix.pine")),
    ] {
        let plan = engine
            .compile_plan(source, FormulaDialect::Pine, &ParamValues::new())
            .unwrap();
        println!(
            "[{name}] hot_nodes={} inputs={} outputs={} vars={}",
            plan.hot().nodes().len(),
            plan.input_bindings().len(),
            plan.outputs().len(),
            plan.variables().len()
        );
        for &id in plan.semantic().plan().execution_order() {
            let node = plan.semantic().plan().node(id).unwrap();
            println!(
                "  semantic {} id={} dependencies={:?}",
                node.operation,
                id.0,
                node.dependencies
                    .iter()
                    .map(|dependency| dependency.0)
                    .collect::<Vec<_>>(),
            );
        }
        println!("  output layout:");
        for (index, (node, slot)) in plan.hot().output_layout().outputs().iter().enumerate() {
            println!("    index={index} node={} slot={}", node.0, slot.0);
        }
        for variable in plan.variables() {
            println!(
                "  variable {} slot={} output_index={}",
                variable.name(),
                variable.slot().0,
                plan.hot()
                    .output_layout()
                    .outputs()
                    .iter()
                    .position(|(_, slot)| *slot == variable.slot())
                    .unwrap()
            );
        }
        for node in plan.hot().nodes() {
            let semantic = plan.semantic().plan().node(node.node).unwrap();
            println!(
                "  {} kernel={:#018x} inputs={:?} output={} params={:?}",
                semantic.operation,
                node.kernel.0,
                node.inputs.iter().map(|slot| slot.0).collect::<Vec<_>>(),
                node.output.0,
                plan.hot().parameter_arena().range(node.parameters),
            );
        }
    }

    let vwap_source = include_str!("../../tests/pine_corpus/vwap.pine");
    let vwap_context = || {
        let high = Array1::from_vec(
            (0..256)
                .map(|index| 101.0 + index as f64 * 0.02 + (index as f64 * 0.07).sin())
                .collect(),
        );
        let low = high.mapv(|value| value - 1.3);
        let close = high.mapv(|value| value - 0.4);
        let open = high.mapv(|value| value - 0.7);
        let volume = Array1::from_vec(
            (0..256)
                .map(|index| 500.0 + (index % 17) as f64 * 23.0)
                .collect(),
        );
        FormulaContext::new(open, high, low, close, volume, None)
    };
    let mut tree_context = vwap_context();
    let mut plan_context = vwap_context();
    tree_context.string_table.push("existing".to_string());
    plan_context.string_table.push("existing".to_string());
    let mut tree = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Tree);
    let mut plan = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
    let tree_values = tree
        .eval_with_dialect(vwap_source, FormulaDialect::Pine, &mut tree_context)
        .unwrap();
    let plan_values = plan
        .eval_with_dialect(vwap_source, FormulaDialect::Pine, &mut plan_context)
        .unwrap();
    let equal_series = |left: &Array1<f64>, right: &Array1<f64>| {
        left.len() == right.len()
            && left
                .iter()
                .zip(right)
                .all(|(left, right)| left == right || (left.is_nan() && right.is_nan()))
    };
    let values_equal = equal_series(&tree_values, &plan_values);
    let variables_equal = ["SRC", "VWAPVALUE", "PLOT"].into_iter().all(|name| {
        equal_series(
            tree_context.variables.get(name).unwrap(),
            plan_context.variables.get(name).unwrap(),
        )
    });
    println!(
        "[pine-vwap-parity] result_equal={values_equal} variables_equal={variables_equal} outputs_equal={} strings_equal={}",
        tree_context.output_names == plan_context.output_names,
        tree_context.string_table == plan_context.string_table,
    );

    let sma_source = include_str!("../../tests/pine_corpus/sma_overlay.pine");
    let sma_context = || {
        let close = Array1::from_vec(
            (0..256)
                .map(|index| 100.0 + index as f64 * 0.03 + (index as f64 * 0.09).sin() * 2.0)
                .collect(),
        );
        FormulaContext::new(
            close.clone(),
            close.clone(),
            close.clone(),
            close,
            Array1::from_elem(256, 1000.0),
            None,
        )
    };
    let mut tree_context = sma_context();
    let mut plan_context = sma_context();
    tree_context.string_table.push("existing".to_string());
    plan_context.string_table.push("existing".to_string());
    let mut tree = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Tree);
    let mut plan = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
    let tree_values = tree
        .eval_with_dialect(sma_source, FormulaDialect::Pine, &mut tree_context)
        .unwrap();
    let plan_values = plan
        .eval_with_dialect(sma_source, FormulaDialect::Pine, &mut plan_context)
        .unwrap();
    let values_equal = equal_series(&tree_values, &plan_values);
    let variables_equal = ["LENGTH", "SRC", "SMAVAL", "PLOT"].into_iter().all(|name| {
        equal_series(
            tree_context.variables.get(name).unwrap(),
            plan_context.variables.get(name).unwrap(),
        )
    });
    println!(
        "[pine-sma-parity] result_equal={values_equal} variables_equal={variables_equal} outputs_equal={} strings_equal={} modifiers_equal={}",
        tree_context.output_names == plan_context.output_names,
        tree_context.string_table == plan_context.string_table,
        tree_context.output_modifiers == plan_context.output_modifiers,
    );

    let obv_ma_source = "OBV_VAL:=OBV(CLOSE,VOL);MAOBV:=MA(OBV_VAL,30);";
    let obv_ma_plan = FormulaHotPlan::compile(&parse_formula(obv_ma_source).unwrap()).unwrap();
    let obv_ma_recognized = format!("{obv_ma_plan:?}").contains("simple_obv_ma: Some(");
    println!("[obv-ma-recognized] {obv_ma_recognized}");
    let obv_ma_context = || {
        let close = Array1::from_vec(
            (0..512)
                .map(|index| 100.0 + index as f64 * 0.01 + (index as f64 * 0.07).sin())
                .collect(),
        );
        let volume = Array1::from_vec(
            (0..512)
                .map(|index| 1000.0 + (index % 17) as f64 * 13.0)
                .collect(),
        );
        FormulaContext::new(
            close.clone(),
            close.clone(),
            close.clone(),
            close,
            volume,
            None,
        )
    };
    let mut tree_context = obv_ma_context();
    let mut plan_context = obv_ma_context();
    let mut tree = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Tree);
    let mut plan = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
    let tree_values = tree
        .eval_with_dialect(obv_ma_source, FormulaDialect::TongDaXin, &mut tree_context)
        .unwrap();
    let plan_values = plan
        .eval_with_dialect(obv_ma_source, FormulaDialect::TongDaXin, &mut plan_context)
        .unwrap();
    let values_equal = equal_series(&tree_values, &plan_values);
    let variables_equal = ["OBV_VAL", "MAOBV"].into_iter().all(|name| {
        equal_series(
            tree_context.variables.get(name).unwrap(),
            plan_context.variables.get(name).unwrap(),
        )
    });
    println!(
        "[obv-ma-parity] result_equal={values_equal} variables_equal={variables_equal} outputs_equal={} strings_equal={} modifiers_equal={}",
        tree_context.output_names == plan_context.output_names,
        tree_context.string_table == plan_context.string_table,
        tree_context.output_modifiers == plan_context.output_modifiers,
    );
}
