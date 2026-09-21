//! Print the compiled plan for a formula or Pine script.
//!
//! Shows the semantic DAG, the pre-resolved input bindings and the numeric hot
//! instructions, which is what you need when the compiled plan path diverges
//! from the AST interpreter or rejects a formula the dispatcher cannot run.
//!
//! ```text
//! cargo run -p finkit --example dump_formula_plan -- "MA(CLOSE,5)+MA(CLOSE,10)"
//! cargo run -p finkit --example dump_formula_plan -- --pine tests/pine_corpus/rsi.pine
//! ```

use finkit::formula::pine::{map_pine_to_alphata, parse_pine};
use finkit::formula::{parse_formula, FormulaHotPlan};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let ast = if args.first().is_some_and(|a| a == "--pine") {
        let path = args.get(1).expect("usage: --pine <file.pine>");
        let source = std::fs::read_to_string(path).expect("read pine script");
        let pine = parse_pine(&source).expect("parse_pine failed");
        map_pine_to_alphata(&pine).expect("map_pine_to_alphata failed")
    } else {
        let source = args.first().cloned().unwrap_or_else(|| {
            "DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26); DEA := EMA(DIF, 9); (DIF - DEA) * 2"
                .to_string()
        });
        parse_formula(&source).expect("parse failed")
    };

    let plan = FormulaHotPlan::compile(&ast).expect("compile failed");

    println!("semantic root: {:?}", plan.semantic().root());
    println!("semantic nodes ({}):", plan.semantic().plan().len());
    for &node_id in plan.semantic().plan().execution_order() {
        let node = plan.semantic().plan().node(node_id).expect("node");
        println!(
            "  {:?} op={:<28} effect={:?} deps={:?}",
            node_id, node.operation, node.capabilities.effect, node.dependencies
        );
    }

    println!("\ninput slots: {}", plan.hot().input_layout().len());
    for binding in plan.input_bindings() {
        println!("  binding {} -> slot {:?}", binding.name(), binding.slot());
    }

    println!("\nhot nodes ({}):", plan.hot().nodes().len());
    for (index, node) in plan.hot().nodes().iter().enumerate() {
        let operation = plan
            .semantic()
            .plan()
            .node(node.node)
            .map(|n| n.operation.as_str())
            .unwrap_or("<missing>");
        println!(
            "  [{index}] semantic={:?} op={operation:<28} kernel=0x{:016x} inputs={:?} out={:?} params={:?}",
            node.node, node.kernel.0, node.inputs, node.output, node.parameters
        );
    }

    let mut ops: Vec<&str> = plan
        .hot()
        .nodes()
        .iter()
        .filter_map(|node| {
            plan.semantic()
                .plan()
                .node(node.node)
                .map(|n| n.operation.as_str())
        })
        .collect();
    ops.sort_unstable();
    ops.dedup();
    println!("\nOPS: {}", ops.join(" "));
}
