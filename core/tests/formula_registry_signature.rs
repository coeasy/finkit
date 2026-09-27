//! The registry's declared call signature must actually be executable.
//!
//! [`builtin_function_registry`] is the metadata surface: it tells the CLI,
//! the FFI schema and the docs what inputs and parameters each function takes.
//! Nothing checked that the declared shape matches what the engine accepts, so
//! the metadata rotted in a way that was invisible to every other gate:
//!
//! * `ICHIMOKU_TENKAN`, `ICHIMOKU_KIJUN`, `FISHER`, `FISHER_SIGNAL`, the five
//!   `DONCHIAN*` projections and `AROON_UP`/`AROON_DN` were declared
//!   `InputKind::Hlc`, i.e. *high, low, close* plus a period. Their
//!   implementations take *high, low* plus a period. A caller who followed the
//!   metadata passed a `close` where the engine expected the period, so the tree
//!   path silently computed over a period of `CLOSE[0]` (about 100 instead of 9)
//!   while the compiled-plan path failed with an arity error. The enum had no
//!   way to say "two series", which is why `Hlc` was used as an approximation;
//!   it now has `InputKind::Hl`.
//!
//! This gate rebuilds the call the metadata describes and runs it through both
//! execution paths. It is deliberately driven by the *declared* metadata rather
//! than by a hand-written table, because a hand-written table is exactly what
//! drifts.
//!
//! The **series** part of the declaration is what matters most: getting it wrong
//! produces plausible-looking wrong numbers. The **parameter** part is allowed
//! to be a superset, because one registry describes two surfaces — the operation
//! and FFI surface (`finkit.bbands(real, timeperiod, nbdevup, nbdevdn, matype)`)
//! and the formula surface, where a call evaluates to one series and so takes a
//! shorter parameter list. [`FORMULA_PARAM_SUBSET`] records exactly which
//! functions that applies to, with a reason, and — like the corpus allowlists —
//! an entry that stops being needed fails the gate.
//!
//! Cases whose declared shape is `InputKind::Dynamic` are skipped: the metadata
//! makes no claim there, so there is nothing to contradict.

use finkit::formula::{
    parse_formula_with_dialect, unified_formula_executor, AstNode, FormulaContext, FormulaDialect,
    FormulaEngine, FormulaHotPlan,
};
use finkit::registry::{builtin_function_registry, InputKind};
use ndarray::Array1;

fn synthetic_ohlcv(n: usize) -> FormulaContext {
    let mut seed: u64 = 0x1234_5678_AB_CD_EF_00;
    let mut rng = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) as f64 / (u64::MAX as f64)
    };
    let (mut open, mut high, mut low, mut close, mut volume) = (
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
    );
    let mut c = 100.0;
    for _ in 0..n {
        let d = (rng() - 0.5) * 4.0;
        c += d;
        let o = c - d * 0.5;
        let hi = c.max(o) + rng() * 2.0 + 0.01;
        let lo = c.min(o) - rng() * 2.0 - 0.01;
        let v = 1000.0 + rng() * 500.0;
        open.push(o);
        high.push(hi);
        low.push(lo);
        close.push(c);
        volume.push(v);
    }
    FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
        None,
    )
}

/// Functions whose *formula* call takes fewer parameters than the registry
/// declares.
///
/// This is a real difference between two surfaces, not a metadata error: the
/// registry describes the operation and FFI entry points, which are the ones
/// that accept the full TA-Lib parameter set, while a formula evaluates to a
/// single series and so accepts a shorter list.
///
/// The list is non-rotting in the same way as `DOMESTIC_UNSUPPORTED`: an entry
/// that stops being needed fails the test, so it can only shrink by a deliberate
/// edit.
const FORMULA_PARAM_SUBSET: &[(&str, &str)] = &[
    (
        "BBANDS",
        "formula takes (close, period, nbdev): one deviation multiplier, no matype",
    ),
    (
        "BOLLUP",
        "formula takes (close, period, nbdev): one deviation multiplier, no matype",
    ),
    (
        "BOLLDN",
        "formula takes (close, period, nbdev): one deviation multiplier, no matype",
    ),
    (
        "STDDEV",
        "formula takes (close, period): the deviation is fixed at 1.0",
    ),
    ("VAR", "formula takes (close, period): the deviation is fixed at 1.0"),
];

/// The series arguments [`InputKind`] promises, as formula expressions.
fn declared_series(input: InputKind) -> Option<&'static [&'static str]> {
    match input {
        InputKind::Series => Some(&["CLOSE"]),
        InputKind::Hl => Some(&["HIGH", "LOW"]),
        InputKind::Hlc => Some(&["HIGH", "LOW", "CLOSE"]),
        InputKind::Hlcv => Some(&["HIGH", "LOW", "CLOSE", "VOLUME"]),
        InputKind::Ohlcv => Some(&["OPEN", "HIGH", "LOW", "CLOSE", "VOLUME"]),
        InputKind::Dynamic => None,
    }
}

/// Render a declared parameter as a literal, falling back to a sane value when
/// the spec has no default.
fn declared_param(default: Option<&str>, value_type: &str) -> String {
    match default {
        Some(value) => value.to_string(),
        None if value_type == "usize" => "14".to_string(),
        None => "2.0".to_string(),
    }
}

fn plan_values(ast: &AstNode, ctx: &FormulaContext) -> Result<Array1<f64>, String> {
    let plan = FormulaHotPlan::compile(ast).map_err(|error| format!("compile: {error}"))?;
    let mut slots: Vec<Option<&[f64]>> = vec![None; plan.hot().input_layout().len()];
    for binding in plan.input_bindings() {
        slots[binding.slot().0] = ctx.get_data(binding.name());
    }
    let mut inputs = Vec::with_capacity(slots.len());
    for (index, slot) in slots.into_iter().enumerate() {
        inputs.push(slot.ok_or_else(|| format!("input slot {index} was never bound"))?);
    }
    let mut executor = unified_formula_executor(&plan);
    let result = executor.execute(&inputs).map_err(|error| format!("execute: {error}"))?;
    result
        .values
        .into_iter()
        .next()
        .map(Array1::from_vec)
        .ok_or_else(|| "plan produced no output series".to_string())
}

/// Run one call shape, returning whether the two paths agree.
///
/// `Ok(())` means the call ran on both paths and agreed, or that the tree path
/// itself produced nothing (an all-NaN reference proves nothing either way, and
/// the numeric gates own that case). `Err` carries why a path rejected it.
fn run_shape(source: &str) -> Result<(), String> {
    let ast = parse_formula_with_dialect(source, FormulaDialect::TongDaXin)
        .map_err(|error| format!("does not parse: {error}"))?;
    let mut reference_ctx = synthetic_ohlcv(128);
    let reference = FormulaEngine::new()
        .eval_ast(&ast, &mut reference_ctx)
        .map_err(|error| format!("tree path rejects it: {error}"))?;
    let plan_ctx = synthetic_ohlcv(128);
    let candidate = plan_values(&ast, &plan_ctx)?;

    if !reference.iter().any(|value| value.is_finite()) {
        return Ok(());
    }
    for index in 0..reference.len().min(candidate.len()) {
        let (a, b) = (reference[index], candidate[index]);
        if !(a.is_nan() && b.is_nan()) && (a - b).abs() > 1e-9 {
            return Err(format!("paths diverge at {index}: {a} vs {b}"));
        }
    }
    Ok(())
}

#[test]
fn declared_signatures_run_on_both_execution_paths() {
    let registry = builtin_function_registry();
    let mut failures: Vec<String> = Vec::new();
    let mut checked = 0usize;
    let mut subset_used: Vec<&str> = Vec::new();

    for spec in registry.iter() {
        let Some(series) = declared_series(spec.input) else {
            continue;
        };
        checked += 1;
        let series_args: Vec<String> = series.iter().map(|name| (*name).to_string()).collect();
        let params: Vec<String> = spec
            .params
            .iter()
            .map(|param| declared_param(param.default, param.value_type))
            .collect();

        // Try the fully declared call first. A function recorded in
        // `FORMULA_PARAM_SUBSET` may then be retried with trailing parameters
        // dropped, which is how the operation surface's superset of parameters
        // is distinguished from a genuinely wrong declaration.
        let allowed_subset = FORMULA_PARAM_SUBSET
            .iter()
            .any(|(name, _)| *name == spec.name);
        let mut kept = params.len();
        let mut last_error = String::new();
        loop {
            let mut args = series_args.clone();
            args.extend_from_slice(&params[..kept]);
            let source = format!("{}({})", spec.name, args.join(", "));
            match run_shape(&source) {
                Ok(()) => {
                    if kept != params.len() {
                        subset_used.push(spec.name);
                    }
                    last_error.clear();
                    break;
                }
                Err(error) => {
                    last_error = format!("{source}: {error}");
                    if !allowed_subset || kept == 0 {
                        break;
                    }
                    kept -= 1;
                }
            }
        }
        if !last_error.is_empty() {
            failures.push(last_error);
        }
    }

    // The subset list cannot rot in either direction.
    let declared_subset: Vec<&str> = FORMULA_PARAM_SUBSET.iter().map(|(n, _)| *n).collect();
    let stale: Vec<&&str> = declared_subset
        .iter()
        .filter(|name| !subset_used.contains(*name))
        .collect();
    let undeclared: Vec<&&str> = subset_used
        .iter()
        .filter(|name| !declared_subset.contains(*name))
        .collect();
    assert!(
        stale.is_empty() && undeclared.is_empty(),
        "FORMULA_PARAM_SUBSET is out of date: no longer needed {stale:?}, \
         needed but not declared {undeclared:?}"
    );

    assert!(
        failures.is_empty(),
        "{} of {} declared signatures are not executable as documented:\n{}",
        failures.len(),
        checked,
        failures.join("\n")
    );
}
