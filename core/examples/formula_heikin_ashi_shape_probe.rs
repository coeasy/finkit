use finkit::formula::{
    FormulaContext, FormulaDialect, FormulaEngine, FormulaExecutionMode, ParamValues,
};
use ndarray::Array1;

fn main() {
    let source = include_str!("../../tests/pine_corpus/heikin_ashi.pine");
    for mode in [FormulaExecutionMode::Tree, FormulaExecutionMode::Plan] {
        let close = (10..26).map(f64::from).collect::<Vec<_>>();
        let open = close.iter().map(|value| value - 1.0).collect::<Vec<_>>();
        let high = close.iter().map(|value| value + 1.0).collect::<Vec<_>>();
        let low = open.iter().map(|value| value - 1.0).collect::<Vec<_>>();
        let mut context = FormulaContext::new(
            Array1::from_vec(open),
            Array1::from_vec(high),
            Array1::from_vec(low),
            Array1::from_vec(close),
            Array1::from_vec(vec![100.0; 16]),
            None,
        );
        let mut engine = FormulaEngine::new().with_execution_mode(mode);
        let primary = engine
            .eval_with_dialect(source, FormulaDialect::Pine, &mut context)
            .unwrap();
        println!("{mode:?} primary={:?}", primary.as_slice().unwrap());
        println!("  keys={:?}", context.variables.keys().collect::<Vec<_>>());
        for name in ["HACLOSE", "HAOPEN", "HAHIGH", "HALOW", "SMOOTHCLOSE"] {
            if let Some(series) = context.variables.get(name) {
                println!("  {name}={:?}", series.as_slice().unwrap());
            }
        }
    }
    let plan = FormulaEngine::new()
        .compile_plan(source, FormulaDialect::Pine, &ParamValues::new())
        .unwrap();
    println!(
        "hot_nodes={} inputs={} outputs={} vars={}",
        plan.hot().nodes().len(),
        plan.input_bindings().len(),
        plan.outputs().len(),
        plan.variables().len()
    );
    for &id in plan.semantic().plan().execution_order() {
        let node = plan.semantic().plan().node(id).unwrap();
        println!(
            "semantic {} id={} dependencies={:?}",
            node.operation,
            id.0,
            node.dependencies
                .iter()
                .map(|dependency| dependency.0)
                .collect::<Vec<_>>(),
        );
    }
    println!("outputs:");
    for (index, binding) in plan.outputs().iter().enumerate() {
        println!("  {index}: {} slot={}", binding.name(), binding.slot().0);
    }
    for variable in plan.variables() {
        println!(
            "  variable {} slot={} output_index={:?}",
            variable.name(),
            variable.slot().0,
            plan.hot()
                .output_layout()
                .outputs()
                .iter()
                .position(|(_, slot)| *slot == variable.slot())
        );
    }
    println!("layout:");
    for (index, (node, slot)) in plan.hot().output_layout().outputs().iter().enumerate() {
        println!("  {index}: node={} slot={}", node.0, slot.0);
    }
    println!("hot nodes:");
    for node in plan.hot().nodes() {
        let semantic = plan.semantic().plan().node(node.node).unwrap();
        println!(
            "  {} id={} output={} input={:?} kernel={:#x}",
            semantic.operation,
            node.node.0,
            node.output.0,
            node.inputs.iter().map(|slot| slot.0).collect::<Vec<_>>(),
            node.kernel.0
        );
    }
}
