//! Pine Script v5 corpus regression runner.
//!
//! Parses, maps and (where supported) end-to-end evaluates every script under
//! `tests/pine_corpus/` through the AlphaTA formula engine
//! (`parse_pine` → `map_pine_to_alphata` → `FormulaEngine::eval_ast`).
//!
//! The runner is the source of truth for `tests/pine_corpus/manifest.json`.
//! It asserts the pipeline has not regressed versus the previously recorded
//! baseline and prints a per-script table plus the recomputed pass rate.

use finkit::formula::pine::{map_pine_to_alphata, parse_pine};
use finkit::formula::{FormulaContext, FormulaEngine};
use ndarray::Array1;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// Deterministic synthetic OHLCV so the runner is reproducible and CI-friendly.
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

    let ctx = FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(vol),
        None,
    );
    // Derived price sources (`hl2`, `hlc3`, `ohlc4`) are deliberately *not*
    // pre-bound here. Doing so used to paper over the mapper emitting
    // unresolvable `HL2`/`HLC3`/`OHLC4` variable references: the scripts passed
    // only because this harness injected the very series the engine should have
    // derived. The mapper now expands them into OPEN/HIGH/LOW/CLOSE arithmetic,
    // so the corpus is a real gate for them.
    ctx
}

#[test]
fn pine_corpus_parse_map_eval() {
    let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/pine_corpus");
    assert!(
        corpus_dir.exists(),
        "corpus dir not found at {:?}",
        corpus_dir
    );
    let manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(corpus_dir.join("manifest.json")).unwrap())
            .expect("Pine corpus manifest must be valid JSON");
    let status_by_id: HashMap<String, String> = manifest["scripts"]
        .as_array()
        .expect("Pine corpus manifest scripts must be an array")
        .iter()
        .map(|entry| {
            (
                entry["id"]
                    .as_str()
                    .expect("Pine corpus entry id must be a string")
                    .to_string(),
                entry["status"]
                    .as_str()
                    .expect("Pine corpus entry status must be a string")
                    .to_string(),
            )
        })
        .collect();

    let mut entries: Vec<_> = std::fs::read_dir(&corpus_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "pine").unwrap_or(false))
        .collect();
    entries.sort();
    assert!(!entries.is_empty(), "no .pine files found");
    assert_eq!(
        status_by_id.len(),
        entries.len(),
        "Pine corpus manifest must describe every .pine script exactly once"
    );

    let mut parse_pass = 0usize;
    let mut map_pass = 0usize;
    let mut eval_pass = 0usize;
    let mut host_required = 0usize;
    let mut results: Vec<(String, bool, bool, bool)> = Vec::new();

    for path in &entries {
        let src = std::fs::read_to_string(path).unwrap();
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let expected_status = status_by_id
            .get(&name)
            .unwrap_or_else(|| panic!("Pine corpus manifest is missing {name}"));
        let expected_status = expected_status.as_str();
        let requires_host = expected_status == "host_required";

        let pine = match parse_pine(&src) {
            Ok(p) => {
                parse_pass += 1;
                p
            }
            Err(e) => {
                println!("PARSE FAIL [{name}]: {e}");
                results.push((name, false, false, false));
                continue;
            }
        };

        let ast = match map_pine_to_alphata(&pine) {
            Ok(a) => {
                map_pass += 1;
                a
            }
            Err(e) => {
                if requires_host {
                    host_required += 1;
                    println!("HOST REQUIRED [{name}]: {e}");
                } else {
                    println!("MAP FAIL [{name}]: {e}");
                }
                results.push((name, true, false, false));
                continue;
            }
        };

        // End-to-end evaluation through the AlphaTA engine.
        let mut ctx = synthetic_ohlcv(120);
        let ok = match FormulaEngine::new().eval_ast(&ast, &mut ctx) {
            Ok(_) => true,
            Err(e) => {
                println!("EVAL FAIL [{name}]: {e}");
                false
            }
        };
        if ok {
            eval_pass += 1;
        }
        results.push((name, true, true, ok));
    }

    let total = entries.len();
    let expected_executable = status_by_id
        .values()
        .filter(|status| status.as_str() != "host_required")
        .count();
    println!("\n===== Pine Corpus Regression =====");
    println!("total={total} parse_pass={parse_pass} map_pass={map_pass} eval_pass={eval_pass} host_required={host_required}");
    println!(
        "overall_eval_pass_rate={:.3}",
        eval_pass as f64 / total as f64
    );
    for (name, p, m, e) in &results {
        println!(
            "  {:<24} parse={} map={} eval={}",
            name,
            if *p { 'Y' } else { 'N' },
            if *m { 'Y' } else { 'N' },
            if *e { 'Y' } else { 'N' }
        );
    }
    println!("==================================\n");

    // Regression gate: manifest-declared host-required scripts must fail
    // before evaluation instead of being silently evaluated on the chart
    // timeframe. All other scripts must keep their parse/map/eval coverage.
    assert_eq!(parse_pass, total, "Pine parse coverage regressed");
    assert_eq!(map_pass, expected_executable, "Pine map coverage regressed");
    assert_eq!(
        eval_pass, expected_executable,
        "Pine eval coverage regressed"
    );
    assert_eq!(host_required, total - expected_executable);
}

/// `barstate.<field>` must survive the whole frontend → backend pipeline.
///
/// The mapper lowers `barstate.islast` to `AstNode::Variable("barstate_islast")`,
/// but nothing downstream classifies that name: `classify_builtin_var` has no
/// `barstate_*` entry and `get_variable` only consults `ctx.variables`. A script
/// using `barstate` therefore parsed, mapped, and then died at evaluation — a
/// broken link between the Pine frontend and the numeric backend that no gate
/// covered, because no corpus script uses `barstate`.
///
/// The assertions are absolute (not cross-path): in a historical batch
/// evaluation `islast` is 1 on exactly the final bar, `isfirst` is 1 on exactly
/// bar 0, and `ishistory` is 1 everywhere.
#[test]
fn pine_barstate_fields_evaluate_end_to_end() {
    const LEN: usize = 64;
    let engine = FormulaEngine::new();

    // (field, index that must be 1, value everywhere else)
    let cases: &[(&str, usize)] = &[("islast", LEN - 1), ("isfirst", 0)];

    for (field, one_at) in cases {
        let source = format!("//@version=5\nindicator(\"BS\")\nplot(barstate.{field})\n");
        let pine = parse_pine(&source).unwrap_or_else(|e| panic!("{field}: parse failed: {e}"));
        let ast = map_pine_to_alphata(&pine).unwrap_or_else(|e| panic!("{field}: map failed: {e}"));

        let mut ctx = synthetic_ohlcv(LEN);
        let values = engine
            .eval_ast(&ast, &mut ctx)
            .unwrap_or_else(|e| panic!("{field}: eval failed: {e}"));
        assert_eq!(values.len(), LEN, "{field}: unexpected length");
        for (index, value) in values.iter().enumerate() {
            let expected = if index == *one_at { 1.0 } else { 0.0 };
            assert!(
                (*value - expected).abs() < 1e-12,
                "{field}: bar {index} = {value}, expected {expected}"
            );
        }
    }

    // `ishistory` is 1 on every bar of a historical evaluation.
    let source = "//@version=5\nindicator(\"BS\")\nplot(barstate.ishistory)\n";
    let pine = parse_pine(source).expect("parse ishistory");
    let ast = map_pine_to_alphata(&pine).expect("map ishistory");
    let mut ctx = synthetic_ohlcv(LEN);
    let values = engine
        .eval_ast(&ast, &mut ctx)
        .expect("ishistory must evaluate");
    assert!(
        values.iter().all(|value| (*value - 1.0).abs() < 1e-12),
        "ishistory must be 1 on every historical bar, got {values:?}"
    );
}

/// Pine's derived price sources and `time` must be *translated*, not looked up.
///
/// `hl2`, `hlc3` and `ohlc4` were renamed to `HL2`/`HLC3`/`OHLC4` and handed to
/// the engine as variable references, which resolve nowhere. The corpus only
/// passed because its harness pre-bound those very series into the context —
/// a workaround that is now removed, so this test asserts the *values* match
/// the `streaming::PriceSource` definitions rather than merely that eval returns.
///
/// `time` was renamed to `DATE`, also a non-existent variable. `DATE` is a
/// zero-argument builtin, so it is now emitted as a call.
#[test]
// The reference series are built from small synthetic prices, so the
// overflow-safe `f64::midpoint` form is not the property under test here.
#[allow(clippy::manual_midpoint)]
fn pine_derived_price_sources_and_time_evaluate_end_to_end() {
    const LEN: usize = 64;
    let engine = FormulaEngine::new();
    // A real datetime series so `time` has something to read.
    let datetime: Array1<i64> = Array1::from_vec(
        (0..LEN)
            .map(|i| 1_700_000_000_i64 + i64::try_from(i).expect("LEN fits in i64") * 86_400)
            .collect(),
    );
    let mut ctx = synthetic_ohlcv(LEN).with_datetime(datetime);

    let expected: Vec<(&str, Vec<f64>)> = vec![
        (
            "hl2",
            (0..LEN).map(|i| (ctx.high[i] + ctx.low[i]) / 2.0).collect(),
        ),
        (
            "hlc3",
            (0..LEN)
                .map(|i| (ctx.high[i] + ctx.low[i] + ctx.close[i]) / 3.0)
                .collect(),
        ),
        (
            "ohlc4",
            (0..LEN)
                .map(|i| (ctx.open[i] + ctx.high[i] + ctx.low[i] + ctx.close[i]) / 4.0)
                .collect(),
        ),
    ];

    for (name, reference) in expected {
        let source = format!("//@version=5\nindicator(\"PS\")\nplot({name})\n");
        let pine = parse_pine(&source).unwrap_or_else(|e| panic!("{name}: parse failed: {e}"));
        let ast = map_pine_to_alphata(&pine).unwrap_or_else(|e| panic!("{name}: map failed: {e}"));
        let values = engine
            .eval_ast(&ast, &mut ctx)
            .unwrap_or_else(|e| panic!("{name}: eval failed: {e}"));
        assert_eq!(values.len(), LEN, "{name}: unexpected length");
        for (index, (got, want)) in values.iter().zip(&reference).enumerate() {
            assert!(
                (*got - *want).abs() < 1e-12,
                "{name}: bar {index} = {got}, expected {want}"
            );
        }
    }

    // `time` must evaluate rather than fail with "Unknown variable: DATE". Its
    // encoding is TDX's `yyyymmdd`, so the only structural property worth
    // pinning is that it is positive and strictly increasing one day at a time.
    let source = "//@version=5\nindicator(\"T\")\nplot(time)\n";
    let pine = parse_pine(source).expect("parse time");
    let ast = map_pine_to_alphata(&pine).expect("map time");
    let values = engine.eval_ast(&ast, &mut ctx).expect("time must evaluate");
    assert_eq!(values.len(), LEN, "time: unexpected length");
    assert!(
        values.iter().all(|value| value.is_finite() && *value > 0.0),
        "time: expected finite positive dates for every bar, got {values:?}"
    );
    let slice = values.as_slice().expect("owned Array1 is contiguous");
    assert!(
        slice.windows(2).all(|pair| pair[1] > pair[0]),
        "time: expected strictly increasing dates, got {values:?}"
    );
}
