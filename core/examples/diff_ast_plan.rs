//! Compare the AST interpreter and the compiled plan path on the shared fixture
//! and print the first divergence with a small value window around it.
//!
//! Use this to localize a failure reported by `formula_differential_tests` or
//! `formula_plan_differential` down to a specific index and formula.
//!
//! ```text
//! cargo run -p finkit --example diff_ast_plan
//! cargo run -p finkit --example diff_ast_plan -- "MA(CLOSE,6)/CLOSE"
//! ```

use finkit::formula::{
    parse_formula, unified_formula_executor, FormulaContext, FormulaEngine, FormulaHotPlan,
};
use ndarray::Array1;

fn fixture_ctx(path: &str) -> FormulaContext {
    let text = std::fs::read_to_string(path).expect("fixture");
    let mut header: Vec<String> = Vec::new();
    let mut cols: Vec<Vec<f64>> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if header.is_empty() {
            header = fields.iter().map(|s| s.to_string()).collect();
            cols = vec![Vec::new(); header.len()];
            continue;
        }
        for (index, field) in fields.iter().enumerate() {
            if let Ok(value) = field.parse::<f64>() {
                cols[index].push(value);
            }
        }
    }
    let column = |name: &str| -> Array1<f64> {
        let index = header.iter().position(|h| h == name).expect(name);
        Array1::from_vec(cols[index].clone())
    };
    FormulaContext::new(
        column("open"),
        column("high"),
        column("low"),
        column("close"),
        column("volume"),
        None,
    )
}

fn main() {
    let source = std::env::args().nth(1).unwrap_or_else(|| {
        "BIAS1:=(CLOSE-MA(CLOSE,6))/MA(CLOSE,6)*100;BIAS2:=(CLOSE-MA(CLOSE,12))/MA(CLOSE,12)*100;BIAS3:=(CLOSE-MA(CLOSE,24))/MA(CLOSE,24)*100;".to_string()
    });

    let mut ctx = fixture_ctx("tests/fixtures/ashare_sh_index_250d.csv");
    let reference = FormulaEngine::new().eval(&source, &mut ctx).expect("ast");

    let ast = parse_formula(&source).expect("parse");
    let plan = FormulaHotPlan::compile(&ast).expect("plan");
    let close = ctx.get_data("CLOSE").unwrap();
    let mut inputs: Vec<&[f64]> = vec![close; plan.hot().input_layout().len()];
    for binding in plan.input_bindings() {
        inputs[binding.slot().0] = ctx.get_data(binding.name()).unwrap();
    }
    let mut executor = unified_formula_executor(&plan);
    let candidate = executor.execute(&inputs).expect("plan execute").values[0].clone();

    println!("len: ast={} plan={}", reference.len(), candidate.len());
    let mut first = None;
    for index in 0..reference.len().min(candidate.len()) {
        let (a, b) = (reference[index], candidate[index]);
        let same = (a.is_nan() && b.is_nan()) || (a - b).abs() <= 1e-8;
        if !same {
            first = Some(index);
            break;
        }
    }
    match first {
        None => println!("no divergence"),
        Some(index) => {
            println!("first divergence at index {index}");
            let start = index.saturating_sub(3);
            for i in start..(index + 4).min(reference.len()) {
                println!(
                    "  [{i}] ast={:<22} plan={:<22} close={:<12}",
                    reference[i], candidate[i], ctx.close[i]
                );
            }
        }
    }
}
