//! Compiled-plan differential gate over the checked-in formula corpora.
//!
//! `formula_differential_tests.rs` proves the compiled plan path
//! (`FormulaHotPlan` + `UnifiedExecutor`) agrees with the AST tree-walker for a
//! small hand-written formula set. This file extends the same gate across every
//! corpus on disk — the domestic terminal corpus (`tests/formula_corpus/`) and
//! the Pine Script corpus (`tests/pine_corpus/`) — because that is the input
//! distribution the plan path must eventually serve in production.
//!
//! Two rules keep this honest:
//!
//! 1. Divergence is never tolerated silently. Any case the plan path cannot run
//!    must appear in [`DOMESTIC_UNSUPPORTED`] or [`PINE_UNSUPPORTED`] with a
//!    written reason. Each corpus has its own list so one corpus's entries can
//!    never look like stale entries to the other.
//! 2. The allowlists cannot rot. A listed case that starts working fails the
//!    test, so a list can only shrink by deliberate edit.

use finkit::execution_plan::KernelId;
use finkit::formula::pine::{map_pine_to_alphata, parse_pine};
use finkit::formula::{
    parse_formula_with_dialect, unified_formula_executor, AstNode, FormulaContext, FormulaDialect,
    FormulaEngine, FormulaHotPlan,
};
use finkit::unified_executor::ExecuteError;
use ndarray::Array1;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// Domestic corpus cases the compiled plan path cannot execute yet.
///
/// Every entry must state *why*. An entry that starts passing fails the test,
/// so this list can only shrink by deliberate edit. The reasons are the
/// concrete backlog for finishing the plan path — they are grouped by cause
/// rather than written per case.
const DOMESTIC_UNSUPPORTED: &[(&str, &str)] = &[
    // Chip distribution and cross-period functions are not numeric series
    // kernels at all: they need host-side data (per-bar chip histograms, the
    // chart period type) that the plan's numeric input layout cannot carry.
    (
        "chip_distribution_ths",
        "no kernel for `CALL:WINNER`/`CALL:COST`; needs per-bar chip-distribution data",
    ),
    (
        "cross_period_refdate",
        "no kernel for `CALL:PERIODTYPE`/`CALL:REFDATE`; needs chart-period host context",
    ),
    // What is left here is host context, not a missing kernel: `WINNER`/`COST`
    // need per-bar chip-distribution data and `PERIODTYPE`/`REFDATE` need the
    // chart period. The plan-path kernel backlog, and the inventory of the gap
    // between the SSOT and the formula surface, is pinned by
    // `formula_function_ssot.rs`.
];

/// Pine corpus cases the compiled plan path cannot execute yet.
///
/// The output-selection failures that used to dominate this list are gone: a
/// statement block now reports its last *value-producing* statement, so a
/// script ending in `hline(...)` reports its `plot(...)` series instead of the
/// marker constant. Every remaining entry is a genuine kernel or lowering gap.
const PINE_UNSUPPORTED: &[(&str, &str)] = &[
    // --- Missing numeric kernels -------------------------------------------------
    (
        "bollinger_bands",
        "no kernel for `CALL:BOLLUP`/`CALL:BOLLMID`/`CALL:BOLLDN`",
    ),
    (
        "macd",
        "no kernel for `CALL:DEA` (`CALL:MACD` itself is implemented)",
    ),
    ("macd_histogram", "no kernel for `CALL:DEA`"),
    ("parabolic_sar", "no kernel for `CALL:SAR`"),
    (
        "supertrend",
        "no kernel for `CALL:IF` / no lowering for the `IF_THEN_ELSE` node",
    ),
    (
        "stochastic",
        "no kernel for `CALL:STOCHF` (fast stochastic)",
    ),
    ("trix", "no kernel for `CALL:TRIX`"),
    // --- Structural lowering gaps ------------------------------------------------
    // `compute_ir` treats loop bodies as opaque control flow and does not lower
    // them into the acyclic compute plan, so `volume[i]` never becomes a bound
    // input and the executor cannot infer an execution length.
    (
        "volume_profile",
        "`for` loop bodies are not lowered into the compute plan (series indexing)",
    ),
];

fn read_fixture(path: &Path) -> HashMap<String, Vec<f64>> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{path:?}: {error}"));
    let mut header = None;
    let mut columns = HashMap::<String, Vec<f64>>::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split(',').map(str::trim).collect::<Vec<_>>();
        if header.is_none() {
            for field in &fields {
                columns.insert((*field).to_string(), Vec::new());
            }
            header = Some(fields);
            continue;
        }
        let names = header.as_ref().unwrap();
        assert_eq!(
            fields.len(),
            names.len(),
            "CSV row width mismatch in {path:?}"
        );
        for (name, value) in names.iter().zip(fields.iter()) {
            if let Ok(value) = value.parse::<f64>() {
                columns.get_mut(*name).unwrap().push(value);
            }
        }
    }
    assert!(!columns.is_empty(), "CSV fixture has no columns: {path:?}");
    columns
}

fn dialect_for(platform: &str) -> FormulaDialect {
    match platform.to_ascii_lowercase().as_str() {
        "tdx" | "dzh" | "cross" => FormulaDialect::TongDaXin,
        "ths" => FormulaDialect::TongHuaShun,
        "eastmoney" | "em" => FormulaDialect::EastMoney,
        "pine" | "tradingview" => FormulaDialect::Pine,
        other => panic!("unsupported formula corpus platform: {other}"),
    }
}

fn context_from(columns: &HashMap<String, Vec<f64>>) -> FormulaContext {
    let values = |name: &str| {
        Array1::from_vec(
            columns
                .get(name)
                .unwrap_or_else(|| panic!("required OHLCV column missing: {name}"))
                .clone(),
        )
    };
    FormulaContext::new(
        values("open"),
        values("high"),
        values("low"),
        values("close"),
        values("volume"),
        None,
    )
}

/// Execute a pre-built AST through the compiled plan path.
///
/// This is the production caller contract: compile once, bind the context
/// series into the plan's numeric input slots, then drive the unified executor.
/// A slot that no binding fills is a hard error — silently substituting another
/// series would hide an input-layout bug behind a numeric mismatch.
fn plan_values(ast: &AstNode, ctx: &FormulaContext) -> Result<Array1<f64>, String> {
    let plan = FormulaHotPlan::compile(ast).map_err(|error| format!("compile: {error}"))?;

    let mut slots: Vec<Option<&[f64]>> = vec![None; plan.hot().input_layout().len()];
    for binding in plan.input_bindings() {
        let values = ctx
            .get_data(binding.name())
            .ok_or_else(|| format!("missing input series `{}`", binding.name()))?;
        slots[binding.slot().0] = Some(values);
    }
    let mut inputs = Vec::with_capacity(slots.len());
    for (index, slot) in slots.into_iter().enumerate() {
        inputs.push(slot.ok_or_else(|| format!("input slot {index} was never bound"))?);
    }

    let mut executor = unified_formula_executor(&plan);
    let result = executor.execute(&inputs).map_err(|error| {
        // Name the operation behind a failed kernel: a bare numeric code is not
        // actionable, and this is what turns the allowlist below into a
        // concrete, checkable backlog.
        if let ExecuteError::Kernel(dispatch) = &error {
            if let Some(kernel) = dispatch.kernel {
                if let Some(operation) = kernel_operations(&plan).get(&kernel.0) {
                    return format!("execute: unsupported kernel `{operation}` (code {})", dispatch.code);
                }
            }
        }
        format!("execute: {error}")
    })?;
    let values = result
        .values
        .into_iter()
        .next()
        .ok_or_else(|| "plan produced no output series".to_string())?;
    Ok(Array1::from_vec(values))
}

/// Map numeric kernel ids back to semantic operation labels for diagnostics.
fn kernel_operations(plan: &FormulaHotPlan) -> BTreeMap<u64, String> {
    let mut map = BTreeMap::new();
    for &node_id in plan.semantic().plan().execution_order() {
        let node = plan
            .semantic()
            .plan()
            .node(node_id)
            .expect("semantic execution order only contains compiled nodes");
        map.insert(
            KernelId::from_static(&node.operation).0,
            node.operation.clone(),
        );
    }
    map
}

fn values_match(a: f64, b: f64, tolerance: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_nan() || b.is_nan() {
        return false;
    }
    (a - b).abs() <= tolerance
}

/// Compare one case, returning a human-readable divergence description.
fn compare(reference: &Array1<f64>, candidate: &Array1<f64>, tolerance: f64) -> Result<(), String> {
    if reference.len() != candidate.len() {
        return Err(format!(
            "length mismatch: ast={} plan={}",
            reference.len(),
            candidate.len()
        ));
    }
    for index in 0..reference.len() {
        if !values_match(reference[index], candidate[index], tolerance) {
            return Err(format!(
                "index {index}: ast={} plan={} (tolerance={tolerance})",
                reference[index], candidate[index]
            ));
        }
    }
    Ok(())
}

fn corpus_files(dir: &str, extension: &str) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    let mut files = std::fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("{root:?}: {error}"))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|found| found == extension))
        .collect::<Vec<_>>();
    files.sort();
    files
}

/// Run the whole gate and report the unsupported set.
///
/// Returns `(ran, unsupported)` where `unsupported` maps case id → reason.
fn run_gate(
    mut reference_of: impl FnMut(&str) -> Result<(AstNode, FormulaContext, Array1<f64>, f64), String>,
    case_ids: impl IntoIterator<Item = String>,
) -> (usize, BTreeMap<String, String>) {
    let mut unsupported = BTreeMap::new();
    let mut ran = 0usize;

    for case_id in case_ids {
        let (ast, ctx, reference, tolerance) = match reference_of(&case_id) {
            Ok(value) => value,
            Err(reason) => {
                unsupported.insert(case_id, format!("AST reference path failed: {reason}"));
                continue;
            }
        };
        match plan_values(&ast, &ctx) {
            Ok(candidate) => match compare(&reference, &candidate, tolerance) {
                Ok(()) => ran += 1,
                Err(divergence) => {
                    unsupported.insert(case_id, format!("divergence: {divergence}"));
                }
            },
            Err(reason) => {
                unsupported.insert(case_id, reason);
            }
        }
    }

    (ran, unsupported)
}

/// Fail when the observed unsupported set differs from the declared allowlist.
///
/// The full inventory is printed before any assertion so a failure reports
/// every gap at once instead of only the first one.
fn assert_allowlist_matches(
    unsupported: &BTreeMap<String, String>,
    declared_entries: &[(&str, &str)],
    label: &str,
) {
    let declared: BTreeMap<String, &str> = declared_entries
        .iter()
        .map(|(id, reason)| ((*id).to_string(), *reason))
        .collect();

    eprintln!(
        "{label}: {} case(s) outside the compiled plan path",
        unsupported.len()
    );
    for (id, reason) in unsupported {
        let marker = if declared.contains_key(id) {
            "declared"
        } else {
            "UNDECLARED"
        };
        eprintln!("  - [{marker}] {id}: {reason}");
    }

    let undeclared: Vec<_> = unsupported
        .keys()
        .filter(|id| !declared.contains_key(*id))
        .collect();
    let stale: Vec<_> = declared
        .keys()
        .filter(|id| !unsupported.contains_key(*id))
        .collect();

    assert!(
        undeclared.is_empty(),
        "{label}: {} case(s) the compiled plan path cannot run are missing from \
         PLAN_UNSUPPORTED: {undeclared:?}\n\
         Either fix the plan path or add them to PLAN_UNSUPPORTED with a reason.",
        undeclared.len()
    );
    assert!(
        stale.is_empty(),
        "{label}: the allowlist lists {stale:?} but the compiled plan path now \
         handles them. Remove the stale entries."
    );
}

#[test]
fn domestic_corpus_plan_matches_ast_reference() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let files = corpus_files("../tests/formula_corpus", "json");
    assert!(!files.is_empty(), "formula corpus is empty");

    let mut cases = Vec::new();
    let mut sources = BTreeMap::<String, (String, FormulaDialect, String, f64)>::new();
    for path in &files {
        let text = std::fs::read_to_string(path).unwrap();
        let case: Value = serde_json::from_str(&text)
            .unwrap_or_else(|error| panic!("invalid corpus JSON {path:?}: {error}"));
        let id = case["id"]
            .as_str()
            .unwrap_or_else(|| panic!("{} is missing id", path.display()))
            .to_string();
        let source = case["source_formula"].as_str().unwrap().to_string();
        let dialect = dialect_for(case["platform"].as_str().unwrap());
        let data_file = case["input"]["data_file"].as_str().unwrap().to_string();
        let tolerance = case["tolerance"].as_f64().unwrap_or(1e-10);
        cases.push(id.clone());
        sources.insert(id, (source, dialect, data_file, tolerance));
    }

    let (ran, unsupported) = run_gate(
        |case_id| {
            let (source, dialect, data_file, tolerance) = sources
                .get(case_id)
                .ok_or_else(|| format!("unknown case {case_id}"))?;
            let columns = read_fixture(&repo_root.join(data_file));
            let mut ctx = context_from(&columns);
            let ast = parse_formula_with_dialect(source, *dialect)
                .map_err(|error| format!("parse: {error}"))?;
            let reference = FormulaEngine::new()
                .eval_with_dialect(source, *dialect, &mut ctx)
                .map_err(|error| format!("eval: {error}"))?;
            Ok((ast, ctx, reference, *tolerance))
        },
        cases,
    );

    eprintln!("domestic corpus: {ran} case(s) verified through the compiled plan path");
    assert_allowlist_matches(&unsupported, DOMESTIC_UNSUPPORTED, "domestic corpus");
}

#[test]
fn pine_corpus_plan_matches_ast_reference() {
    let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/pine_corpus");
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(corpus_dir.join("manifest.json")).unwrap(),
    )
    .expect("Pine corpus manifest must be valid JSON");
    let skipped: Vec<String> = manifest["scripts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["status"].as_str() == Some("host_required"))
        .map(|entry| entry["id"].as_str().unwrap().to_string())
        .collect();

    let mut sources = BTreeMap::<String, String>::new();
    let mut cases = Vec::new();
    for path in corpus_files("../tests/pine_corpus", "pine") {
        let id = path.file_stem().unwrap().to_string_lossy().to_string();
        // `host_required` scripts need an external security/host resolver that
        // the plan path does not provide either, so they are out of scope here.
        if skipped.contains(&id) {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        cases.push(id.clone());
        sources.insert(id, source);
    }
    assert!(!cases.is_empty(), "Pine corpus has no evaluable scripts");

    let (ran, unsupported) = run_gate(
        |case_id| {
            let source = sources
                .get(case_id)
                .ok_or_else(|| format!("unknown case {case_id}"))?;
            let pine = parse_pine(source).map_err(|error| format!("parse_pine: {error}"))?;
            let ast = map_pine_to_alphata(&pine)
                .map_err(|error| format!("map_pine_to_alphata: {error}"))?;
            let mut ctx = synthetic_ohlcv(120);
            let reference = FormulaEngine::new()
                .eval_ast(&ast, &mut ctx)
                .map_err(|error| format!("eval_ast: {error}"))?;
            Ok((ast, ctx, reference, 1e-10))
        },
        cases,
    );

    eprintln!("Pine corpus: {ran} case(s) verified through the compiled plan path");
    assert_allowlist_matches(&unsupported, PINE_UNSUPPORTED, "Pine corpus");
}

/// Deterministic synthetic OHLCV, matching the Pine corpus runner so the two
/// gates cannot disagree about inputs.
fn synthetic_ohlcv(n: usize) -> FormulaContext {
    let mut seed: u64 = 0x1234_5678_AB_CD_EF_00;
    let mut rng = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) as f64 / (u64::MAX as f64)
    };

    let mut open = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut close = Vec::with_capacity(n);
    let mut vol = Vec::with_capacity(n);
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
        vol.push(v);
    }

    let mut ctx = FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(vol),
        None,
    );
    let n = ctx.data_len;
    let hl2: Vec<f64> = (0..n).map(|i| (ctx.high[i] + ctx.low[i]) / 2.0).collect();
    let hlc3: Vec<f64> = (0..n)
        .map(|i| (ctx.high[i] + ctx.low[i] + ctx.close[i]) / 3.0)
        .collect();
    let ohlc4: Vec<f64> = (0..n)
        .map(|i| (ctx.open[i] + ctx.high[i] + ctx.low[i] + ctx.close[i]) / 4.0)
        .collect();
    ctx.set_variable("HL2".to_string(), Array1::from_vec(hl2));
    ctx.set_variable("HLC3".to_string(), Array1::from_vec(hlc3));
    ctx.set_variable("OHLC4".to_string(), Array1::from_vec(ohlc4));
    ctx
}
