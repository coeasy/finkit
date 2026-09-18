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

    let mut ctx = FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(vol),
        None,
    );
    // Pre-register derived price sources used by some scripts (e.g. hlc3).
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
