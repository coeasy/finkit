//! Probe: does one formula source produce one result through every public
//! compile/execute entry point?
//!
//! `FormulaOptimizer::optimize` runs statement-level dead-code elimination.
//! `optimize_for_execution` deliberately does not, because `X:=...`
//! assignments are observable through `FormulaContext::variables`. Several
//! *execution* entry points still call the former, so the same source can
//! behave differently depending on which method the caller picked.
//!
//! Run with: `cargo run -p finkit --example backend_divergence_probe`

use finkit::formula::compiler::FormulaCompiler;
use finkit::formula::{FormulaContext, FormulaEngine, FormulaExecutionMode};
use ndarray::Array1;

fn make_ctx(len: usize) -> FormulaContext {
    let open = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.1).collect());
    let high = Array1::from_vec((0..len).map(|i| 11.0 + i as f64 * 0.2).collect());
    let low = Array1::from_vec((0..len).map(|i| 9.0 + i as f64 * 0.1).collect());
    let close = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.15).collect());
    let volume = Array1::from_vec((0..len).map(|i| 1000.0 + i as f64 * 10.0).collect());
    FormulaContext::new(open, high, low, close, volume, None)
}

/// `ctx.variables` after `FormulaEngine::eval` — the documented contract is that
/// every `X:=...` assignment a formula executes is published there.
fn variables_after_default(source: &str) -> Vec<String> {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(30);
    engine.eval(source, &mut ctx).expect("default eval failed");
    let mut names: Vec<String> = ctx.variables.keys().map(|k| k.to_string()).collect();
    names.sort();
    names
}

fn variables_after_optimized(source: &str) -> Vec<String> {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(30);
    engine
        .eval_optimized(source, &mut ctx)
        .expect("optimized eval failed");
    let mut names: Vec<String> = ctx.variables.keys().map(|k| k.to_string()).collect();
    names.sort();
    names
}

fn variables_after_compiler(source: &str) -> Vec<String> {
    let mut compiler = FormulaCompiler::new(10);
    let mut ctx = make_ctx(30);
    compiler
        .compile_and_execute(source, &mut ctx)
        .expect("FormulaCompiler failed");
    let mut names: Vec<String> = ctx.variables.keys().map(|k| k.to_string()).collect();
    names.sort();
    names
}

fn last_value(engine: &mut FormulaEngine, source: &str) -> f64 {
    let mut ctx = make_ctx(30);
    let values = engine.eval(source, &mut ctx).expect("eval failed");
    *values.last().expect("non-empty")
}

fn main() {
    println!("== case 1: an assignment no output reads ==");
    let src = "JUNK:=MA(CLOSE,5); OUT: CLOSE;";
    println!("  source            : {src}");
    println!("  eval              : {:?}", variables_after_default(src));
    println!("  eval_optimized    : {:?}", variables_after_optimized(src));
    println!("  FormulaCompiler   : {:?}", variables_after_compiler(src));

    println!();
    println!("== case 2: does it reach the primary result? ==");
    // A string literal evaluates to its index in `FormulaContext::string_table`.
    // Dropping the unused assignment drops one literal, which shifts the index
    // the surviving output reports.
    let src = "TMP:='HELLO'; OUT: 'WORLD';";
    println!("  source            : {src}");
    let mut default_engine = FormulaEngine::new();
    println!(
        "  eval              : {}",
        last_value(&mut default_engine, src)
    );

    let mut optimized_engine = FormulaEngine::new();
    let mut ctx = make_ctx(30);
    let optimized = optimized_engine
        .eval_optimized(src, &mut ctx)
        .expect("optimized eval failed");
    println!(
        "  eval_optimized    : {}",
        optimized.last().copied().unwrap_or(f64::NAN)
    );

    let mut compiler = FormulaCompiler::new(10);
    let mut ctx = make_ctx(30);
    let compiled = compiler
        .compile_and_execute(src, &mut ctx)
        .expect("FormulaCompiler failed");
    println!(
        "  FormulaCompiler   : {}",
        compiled.last().copied().unwrap_or(f64::NAN)
    );
    println!(
        "  string_table(len) : {} (after the last call)",
        ctx.string_table.len()
    );

    println!();
    println!("== case 3: the plan backend, same sources ==");
    for source in [
        "JUNK:=MA(CLOSE,5); OUT: CLOSE;",
        "TMP:='HELLO'; OUT: 'WORLD';",
    ] {
        let mut engine = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
        let mut ctx = make_ctx(30);
        match engine.eval(source, &mut ctx) {
            Ok(values) => println!(
                "  {source:<34} -> {}",
                values.last().copied().unwrap_or(f64::NAN)
            ),
            Err(error) => println!("  {source:<34} -> ERROR {error:?}"),
        }
    }

    println!();
    println!("== case 4: the bytecode VM, same sources ==");
    for source in [
        "JUNK:=MA(CLOSE,5); OUT: CLOSE;",
        "TMP:='HELLO'; OUT: 'WORLD';",
    ] {
        let mut engine = FormulaEngine::new();
        let ctx = make_ctx(30);
        match engine
            .compile_bytecode(source)
            .and_then(|bytecode| engine.execute_bytecode(&bytecode, &ctx))
        {
            Ok(values) => println!(
                "  {source:<34} -> {}",
                values.last().copied().unwrap_or(f64::NAN)
            ),
            Err(error) => println!("  {source:<34} -> ERROR {error:?}"),
        }
    }

    println!();
    println!("== case 5: an assignment that shadows a builtin alias ==");
    // `A` is an official alias for AMOUNT on every path, so `A:=...` has to
    // shadow it the way `X:=...` would.
    let shadow = "A:=CLOSE*2; B:=A+CLOSE; OUT: B;";
    println!("  source            : {shadow}");
    {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        match engine.eval(shadow, &mut ctx) {
            Ok(values) => println!(
                "  eval              : {}",
                values.last().copied().unwrap_or(f64::NAN)
            ),
            Err(error) => println!("  eval              : ERROR {error:?}"),
        }
    }
    {
        let mut engine = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
        let mut ctx = make_ctx(30);
        match engine.eval(shadow, &mut ctx) {
            Ok(values) => println!(
                "  plan              : {}",
                values.last().copied().unwrap_or(f64::NAN)
            ),
            Err(error) => println!("  plan              : ERROR {error:?}"),
        }
    }
    {
        let mut engine = FormulaEngine::new();
        let ctx = make_ctx(30);
        match engine
            .compile_bytecode(shadow)
            .and_then(|bytecode| engine.execute_bytecode(&bytecode, &ctx))
        {
            Ok(values) => println!(
                "  bytecode          : {}",
                values.last().copied().unwrap_or(f64::NAN)
            ),
            Err(error) => println!("  bytecode          : ERROR {error:?}"),
        }
    }
    {
        let mut compiler = FormulaCompiler::new(10);
        let mut ctx = make_ctx(30);
        match compiler.compile_and_execute(shadow, &mut ctx) {
            Ok(values) => println!(
                "  FormulaCompiler   : {}",
                values.last().copied().unwrap_or(f64::NAN)
            ),
            Err(error) => println!("  FormulaCompiler   : ERROR {error:?}"),
        }
    }
}
