//! Three-way SSOT gate across the three surfaces that must agree.
//!
//! 1. [`builtin_function_registry`] — the **indicator/operation** SSOT. Drives
//!    discovery (`schema.rs`), the operation registry (`operation.rs`), the FFI
//!    surfaces, and — load-bearing here — the planner's purity decision in
//!    `formula::compute_ir::function_metadata`.
//! 2. [`get_builtin_functions`] — the **formula-language** surface: the names a
//!    formula script may call. (`formula/mod.rs` aliases `functions_router.rs`
//!    as `functions` and keeps `functions.rs` private as `functions_legacy`.)
//! 3. `FormulaKernelDispatcher` — the numeric kernels the compiled-plan path can
//!    actually execute.
//!
//! The three are *not* required to be equal: each exists for a different
//! consumer, and a name can legitimately live on one surface only. For example
//! `ACCBANDS` and `MAMA` are real operations implemented in `indicators/` and
//! reachable through `operation.rs` and the FFI, but they are not
//! formula-language functions, so the alias-injection loop in
//! `get_builtin_functions` deliberately skips them.
//!
//! What *is* required is that every difference is **deliberate and recorded**.
//! These two invariants must hold outright:
//!
//! * every plan kernel is registered in the SSOT (otherwise the planner would
//!   treat an executable operation as unknown and lower it to a stateful
//!   barrier), and
//! * every plan kernel has a formula-language implementation (otherwise the
//!   plan path could execute something no script can name).
//!
//! The remaining differences are pinned to explicit lists below. Each list is
//! non-rotting in the same sense as the differential allowlist in
//! `formula_plan_differential.rs`: an entry that stops being needed fails the
//! test, so a list can only shrink by deliberate edit.

use finkit::buffer_arena::BufferSlot;
use finkit::execution_plan::KernelId;
use finkit::formula::{get_builtin_functions, FormulaKernelDispatcher};
use finkit::registry::builtin_function_registry;
use finkit::state_arena::StateArena;
use finkit::unified_executor::{KernelCall, KernelDispatcher};
use std::collections::BTreeSet;
use std::path::Path;

/// `FormulaKernelDispatcher::ERR_UNSUPPORTED_KERNEL`.
///
/// The constant is private inside the trait impl, so the value is pinned here.
/// It is part of the dispatcher's error contract, so pinning it is the point:
/// if the code ever changes, this gate fails loudly instead of silently
/// classifying every kernel as unsupported.
const UNSUPPORTED_KERNEL_CODE: u32 = 1;

/// Number of physical buffers used by the probe. Six is more than the widest
/// kernel arity, so every handler reaches its own arity check rather than
/// indexing out of bounds.
const PROBE_SLOTS: usize = 6;

/// Kernel operations the compiled-plan path can execute today.
///
/// Derived behaviourally by [`probed_plan_kernels`], then compared against this
/// list. Adding a kernel therefore requires editing this list, which is the
/// deliberate step that keeps the coverage claim honest.
const PLAN_KERNELS: &[&str] = &[
    "ABS",
    "AD",
    "ADD",
    "ADOSC",
    "ADX",
    "AROON_DN",
    "AROON_UP",
    "ATR",
    "BARSSINCE",
    "BBANDS",
    "BIGORDER",
    "BOLLDN",
    "BOLLMID",
    "BOLLUP",
    "CCI",
    "CHOP",
    "CMF",
    "CORREL",
    "COSH",
    "CROSS",
    "CROSSBELOW",
    "CUMSUM",
    // Host-context kernels: they read data the numeric input slots cannot carry
    // (see `HostContext`), so they are only executable when the caller supplies it.
    "COST",
    "DEA",
    "DIV",
    "DONCHIAN",
    "DONCHIAN_LOWER",
    "DONCHIAN_MIDDLE",
    "DONCHIAN_UPPER",
    "DONCHIAN_WIDTH",
    "EMA",
    "FISHER",
    "FIXNAN",
    "FISHER_SIGNAL",
    "FRACPART",
    "HHV",
    "HHVBARS",
    "HMA",
    "ICHIMOKU_KIJUN",
    "ICHIMOKU_TENKAN",
    "IF",
    "INTPART",
    "ISNA",
    "KAMA",
    "KDJ",
    "KDJ_D",
    "KDJ_J",
    // The slope kernel behind Qlib's `Slope`, needed by Alpha158's `BETA{d}`.
    // `LINEARREG_SLOPE` already had an SSOT entry from the TA-Lib track, so
    // only the plan kernel was missing.
    "LINEARREG_SLOPE",
    "LLV",
    "LLVBARS",
    // `LN` is the SSOT spelling of `LOG` on the formula surface (`fn_log` is
    // registered under both names); Qlib's Alpha158 needs it for `CORR`/`CORD`.
    "LN",
    "MA",
    "MACD",
    "MAININFLOW",
    "MAININFLOWPCT",
    "MATH_AVG",
    "MAX",
    "MAXINDEX",
    "MEDIAN",
    "MFI",
    "MIN",
    "MININDEX",
    "MINUS",
    "MINUS_DI",
    "MOD",
    "MONEYFLOW",
    "MOM",
    "MULT",
    "NATR",
    "NETINFLOW",
    "OBV",
    "PERIODTYPE",
    "PLUS_DI",
    // Reference-parity rolling operators for Alpha158. Each delegates to a
    // `math::` kernel that the formula-language implementation also calls, so
    // the two paths share one arithmetic rather than two that agree by luck.
    "QUANTILE",
    "RANK_PCT",
    "REF",
    "REFDATE",
    "RESI",
    "REVERSE",
    "RMA",
    "ROC",
    "ROCP",
    "ROCR",
    "ROCR100",
    "ROLLING_RANGE",
    "RSI",
    "RSQUARE",
    "SAR",
    // `sign(x)` as `-1`/`0`/`1`, needed by the WorldQuant alphas to re-attach
    // the direction of a differenced series.
    "SIGN",
    "SINH",
    "SMA",
    "SMALLORDER",
    "SQRT",
    "STD",
    "STDDEV",
    // pandas'/Qlib's `ddof = 1` standard deviation, i.e. what `Std` means in
    // Alpha158. `STD`/`STDDEV` stay TA-Lib's population convention; the two
    // differ by the exact factor `sqrt((n - 1) / n)`, so both names are needed.
    "STDDEV_SAMPLE",
    "SUB",
    "STOCHF",
    "SUM",
    "SUMBARS",
    "SUPERBIGORDER",
    "SUPERTREND",
    "TANH",
    "TR",
    "TRANGE",
    "TRIMA",
    "TRIX",
    "TSI",
    "VAR",
    "VWAP",
    "VWMA",
    "WILLR",
    "WINNER",
    "WMA",
    "ZSCORE",
];

/// Functions registered in the SSOT **and** callable from a formula, but with
/// no numeric kernel, so the compiled-plan path cannot execute them.
///
/// This is the plan-path coverage backlog: each name is already declared pure
/// and already implemented, so it needs a dispatcher kernel and nothing else.
/// The list is the measurable target for expanding plan coverage.
const DECLARED_BUT_NO_KERNEL: &[&str] = &[
    "ACCBANDS",
    "ACOS",
    "ASIN",
    "ATAN",
    "AVG",
    "AVGDEV",
    "BARSLAST",
    "BEARISH_CROSS",
    "BETA",
    "BOLL",
    "BOLLINGER",
    "BREAKDOWN",
    "BREAKOUT",
    "BREAKOUT_DOWN",
    "BREAKOUT_SCREEN",
    "BREAKOUT_UP",
    "BULLISH_CROSS",
    "CDL2CROWS",
    "CDL3BLACKCROWS",
    "CDL3INSIDE",
    "CDL3LINESTRIKE",
    "CDL3OUTSIDE",
    "CDL3STARSINSOUTH",
    "CDL3WHITESOLDIERS",
    "CDLABANDONEDBABY",
    "CDLADVANCEBLOCK",
    "CDLBELTHOLD",
    "CDLBREAKAWAY",
    "CDLCLOSINGMARUBOZU",
    "CDLCONCEALBABYSWALL",
    "CDLCOUNTERATTACK",
    "CDLDARKCLOUDCOVER",
    "CDLDOJI",
    "CDLDOJISTAR",
    "CDLDRAGONFLYDOJI",
    "CDLENGULFING",
    "CDLEVENINGDOJISTAR",
    "CDLEVENINGSTAR",
    "CDLGAPSIDESIDEWHITE",
    "CDLGRAVESTONEDOJI",
    "CDLHAMMER",
    "CDLHANGINGMAN",
    "CDLHARAMI",
    "CDLHARAMICROSS",
    "CDLHIGHWAVE",
    "CDLHIKKAKE",
    "CDLHIKKAKEMOD",
    "CDLHOMINGPIGEON",
    "CDLIDENTICAL3CROWS",
    "CDLINNECK",
    "CDLINVERTEDHAMMER",
    "CDLKICKING",
    "CDLKICKINGBYLENGTH",
    "CDLLADDERBOTTOM",
    "CDLLONGLEGGEDDOJI",
    "CDLLONGLINE",
    "CDLMARUBOZU",
    "CDLMATCHINGLOW",
    "CDLMATHOLD",
    "CDLMORNINGDOJISTAR",
    "CDLMORNINGSTAR",
    "CDLONNECK",
    "CDLPIERCING",
    "CDLRICKSHAWMAN",
    "CDLRISEFALL3METHODS",
    "CDLSEPARATINGLINES",
    "CDLSHOOTINGSTAR",
    "CDLSHORTLINE",
    "CDLSPINNINGTOP",
    "CDLSTALLEDPATTERN",
    "CDLSTICKSANDWICH",
    "CDLTAKURI",
    "CDLTASUKIGAP",
    "CDLTHRUSTING",
    "CDLTRISTAR",
    "CDLUNIQUE3RIVER",
    "CDLUPSIDEGAP2CROWS",
    "CDLXSIDEGAP3METHODS",
    "CEIL",
    "CHOPPINESS",
    "CMO",
    "COS",
    "COUNT",
    "CROSSDOWN",
    "CROSSOVER",
    "CROSSUP",
    "DEAD_CROSS",
    "DEMA",
    "DONCHIAN_MID",
    "EXP",
    "FISHER_TRANSFORM",
    "FLOOR",
    "GAP",
    "GAP_SIGNAL",
    "GOLDEN_CROSS",
    "HT_DCPERIOD",
    "HT_DCPHASE",
    "HT_PHASOR",
    "HT_SINE",
    "HT_TRENDLINE",
    "HT_TRENDMODE",
    "IFF",
    "IMI",
    "KD",
    "KDJ_K",
    "KIJUN",
    "KIJUN_SEN",
    "LINEARREG",
    "LINEARREG_ANGLE",
    "LINEARREG_INTERCEPT",
    "LOG10",
    "MACDEXT",
    "MACDFIX",
    "MAMA",
    "MAVP",
    "MA_ALIGN",
    "MA_ALIGNMENT",
    "MINMAX",
    "MINMAXINDEX",
    "MINUS_DM",
    "MOMENTUM",
    "PLUS_DM",
    "PPO",
    "PRICE_BREAKDOWN",
    "PRICE_BREAKOUT",
    "RELATIVE_STRENGTH",
    "RELSTRENGTH",
    "RS_EXCESS_RETURN",
    "SAREXT",
    "SHIFT",
    "SIN",
    "SUPERTREND_LINE",
    "T3",
    "TAN",
    "TEMA",
    "TENKAN",
    "TREND_ALIGN",
    "TREND_BREAKOUT",
    "TREND_SCREEN",
    "TSF",
    "ULTOSC",
    "VOLSURGE",
    "VOLUME_EXPANSION",
    "VOLUME_SURGE",
    "Z_SCORE",
];

/// Registry canonical names plus aliases.
fn registered_names() -> BTreeSet<String> {
    let registry = builtin_function_registry();
    let mut names = BTreeSet::new();
    for spec in registry.iter() {
        names.insert(spec.name.to_string());
        for alias in spec.aliases {
            names.insert((*alias).to_string());
        }
    }
    names
}

/// Formula-language callable names.
fn formula_names() -> BTreeSet<String> {
    get_builtin_functions().keys().cloned().collect()
}

/// Ask the dispatcher itself which `CALL:<name>` operations it handles.
///
/// Probing behaviour beats reading a declared list: a list could drift from the
/// `if` chain in `dispatch`, whereas this cannot. A handler that rejects the
/// probe for arity or parameter reasons still proves the branch exists, so only
/// [`UNSUPPORTED_KERNEL_CODE`] means "no kernel".
fn probed_plan_kernels(candidates: &BTreeSet<String>) -> BTreeSet<String> {
    let mut handled = BTreeSet::new();
    let buffers = vec![vec![1.0_f64; 32]; PROBE_SLOTS];
    let inputs: Vec<BufferSlot> = (0..PROBE_SLOTS - 1).map(BufferSlot).collect();
    let output = BufferSlot(PROBE_SLOTS - 1);

    for name in candidates {
        // Host context is irrelevant to probing: a kernel that *handles* the
        // call is covered whether or not host data is present.
        let mut dispatcher = FormulaKernelDispatcher::new();
        let mut states = StateArena::new();
        let mut scratch = buffers.clone();
        let call = KernelCall {
            kernel: KernelId::from_static(&format!("CALL:{name}")),
            inputs: &inputs,
            output,
            parameters: &[],
            state: None,
        };
        match dispatcher.dispatch(call, &mut scratch, &mut states) {
            Err(error) if error.code == UNSUPPORTED_KERNEL_CODE => {}
            _ => {
                handled.insert(name.clone());
            }
        }
    }
    handled
}

/// Every name either surface can produce.
///
/// The probe must run over the *union*: probing only the registry would make
/// "every kernel is registered" vacuously true, since the probe can only ever
/// return names it was given. Running over the union means a kernel added under
/// a formula-only name (registered nowhere) is caught.
///
/// A kernel whose name appears on *neither* surface still cannot be discovered
/// by probing, because nothing enumerates it. That residue is covered by
/// [`the_three_surfaces_have_the_expected_sizes`], which pins the kernel count.
fn all_candidate_names() -> BTreeSet<String> {
    let mut names = registered_names();
    names.extend(formula_names());
    names
}

/// Symmetric difference formatted for an assertion message.
fn assert_recorded(expected: &[&str], actual: &BTreeSet<String>, what: &str) {
    let expected_set: BTreeSet<String> = expected.iter().map(|name| (*name).to_string()).collect();
    let missing: Vec<&String> = expected_set.difference(actual).collect();
    let unexpected: Vec<&String> = actual.difference(&expected_set).collect();
    assert!(
        missing.is_empty() && unexpected.is_empty(),
        "{what} changed; update the recorded list deliberately.\n\
         recorded but no longer true: {missing:?}\n\
         true but not recorded: {unexpected:?}"
    );
}

#[test]
fn every_plan_kernel_is_registered_in_the_ssot() {
    let kernels = probed_plan_kernels(&all_candidate_names());
    let registry = registered_names();

    // Sanity: a broken probe would make this test vacuously green.
    assert!(
        kernels.len() >= 40,
        "the dispatcher probe found only {} kernels, which means the probe is \
         not exercising the dispatcher; check KernelCall construction",
        kernels.len()
    );

    let unregistered: Vec<&String> = kernels.difference(&registry).collect();
    assert!(
        unregistered.is_empty(),
        "these kernels execute but are not declared in the SSOT, so the planner \
         lowers them to stateful barriers: {unregistered:?}"
    );
}

#[test]
fn every_plan_kernel_has_a_formula_implementation() {
    let kernels = probed_plan_kernels(&all_candidate_names());
    let formulas = formula_names();
    let missing: Vec<&String> = kernels.difference(&formulas).collect();
    assert!(
        missing.is_empty(),
        "these kernels execute but no formula can name them: {missing:?}"
    );
}

#[test]
fn plan_kernel_coverage_is_exactly_the_recorded_set() {
    let kernels = probed_plan_kernels(&all_candidate_names());
    assert_recorded(PLAN_KERNELS, &kernels, "plan kernel coverage");
}

#[test]
fn declared_functions_without_a_kernel_are_recorded() {
    let kernels = probed_plan_kernels(&all_candidate_names());
    let declared_and_callable: BTreeSet<String> = registered_names()
        .intersection(&formula_names())
        .cloned()
        .collect();
    let backlog: BTreeSet<String> = declared_and_callable
        .difference(&kernels)
        .cloned()
        .collect();
    assert_recorded(DECLARED_BUT_NO_KERNEL, &backlog, "plan-path backlog");
}

#[test]
fn the_three_surfaces_have_the_expected_sizes() {
    // Sizes are recorded so an accidental mass registration or mass removal is
    // visible as a single, deliberate edit rather than a silent drift.
    //
    // The formula surface is larger than the literal `map.insert` count across
    // `functions_legacy.rs` and `functions.rs` (368) because `get_builtin_functions`
    // then injects the SSOT aliases of every spec whose canonical name resolved
    // (+31 -> 399). Measured, not counted by hand: see the doc-vs-runtime gate in
    // `generated_formula_catalogue_matches_the_runtime_surface`.
    //
    // 419 -> 399: the twenty out-of-scope registrations (FoxTrader trade signals
    // and backtest metrics, the 文华财经 signal-filter directives, and the THS
    // condition-selection directives) were removed. finkit does not do
    // backtesting or stock selection; see the product-boundary section of
    // `.workbuddy-ai/memory/MEMORY.md`.
    let registry = registered_names();
    let formulas = formula_names();
    let kernels = probed_plan_kernels(&registry);

    // Compared as one tuple so a single run reports all three live values.
    // 399 -> 447: the TA-Lib 0.7/0.8 formula bridge
    // (`core/src/formula/functions_talib_081.rs`) registered 48 names whose
    // kernels and golden vectors already existed but which no formula could
    // name. The registry SSOT and the plan-kernel surfaces are unchanged: this
    // was pure formula-surface wiring, not new compute.
    //
    // 447 -> 451 and 107 -> 113: the Alpha158 factor library needed four
    // operators no domestic/TALib spelling covers (`QUANTILE`, `RSQUARE`,
    // `RESI`, `RANK_PCT`), plus plan kernels for those and for the already
    // registered `LINEARREG_SLOPE` and `LN`. The registry grew by the four new
    // names (254 -> 258); the plan-kernel surface grew by six because
    // `LINEARREG_SLOPE` and `LN` were already in the registry but not
    // executable from a compiled plan.
    //
    // 258 -> 259, 451 -> 452 and 113 -> 114: `STDDEV_SAMPLE`. Alpha158's `Std`
    // is pandas' `ddof = 1` standard deviation, while finkit's `STD`/`STDDEV`
    // are TA-Lib's population convention; the ratio is exactly
    // `sqrt((n - 1) / n)`, measured at 0.894427191 for `n = 5`. A ~10% scale
    // error is not something a tolerance can absorb, so the sample convention
    // needed its own name on all three surfaces.
    //
    // 259 -> 260 and 114 -> 115, with the formula surface unchanged: `SIGN`.
    // This one is not a new name, and the asymmetry is the point. `SIGN` was
    // already callable from a formula — `functions_legacy.rs` registers it — but
    // it was absent from the registry SSOT and had no plan kernel, so a compiled
    // factor graph could not execute it. Registering it and giving it a kernel
    // grew the other two surfaces by one each and left the formula surface at
    // 452, where `map.insert("SIGN", ...)` overwrites the legacy entry rather
    // than adding a key.
    //
    // The overwrite is a semantic correction, not a rename: the legacy body was
    // `f64::signum()`, which returns `1.0` for `0.0`, while the dialect
    // contracts and Qlib both describe `0` for `0`. That distinction is load
    // bearing — `Alpha7` multiplies `sign(close - ref(close, 7))` into a factor,
    // and a flat 7-bar stretch must contribute `0`, not a fabricated direction.
    let actual = (registry.len(), formulas.len(), kernels.len());
    assert_eq!(
        actual,
        (260, 452, 115),
        "surface sizes changed: (registry, formula, plan kernels)"
    );
}

/// The generated catalogue must agree with the table the engine actually builds.
///
/// `scripts/gen_ssot_docs.py` parses the sources statically, and static parsing
/// drifts in both directions: an earlier regex invented names from error-message
/// literals (`ensure_args_len("ROCP", ...)` listed a function no script can
/// call) while silently missing every `map.insert("NAME".to_string(), ...)` site.
/// Pinning the document to the live table makes that class of drift impossible
/// to reintroduce: regenerate the docs or the gate fails.
#[test]
fn generated_formula_catalogue_matches_the_runtime_surface() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("docs")
        .join("generated")
        .join("formula-functions.md");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));

    let mut documented: BTreeSet<String> = BTreeSet::new();
    let mut stated_count: Option<usize> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Built-in formula functions: **") {
            stated_count = rest
                .split("**")
                .next()
                .and_then(|value| value.trim().parse().ok());
        }
        if let Some(rest) = line.trim().strip_prefix("| `") {
            if let Some(name) = rest.split('`').next() {
                documented.insert(name.to_string());
            }
        }
    }

    let runtime = formula_names();
    let missing: Vec<&String> = runtime.difference(&documented).collect();
    let invented: Vec<&String> = documented.difference(&runtime).collect();
    assert!(
        missing.is_empty() && invented.is_empty(),
        "docs/generated/formula-functions.md drifted from get_builtin_functions(): \
         documented={} runtime={} missing={:?} invented={:?}. \
         Regenerate with `python scripts/gen_ssot_docs.py --generate`.",
        documented.len(),
        runtime.len(),
        missing,
        invented
    );
    assert_eq!(
        stated_count,
        Some(runtime.len()),
        "the stated count in docs/generated/formula-functions.md must match the runtime surface"
    );
}
