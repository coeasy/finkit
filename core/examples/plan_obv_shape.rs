use finkit::formula::{FormulaContext, FormulaDialect, FormulaEngine, ParamValues};
use ndarray::Array1;

fn main() {
    let source = include_str!("../../tests/pine_corpus/obv.pine");
    let engine = FormulaEngine::new();
    let plan = engine
        .compile_plan(source, FormulaDialect::Pine, &ParamValues::new())
        .unwrap();
    let semantic = plan.semantic();
    let graph = semantic.plan();
    println!(
        "nodes={} root={:?} hot={} inputs={} outputs={} variables={} strings={}",
        graph.execution_order().len(),
        graph
            .node(semantic.root())
            .map(|node| node.operation.as_str()),
        plan.hot().nodes().len(),
        plan.input_bindings().len(),
        plan.outputs().len(),
        plan.variables().len(),
        semantic.string_literals_ordered().len(),
    );
    for &id in graph.execution_order() {
        let node = graph.node(id).unwrap();
        println!("{}: {} deps={:?}", id.0, node.operation, node.dependencies);
    }
    for binding in plan.input_bindings() {
        println!("input {} -> {}", binding.slot().0, binding.name());
    }
    for binding in plan.outputs() {
        println!("output {} -> {}", binding.slot().0, binding.name());
    }
    for binding in plan.variables() {
        println!("variable {} -> {}", binding.slot().0, binding.name());
    }

    let close = Array1::from_iter((0..512).map(|i| 100.0 + i as f64 * 0.01));
    let volume = Array1::from_iter((0..512).map(|i| 1000.0 + i as f64));
    let mut context = FormulaContext::new(
        close.clone(),
        close.clone(),
        close.clone(),
        close.clone(),
        volume.clone(),
        None,
    );
    let result = engine
        .eval_plan_channels(
            source,
            FormulaDialect::Pine,
            &ParamValues::new(),
            &mut context,
        )
        .unwrap();
    let expected = finkit::math::volume_kernels::obv_vec(
        close.as_slice().unwrap(),
        volume.as_slice().unwrap(),
    )
    .unwrap();
    assert_eq!(result.primary(), expected);
    assert_eq!(result.channels().len(), 1);
    assert_eq!(result.variables().len(), 2);
    assert!(context.variables.contains_key("OBVVAL"));
    assert!(context.variables.contains_key("PLOT"));
    assert_eq!(context.output_names, vec!["PLOT"]);
    println!(
        "evaluation rows={} channels={} variables={} side-effects=preserved",
        result.primary().len(),
        result.channels().len(),
        result.variables().len(),
    );
}
