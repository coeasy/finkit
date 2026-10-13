//! Phase 1.4 workload probe: Tree vs Plan over the *real* checked-in corpora.
//!
//! The 2026-10-13 refactor plan's phase 1 item 5 asks for functional coverage,
//! result consistency, performance and memory to be measured against the real
//! application formula corpus, and says explicitly that the repository's
//! recorded "all kernels covered" status does not substitute for that evidence.
//! This probe is that measurement. It answers, with numbers:
//!
//! * **coverage** — how many corpus cases the Plan backend can run at all;
//! * **consistency** — how many agree with the Tree reference, and how many are
//!   *degenerate* (both paths agree but both are all-NaN, which is parity on
//!   nothing and must not be counted as a pass);
//! * **performance** — cold (parse + compile + execute) and warm (cached
//!   execute) nanoseconds per mode, because those are the two costs a host
//!   actually pays;
//! * **memory** — allocations per warm execute, via a counting global
//!   allocator.
//!
//! Measured 2026-10-13 over 60 cases (35 domestic + 25 Pine) at 250 and 5000
//! rows: coverage is **60/60** non-degenerate at both sizes. The two corpora
//! behave very differently, and the reason is worth stating because it is not
//! obvious from the numbers alone:
//!
//! * **Pine** — Plan wins decisively. `FormulaEngine::eval_with_dialect`'s Tree
//!   branch re-parses and re-maps Pine on *every* call (it does not go through
//!   the cached `compile_shared` path), so warm Tree Pine is ~4.3x slower and
//!   ~11x more allocating than warm Plan, which hits `plan_cache` for every
//!   dialect.
//! * **Domestic** — Plan is slower warm (roughly 1.5–2.0x) and allocates
//!   ~1.5x more, because the Tree branch caches its `CompiledFormula` and the
//!   plan executor's per-node dispatch costs more than an AST walk at these
//!   sizes.
//!
//! Run with:
//!
//! ```text
//! cargo run -p finkit --release --example formula_mode_workload_probe
//! cargo run -p finkit --release --example formula_mode_workload_probe -- --scaled
//! ```

// The synthetic-input helpers below deliberately mirror
// `core/tests/formula_plan_differential.rs` so the two tools cannot disagree
// about inputs. That means a few pedantic lints fire here exactly as they do
// there, and "fixing" them would silently change the numbers this probe exists
// to compare. The crate's clippy gate does not cover examples.
//!
//! Exit status is 0 even when a gap is found: this is a measurement, not a
//! gate. The gates that *fail* on a Plan/Tree gap are
//! `core/tests/formula_plan_differential.rs` and `formula_draw_parity.rs`.

#![allow(
    clippy::unusual_byte_groupings, // mirrors the gate's `0x1234_5678_AB_CD_EF_00`
    clippy::cast_precision_loss,    // mirrors the gate's RNG and index arithmetic
    clippy::cast_lossless,          // same
    clippy::manual_midpoint,        // `(h + l) / 2.0` must stay bit-identical
    clippy::too_many_lines,         // `report` is one long print block on purpose
    clippy::match_same_arms,        // separate arms document distinct causes
    clippy::ignored_unit_patterns,  // `let _ =` on the warm-up calls is deliberate
)]

use finkit::formula::{
    ChipData, FormulaContext, FormulaDialect, FormulaEngine, FormulaExecutionMode,
};
use ndarray::Array1;
use serde_json::Value;
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

// ───────────────────────── allocation counting ─────────────────────────

/// Counts allocations and requested bytes while `MEASURING` is set.
///
/// Mirrors `core/tests/memory_regression.rs` and `core/examples/memory_profile.rs`
/// so the three cannot disagree about what "one allocation" means.
struct CountingAllocator;

static MEASURING: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if MEASURING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        // SAFETY: forward the exact request to the system allocator.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if MEASURING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        // SAFETY: forward the exact request to the system allocator.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr`/`layout` come from the corresponding system allocation.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if MEASURING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(new_size, Ordering::Relaxed);
        }
        // SAFETY: forward the exact request to the system allocator.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Run `body` with allocation counting on, returning `(allocations, bytes)`.
fn count_allocations<T>(body: impl FnOnce() -> T) -> ((usize, usize), T) {
    ALLOCATIONS.store(0, Ordering::SeqCst);
    BYTES.store(0, Ordering::SeqCst);
    MEASURING.store(true, Ordering::SeqCst);
    let value = body();
    MEASURING.store(false, Ordering::SeqCst);
    (
        (
            ALLOCATIONS.load(Ordering::SeqCst),
            BYTES.load(Ordering::SeqCst),
        ),
        value,
    )
}

// ───────────────────────── corpus loading ─────────────────────────

fn read_fixture(path: &Path) -> HashMap<String, Vec<f64>> {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let mut header: Option<Vec<String>> = None;
    let mut columns = HashMap::<String, Vec<f64>>::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split(',').map(str::trim).collect::<Vec<_>>();
        match &header {
            None => {
                for field in &fields {
                    columns.insert((*field).to_string(), Vec::new());
                }
                header = Some(fields.iter().map(|f| (*f).to_string()).collect());
            }
            Some(names) => {
                assert_eq!(
                    fields.len(),
                    names.len(),
                    "CSV row width mismatch in {}",
                    path.display()
                );
                for (name, value) in names.iter().zip(fields.iter()) {
                    if let Ok(value) = value.parse::<f64>() {
                        columns.get_mut(name).unwrap().push(value);
                    }
                }
            }
        }
    }
    assert!(
        !columns.is_empty(),
        "CSV fixture has no columns: {}",
        path.display()
    );
    columns
}

/// Map a corpus `platform` tag onto an implemented dialect.
///
/// `dzh` has no `FormulaDialect` variant; its cases run under `TongDaXin`, exactly
/// as `formula_plan_differential.rs` does, so the two tools agree.
fn dialect_for(platform: &str) -> FormulaDialect {
    match platform.to_ascii_lowercase().as_str() {
        "tdx" | "cross" | "dzh" => FormulaDialect::TongDaXin,
        "ths" => FormulaDialect::TongHuaShun,
        "eastmoney" | "em" => FormulaDialect::EastMoney,
        "pine" | "tradingview" => FormulaDialect::Pine,
        other => panic!("unsupported formula corpus platform: {other}"),
    }
}

/// Repeat a series `scale` times so the same corpus can be measured at a
/// realistic long-history length without inventing new fixtures.
fn tile(values: &Array1<f64>, scale: usize) -> Array1<f64> {
    if scale <= 1 {
        return values.clone();
    }
    let slice = values.as_slice().expect("Array1 is contiguous");
    let mut out = Vec::with_capacity(slice.len() * scale);
    for _ in 0..scale {
        out.extend_from_slice(slice);
    }
    Array1::from_vec(out)
}

fn context_from(columns: &HashMap<String, Vec<f64>>, scale: usize) -> FormulaContext {
    let values = |name: &str| {
        let raw = Array1::from_vec(
            columns
                .get(name)
                .unwrap_or_else(|| panic!("required OHLCV column missing: {name}"))
                .clone(),
        );
        tile(&raw, scale)
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

/// Deterministic synthetic OHLCV, byte-identical to the Pine corpus runner in
/// `formula_plan_differential.rs` so the two tools cannot disagree on inputs.
fn synthetic_ohlcv(n: usize) -> FormulaContext {
    let mut seed: u64 = 0x1234_5678_AB_CD_EF_00;
    let mut rng = || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 33) as f64 / (u64::MAX as f64)
    };

    let (mut open, mut high, mut low, mut close, mut vol) = (
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
        open.push(o);
        high.push(hi);
        low.push(lo);
        close.push(c);
        vol.push(1000.0 + rng() * 500.0);
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

/// Build a chip-distribution table spanning the fixture's own price range.
///
/// The corpus marks `chip_distribution_ths` with `"chip_data": true`, but no
/// consumer reads that flag, so both backends evaluate `WINNER`/`COST` with no
/// chip data and return all-NaN. The differential gate counts that case as
/// verified; `core/tests/formula_host_context.rs` already documents why the
/// corpus cannot prove host-context parity and covers it separately with real
/// host data. This probe supplies the chip table so the *workload measurement*
/// is not itself a vacuous green — without it, one of 35 domestic cases would
/// be reported as agreeing while computing nothing.
fn chip_data_for(columns: &HashMap<String, Vec<f64>>) -> ChipData {
    let close = columns
        .get("close")
        .expect("chip-distribution cases need a close column");
    let low = close.iter().copied().fold(f64::INFINITY, f64::min);
    let high = close.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let levels = 7;
    let price_levels: Vec<f64> = (0..levels)
        .map(|index| low + (high - low) * index as f64 / (levels - 1) as f64)
        .collect();
    // Monotone cumulative ratios, the shape `ChipData::winner` expects.
    let volume_ratios: Vec<f64> = (0..levels)
        .map(|index| (index + 1) as f64 / levels as f64)
        .collect();
    ChipData::with_data(price_levels, volume_ratios, 1_000_000.0)
}

/// One evaluable corpus case.
struct Case {
    id: String,
    source: String,
    dialect: FormulaDialect,
    /// Fixture columns, loaded once so the timed loop never touches the disk.
    /// `None` for Pine cases, which use the shared synthetic OHLCV.
    columns: Option<HashMap<String, Vec<f64>>>,
    /// The corpus asked for a chip-distribution table.
    needs_chip: bool,
    tolerance: f64,
    corpus: &'static str,
}

fn corpus_files(dir: &str, extension: &str) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    let mut files = std::fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("{}: {error}", root.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|found| found == extension))
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn load_domestic(repo_root: &Path) -> Vec<Case> {
    let mut cases = Vec::new();
    for path in corpus_files("../tests/formula_corpus", "json") {
        let text = std::fs::read_to_string(&path).unwrap();
        let case: Value = serde_json::from_str(&text)
            .unwrap_or_else(|error| panic!("invalid corpus JSON {}: {error}", path.display()));
        let relative = case["input"]["data_file"].as_str().unwrap();
        cases.push(Case {
            id: case["id"].as_str().unwrap().to_string(),
            source: case["source_formula"].as_str().unwrap().to_string(),
            dialect: dialect_for(case["platform"].as_str().unwrap()),
            // Read once here: the timed loop must measure the engine, not the
            // filesystem. A cold run that included a 250-row CSV parse was
            // ~1.8 ms of I/O pretending to be engine cost.
            columns: Some(read_fixture(&repo_root.join(relative))),
            needs_chip: case["input"]["chip_data"].as_bool().unwrap_or(false),
            tolerance: case["tolerance"].as_f64().unwrap_or(1e-10),
            corpus: "domestic",
        });
    }
    cases
}

fn load_pine() -> Vec<Case> {
    let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/pine_corpus");
    let manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(corpus_dir.join("manifest.json")).unwrap())
            .expect("Pine corpus manifest must be valid JSON");
    let skipped: Vec<String> = manifest["scripts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["status"].as_str() == Some("host_required"))
        .map(|entry| entry["id"].as_str().unwrap().to_string())
        .collect();

    let mut cases = Vec::new();
    for path in corpus_files("../tests/pine_corpus", "pine") {
        let id = path.file_stem().unwrap().to_string_lossy().to_string();
        // `host_required` scripts need an external security resolver that
        // neither backend provides, so they are out of scope, not a gap.
        if skipped.contains(&id) {
            continue;
        }
        cases.push(Case {
            id,
            source: std::fs::read_to_string(&path).unwrap(),
            dialect: FormulaDialect::Pine,
            columns: None,
            needs_chip: false,
            tolerance: 1e-10,
            corpus: "pine",
        });
    }
    cases
}

// ───────────────────────── measurement ─────────────────────────

/// How many times each timed path is repeated.
const COLD_ITERATIONS: usize = 20;
const WARM_ITERATIONS: usize = 50;
const WARMUP_ITERATIONS: usize = 5;

/// Build a context for a case from the columns loaded at startup.
///
/// Called fresh for every timed iteration so the measured span never includes
/// another iteration's mutated `variables` map, and never touches the disk.
fn case_context(case: &Case, scale: usize) -> FormulaContext {
    match &case.columns {
        Some(columns) => {
            let ctx = context_from(columns, scale);
            if case.needs_chip {
                ctx.with_chip_data(chip_data_for(columns))
            } else {
                ctx
            }
        }
        None => synthetic_ohlcv(120 * scale),
    }
}

fn engine(mode: FormulaExecutionMode) -> FormulaEngine {
    FormulaEngine::new().with_execution_mode(mode)
}

/// One mode's result for one case.
struct Run {
    values: Vec<f64>,
    finite: usize,
}

fn evaluate(mode: FormulaExecutionMode, case: &Case, scale: usize) -> Result<Run, String> {
    let mut ctx = case_context(case, scale);
    let mut engine = engine(mode);
    let values = engine
        .eval_with_dialect(&case.source, case.dialect, &mut ctx)
        .map_err(|error| error.to_string())?;
    let finite = values.iter().filter(|value| value.is_finite()).count();
    Ok(Run {
        values: values.to_vec(),
        finite,
    })
}

/// Mean nanoseconds of `iterations` calls, each on a fresh engine (cold) or on
/// one warm engine (warm).
///
/// Cold is a fresh engine *and* a fresh context per call, so it is the cost a
/// one-shot caller pays. Warm reuses both, so it is the cost a chart refresh
/// pays. Both are reported because a default flip has to be acceptable in both.
/// Neither includes file I/O: the fixture is read once at startup.
fn time_mode(mode: FormulaExecutionMode, case: &Case, scale: usize) -> (f64, f64, usize) {
    for _ in 0..WARMUP_ITERATIONS {
        let _ = evaluate(mode, case, scale);
    }

    let cold_start = Instant::now();
    for _ in 0..COLD_ITERATIONS {
        let _ = evaluate(mode, case, scale);
    }
    let cold_ns = cold_start.elapsed().as_secs_f64() * 1e9 / COLD_ITERATIONS as f64;

    let mut ctx = case_context(case, scale);
    let mut engine = engine(mode);
    let warm_start = Instant::now();
    for _ in 0..WARM_ITERATIONS {
        let _ = engine.eval_with_dialect(&case.source, case.dialect, &mut ctx);
    }
    let warm_ns = warm_start.elapsed().as_secs_f64() * 1e9 / WARM_ITERATIONS as f64;

    // Allocations for one warm execute, measured on its own so the timing loop
    // above cannot pollute the counter.
    let ((allocs, _bytes), _) = count_allocations(|| {
        let _ = engine.eval_with_dialect(&case.source, case.dialect, &mut ctx);
    });

    (cold_ns, warm_ns, allocs)
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

/// One case's verdict plus its measurements.
#[derive(Clone)]
struct Outcome {
    id: String,
    corpus: &'static str,
    verdict: Verdict,
    tree_cold_ns: f64,
    plan_cold_ns: f64,
    tree_warm_ns: f64,
    plan_warm_ns: f64,
    tree_allocs: usize,
    plan_allocs: usize,
}

#[derive(Clone)]
enum Verdict {
    /// Both backends produced values that agree, and at least one is finite.
    Agree,
    /// Both agree, but both are entirely non-finite: parity on nothing.
    Degenerate {
        tree_finite: usize,
        plan_finite: usize,
    },
    /// The Tree reference could not run, so there is nothing to compare.
    TreeUnavailable(String),
    /// Tree ran; Plan could not.
    PlanUnavailable(String),
    /// Both ran and disagreed.
    Divergent(String),
    /// Lengths differ.
    LengthMismatch { tree: usize, plan: usize },
}

impl Verdict {
    fn is_agree(&self) -> bool {
        matches!(self, Self::Agree)
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Agree => "agree",
            Self::Degenerate { .. } => "DEGENERATE",
            Self::TreeUnavailable(_) => "tree-unavailable",
            Self::PlanUnavailable(_) => "PLAN-GAP",
            Self::Divergent(_) => "DIVERGENT",
            Self::LengthMismatch { .. } => "LENGTH-MISMATCH",
        }
    }

    fn detail(&self) -> Option<String> {
        match self {
            Self::Agree => None,
            Self::Degenerate {
                tree_finite,
                plan_finite,
            } => Some(format!(
                "both paths agree but tree has {tree_finite} finite value(s) and plan {plan_finite}"
            )),
            Self::TreeUnavailable(reason) => Some(reason.clone()),
            Self::PlanUnavailable(reason) => Some(reason.clone()),
            Self::Divergent(reason) => Some(reason.clone()),
            Self::LengthMismatch { tree, plan } => {
                Some(format!("tree len={tree}, plan len={plan}"))
            }
        }
    }
}

fn measure_case(case: &Case, scale: usize) -> Outcome {
    let (tree_cold_ns, tree_warm_ns, tree_allocs) =
        time_mode(FormulaExecutionMode::Tree, case, scale);
    let (plan_cold_ns, plan_warm_ns, plan_allocs) =
        time_mode(FormulaExecutionMode::Plan, case, scale);

    let verdict = match evaluate(FormulaExecutionMode::Tree, case, scale) {
        Err(reason) => Verdict::TreeUnavailable(reason),
        Ok(tree) => match evaluate(FormulaExecutionMode::Plan, case, scale) {
            Err(reason) => Verdict::PlanUnavailable(reason),
            Ok(plan) => {
                if tree.values.len() == plan.values.len() {
                    let divergence = tree
                        .values
                        .iter()
                        .zip(plan.values.iter())
                        .position(|(a, b)| !values_match(*a, *b, case.tolerance))
                        .map(|index| {
                            format!(
                                "index {index}: tree={} plan={} (tol={})",
                                tree.values[index], plan.values[index], case.tolerance
                            )
                        });
                    match divergence {
                        Some(reason) => Verdict::Divergent(reason),
                        None if tree.finite == 0 && plan.finite == 0 => Verdict::Degenerate {
                            tree_finite: tree.finite,
                            plan_finite: plan.finite,
                        },
                        None => Verdict::Agree,
                    }
                } else {
                    Verdict::LengthMismatch {
                        tree: tree.values.len(),
                        plan: plan.values.len(),
                    }
                }
            }
        },
    };

    Outcome {
        id: case.id.clone(),
        corpus: case.corpus,
        verdict,
        tree_cold_ns,
        plan_cold_ns,
        tree_warm_ns,
        plan_warm_ns,
        tree_allocs,
        plan_allocs,
    }
}

// ───────────────────────── reporting ─────────────────────────

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).expect("no NaN timings"));
    values[values.len() / 2]
}

fn report(corpus: &str, outcomes: &[Outcome]) {
    println!("\n───────────────────────────────────────────────────────────────");
    println!(" 语料: {corpus}  ({} 个用例)", outcomes.len());
    println!("───────────────────────────────────────────────────────────────");

    let agree = outcomes.iter().filter(|o| o.verdict.is_agree()).count();
    let degenerate = outcomes
        .iter()
        .filter(|o| matches!(o.verdict, Verdict::Degenerate { .. }))
        .count();
    let tree_unavailable = outcomes
        .iter()
        .filter(|o| matches!(o.verdict, Verdict::TreeUnavailable(_)))
        .count();
    let plan_gap = outcomes
        .iter()
        .filter(|o| matches!(o.verdict, Verdict::PlanUnavailable(_)))
        .count();
    let divergent = outcomes
        .iter()
        .filter(|o| {
            matches!(
                o.verdict,
                Verdict::Divergent(_) | Verdict::LengthMismatch { .. }
            )
        })
        .count();

    println!("  覆盖/一致性");
    println!(
        "    Plan 可运行且与 Tree 一致(非退化) : {agree}/{len}",
        len = outcomes.len()
    );
    println!("    退化(两路径一致但全非有限)         : {degenerate}");
    println!("    Tree 不可运行(无可比基准)          : {tree_unavailable}");
    println!("    Plan 缺口(PlanUnavailable)         : {plan_gap}");
    println!("    数值分歧(Divergent)                : {divergent}");

    let mut issues: Vec<&Outcome> = outcomes
        .iter()
        .filter(|o| !o.verdict.is_agree() && !matches!(o.verdict, Verdict::TreeUnavailable(_)))
        .collect();
    issues.sort_by(|a, b| a.id.cmp(&b.id));
    if !issues.is_empty() {
        println!("\n  需要关注的用例:");
        for outcome in issues {
            println!(
                "    [{:<14}] {:<28} {}",
                outcome.verdict.label(),
                outcome.id,
                outcome.verdict.detail().unwrap_or_default()
            );
        }
    }

    // Only cases both backends could run have comparable timings.
    let comparable: Vec<&Outcome> = outcomes
        .iter()
        .filter(|o| !matches!(o.verdict, Verdict::TreeUnavailable(_)))
        .collect();
    if comparable.is_empty() {
        return;
    }

    let mut tree_cold: Vec<f64> = comparable.iter().map(|o| o.tree_cold_ns).collect();
    let mut plan_cold: Vec<f64> = comparable.iter().map(|o| o.plan_cold_ns).collect();
    let mut tree_warm: Vec<f64> = comparable.iter().map(|o| o.tree_warm_ns).collect();
    let mut plan_warm: Vec<f64> = comparable.iter().map(|o| o.plan_warm_ns).collect();
    let tree_alloc_mean = mean(
        &comparable
            .iter()
            .map(|o| o.tree_allocs as f64)
            .collect::<Vec<_>>(),
    );
    let plan_alloc_mean = mean(
        &comparable
            .iter()
            .map(|o| o.plan_allocs as f64)
            .collect::<Vec<_>>(),
    );

    println!("\n  性能 (每个用例的均值/中位数, 单位 µs)");
    println!(
        "    {:<12}{:>12}{:>14}{:>12}",
        "路径", "cold(mean)", "cold(median)", "warm(median)"
    );
    println!(
        "    {:<12}{:>12.1}{:>14.1}{:>12.1}",
        "Tree",
        mean(&tree_cold) / 1000.0,
        median(&mut tree_cold) / 1000.0,
        median(&mut tree_warm) / 1000.0
    );
    println!(
        "    {:<12}{:>12.1}{:>14.1}{:>12.1}",
        "Plan",
        mean(&plan_cold) / 1000.0,
        median(&mut plan_cold) / 1000.0,
        median(&mut plan_warm) / 1000.0
    );

    let total_tree_warm: f64 = comparable.iter().map(|o| o.tree_warm_ns).sum();
    let total_plan_warm: f64 = comparable.iter().map(|o| o.plan_warm_ns).sum();
    let total_tree_cold: f64 = comparable.iter().map(|o| o.tree_cold_ns).sum();
    let total_plan_cold: f64 = comparable.iter().map(|o| o.plan_cold_ns).sum();
    println!("\n  性能汇总 (全部可比用例之和)");
    println!(
        "    cold: Tree {:>10.1} µs   Plan {:>10.1} µs   Plan/Tree = {:.3}x",
        total_tree_cold / 1000.0,
        total_plan_cold / 1000.0,
        total_plan_cold / total_tree_cold.max(f64::EPSILON)
    );
    println!(
        "    warm: Tree {:>10.1} µs   Plan {:>10.1} µs   Plan/Tree = {:.3}x",
        total_tree_warm / 1000.0,
        total_plan_warm / 1000.0,
        total_plan_warm / total_tree_warm.max(f64::EPSILON)
    );

    println!("\n  内存 (每次 warm 执行的分配次数均值)");
    println!(
        "    Tree {:>8.1}   Plan {:>8.1}   Plan/Tree = {:.3}x",
        tree_alloc_mean,
        plan_alloc_mean,
        plan_alloc_mean / tree_alloc_mean.max(f64::EPSILON)
    );

    // Slowest regressions, so a single bad formula cannot hide in the mean.
    let mut regressions: Vec<(&Outcome, f64)> = comparable
        .iter()
        .map(|o| (*o, o.plan_warm_ns / o.tree_warm_ns.max(f64::EPSILON)))
        .filter(|(_, ratio)| *ratio > 1.5)
        .collect();
    regressions.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("no NaN ratios"));
    if !regressions.is_empty() {
        println!("\n  warm 路径 Plan 比 Tree 慢 >1.5x 的用例:");
        for (outcome, ratio) in regressions.iter().take(10) {
            println!(
                "    {:<28} {:>6.2}x   (Tree {:.1} µs / Plan {:.1} µs)",
                outcome.id,
                ratio,
                outcome.tree_warm_ns / 1000.0,
                outcome.plan_warm_ns / 1000.0
            );
        }
    }
}

/// Rows in the native fixtures; a `--scaled` pass repeats them to this length.
const SCALED_ROWS: usize = 5000;

fn run_pass(cases: &[Case], scale: usize, label: &str) {
    let rows = cases
        .iter()
        .find_map(|case| case.columns.as_ref())
        .and_then(|columns| columns.get("close"))
        .map_or(120, Vec::len)
        * scale;

    println!("\n\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║  测量档位: {label}  (每用例 {rows} 行)");
    println!("╚═══════════════════════════════════════════════════════════════╝");

    let outcomes: Vec<Outcome> = cases.iter().map(|case| measure_case(case, scale)).collect();

    report(
        "domestic",
        &outcomes
            .iter()
            .filter(|o| o.corpus == "domestic")
            .cloned()
            .collect::<Vec<_>>(),
    );
    report(
        "pine",
        &outcomes
            .iter()
            .filter(|o| o.corpus == "pine")
            .cloned()
            .collect::<Vec<_>>(),
    );

    let total_agree = outcomes.iter().filter(|o| o.verdict.is_agree()).count();
    println!("\n═══════════════════════════════════════════════════════════════");
    println!(
        " [{label}] 全部语料: {total_agree}/{} 个用例 Plan 可运行且与 Tree 一致(非退化)",
        outcomes.len()
    );
    println!("═══════════════════════════════════════════════════════════════");
}

fn main() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let scaled = std::env::args().any(|arg| arg == "--scaled");

    let mut cases = load_domestic(&repo_root);
    cases.extend(load_pine());

    println!("═══════════════════════════════════════════════════════════════");
    println!(" finkit 公式后端 workload 测量 (Tree vs Plan) — 阶段 1.4 证据");
    println!("═══════════════════════════════════════════════════════════════");
    println!(
        " 用例总数 {} (domestic {} / pine {})",
        cases.len(),
        cases.iter().filter(|c| c.corpus == "domestic").count(),
        cases.iter().filter(|c| c.corpus == "pine").count()
    );
    println!(
        " 计时: cold={COLD_ITERATIONS} 次(每次新建 engine+ctx), warm={WARM_ITERATIONS} 次(复用), \
         预热={WARMUP_ITERATIONS} 次"
    );
    println!(" 默认执行后端: {:?}", FormulaExecutionMode::default());
    println!(" 加 --scaled 可追加 {SCALED_ROWS} 行长历史档位");

    run_pass(&cases, 1, "native");
    if scaled {
        let scale = SCALED_ROWS / 250;
        run_pass(&cases, scale, "scaled");
    }
}
