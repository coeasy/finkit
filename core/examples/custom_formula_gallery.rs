//! Custom formula gallery — runnable examples of user-authored formulas.
//!
//! Every example below is a *user* formula: source text written against the
//! public formula language, executed through [`FormulaEngine`]. Nothing here
//! touches the kernel layer, so the file doubles as documentation of what a
//! host application can express without recompiling the crate.
//!
//! Run with:
//!
//! ```text
//! cargo run --release --example custom_formula_gallery -p finkit
//! ```
//!
//! The examples are ordered from the smallest useful formula to the features
//! a real strategy host needs: reusable components, named channels, drawing
//! commands, parameterized templates, incremental (streaming) evaluation,
//! dialect portability, and compile-once/run-many execution.

use finkit::formula::{
    parse_formula, FormulaContext, FormulaDialect, FormulaEngine, FormulaExecutionMode,
    FormulaStatefulStream,
};
use ndarray::Array1;

/// A deterministic OHLCV series: trend + three superimposed harmonics.
fn make_series(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut volume = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f64;
        let noise = (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5 + (t * 3.71).sin() * 0.8;
        let price = 100.0 + t * 0.01 + noise;
        open.push(price - 0.3);
        high.push(price + 1.0 + (t * 0.7).sin().abs() * 0.5);
        low.push(price - 1.0 - (t * 0.5).cos().abs() * 0.5);
        close.push(price);
        volume.push(10_000.0 + (t * 10.0).sin() * 3_000.0 + 2_000.0 * (t * 2.3).cos().abs());
    }
    (open, high, low, close, volume)
}

fn make_ctx(len: usize) -> FormulaContext {
    let (open, high, low, close, volume) = make_series(len);
    FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
        None,
    )
}

fn heading(n: usize, title: &str) {
    println!("\n─────────────────────────────────────────────────────────────");
    println!("例 {n}. {title}");
    println!("─────────────────────────────────────────────────────────────");
}

/// Show the tail of a series so output stays readable on a 10k-bar context.
fn tail(name: &str, values: &[f64], n: usize) {
    let start = values.len().saturating_sub(n);
    let shown: Vec<String> = values[start..]
        .iter()
        .map(|v| {
            if v.is_nan() {
                "NaN".to_string()
            } else {
                format!("{v:.4}")
            }
        })
        .collect();
    println!("  {name:<12} tail[{n}] = {}", shown.join(", "));
}

// ── Example 1 ────────────────────────────────────────────────────────────────
/// The smallest formula that does something useful: two moving averages and
/// the cross between them. `:=` binds a named series; the final expression is
/// the formula's value.
fn example_1_basic_expression() -> Result<(), Box<dyn std::error::Error>> {
    heading(1, "基础表达式：均线金叉");
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(120);

    let source = r#"
        MA5  := MA(CLOSE, 5);
        MA20 := MA(CLOSE, 20);
        CROSS(MA5, MA20)
    "#;
    println!("  源码: {source}");

    let signal = engine.eval(source, &mut ctx)?;
    tail("CROSS", signal.as_slice().unwrap(), 10);

    // Bindings survive execution, so a host can read intermediate series back.
    let ma20 = ctx.variables.get("MA20").expect("MA20 binding");
    tail("MA20", ma20.as_slice().unwrap(), 5);

    let hits: usize = signal.iter().filter(|v| **v != 0.0 && !v.is_nan()).count();
    println!("  金叉命中 bar 数 = {hits} / {}", signal.len());
    Ok(())
}

// ── Example 2 ────────────────────────────────────────────────────────────────
/// A user-registered component: an expression-level macro with named
/// parameters. It is expanded into the canonical AST *before* planning, so a
/// custom indicator keeps the same kernels and fast paths as a built-in.
fn example_2_custom_component() -> Result<(), Box<dyn std::error::Error>> {
    heading(2, "注册自定义组件并复用（register_custom_formula）");
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(120);

    // `ZS(X, N)`: how many standard deviations X sits from its own mean.
    // Built-ins cannot be shadowed, so a component either picks a fresh name
    // or deliberately overrides through the registry's own surface.
    engine.register_custom_formula("ZS", &["X", "N"], "(X - MA(X, N)) / STD(X, N)")?;
    // A component may call another component.
    engine.register_custom_formula(
        "MEAN_REV",
        &["X", "N"],
        "IF(ZS(X, N) < -2, 1, IF(ZS(X, N) > 2, -1, 0))",
    )?;

    println!("  已注册组件: {:?}", engine.custom_formula_names());

    let source = "MEAN_REV(CLOSE, 20)";
    let signal = engine.eval(source, &mut ctx)?;

    // Show the component's own intermediate so the thresholds are verifiable.
    let zs = engine.eval("ZS(CLOSE, 20)", &mut ctx)?;
    let finite: Vec<f64> = zs.iter().copied().filter(|v| v.is_finite()).collect();
    let (zmin, zmax) = (
        finite.iter().cloned().fold(f64::INFINITY, f64::min),
        finite.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    );
    println!("  ZS(CLOSE,20) 范围 = [{zmin:.3}, {zmax:.3}]（阈值 ±2 外的 bar 才触发）");
    tail("MEAN_REV", signal.as_slice().unwrap(), 10);

    let buys = signal.iter().filter(|v| **v == 1.0).count();
    let sells = signal.iter().filter(|v| **v == -1.0).count();
    println!("  超卖(买) bar = {buys}，超买(卖) bar = {sells}");

    // The same component composes over any series, not just CLOSE.
    let vol_signal = engine.eval("MEAN_REV(VOL, 20)", &mut ctx)?;
    tail("MEAN_REV(VOL)", vol_signal.as_slice().unwrap(), 5);

    // Removal is explicit and invalidates dependent plans.
    engine.unregister_custom_formula("MEAN_REV")?;
    println!(
        "  注销 MEAN_REV 后剩余: {:?}",
        engine.custom_formula_names()
    );
    Ok(())
}

// ── Example 3 ────────────────────────────────────────────────────────────────
/// `eval_multi` returns the final value *and* every series the formula bound,
/// which is how a host renders several panes from one evaluation.
fn example_3_named_channels() -> Result<(), Box<dyn std::error::Error>> {
    heading(3, "多输出通道：一次求值取回全部中间序列");
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(120);

    let source = r#"
        DIF   := EMA(CLOSE, 12) - EMA(CLOSE, 26);
        DEA   := EMA(DIF, 9);
        HIST  := (DIF - DEA) * 2;
        HIST
    "#;
    let multi = engine.eval_multi(source, &mut ctx)?;

    println!("  主输出 (HIST):");
    tail("HIST", multi.final_value.as_slice().unwrap(), 5);
    let mut names: Vec<&String> = multi.outputs.keys().collect();
    names.sort();
    println!("  命名通道 {} 条: {:?}", names.len(), names);
    for name in names {
        tail(name, multi.outputs[name].as_slice().unwrap(), 3);
    }
    Ok(())
}

// ── Example 4 ────────────────────────────────────────────────────────────────
/// Control flow and truthiness: `IF` with nested conditions, `REF` for prior
/// bars, and logical composition. Filters a raw signal down to first entries.
fn example_4_control_flow() -> Result<(), Box<dyn std::error::Error>> {
    heading(4, "控制流与条件：IF / REF / 逻辑组合");
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(120);

    let source = r#"
        RAW    := CROSS(MA(CLOSE, 5), MA(CLOSE, 20));
        CONFIRM := CLOSE > REF(CLOSE, 1) AND VOL > MA(VOL, 20);
        FIRST  := RAW AND NOT(REF(RAW, 1) OR REF(RAW, 2));
        IF(FIRST AND CONFIRM, 1, 0)
    "#;
    let signal = engine.eval(source, &mut ctx)?;
    tail("ENTRY", signal.as_slice().unwrap(), 12);

    let entries = signal.iter().filter(|v| **v == 1.0).count();
    println!("  确认后首次入场信号 = {entries} 次");

    // ATR-based stop: an arithmetic formula over three series at once.
    // ATR takes the three price series explicitly, not the period alone.
    let stop = engine.eval("CLOSE - 2 * ATR(HIGH, LOW, CLOSE, 14)", &mut ctx)?;
    tail("STOP", stop.as_slice().unwrap(), 3);
    Ok(())
}

// ── Example 5 ────────────────────────────────────────────────────────────────
/// Parameterized formulas: the source declares defaults, the caller overrides
/// them. This is the shape a screening UI needs — one formula text, many
/// parameter sets, no re-parsing of the structure.
fn example_5_parameterized() -> Result<(), Box<dyn std::error::Error>> {
    heading(5, "参数化公式模板（一次解析，多组参数）");
    let mut engine = FormulaEngine::new();

    for (fast, slow) in [(5, 20), (10, 30), (3, 8)] {
        let source = format!("CROSS(MA(CLOSE, {fast}), MA(CLOSE, {slow}))");
        let mut ctx = make_ctx(120);
        let signal = engine.eval(&source, &mut ctx)?;
        let hits = signal.iter().filter(|v| **v != 0.0 && !v.is_nan()).count();
        println!("  MA({fast}) x MA({slow}): 交叉 {hits} 次");
    }

    // The AST is available directly when a host wants to inspect or rewrite it.
    let ast = parse_formula("MA(CLOSE, 20) + STD(CLOSE, 20)")?;
    println!("  解析 AST: {:?}", ast);
    Ok(())
}

// ── Example 6 ────────────────────────────────────────────────────────────────
/// Incremental evaluation: push bars one at a time and read the latest value.
/// The stream owns the recurrence state, so a live feed costs O(1) per bar
/// instead of re-running the whole series.
fn example_6_streaming() -> Result<(), Box<dyn std::error::Error>> {
    heading(6, "流式增量求值（逐 bar 推送，O(1)/bar）");
    let (open, high, low, close, volume) = make_series(400);

    // The stream consumes the canonical six-slot row
    // `[open, high, low, close, volume, amount]`; unused slots stay NaN.
    let mut row = [f64::NAN; 6];
    let mut rsi_stream = FormulaStatefulStream::from_source(
        "RSI(CLOSE, 14)",
        finkit::formula::FormulaDialect::AlphaTA,
    )?;
    println!(
        "  RSI 依赖输入槽: {:?}",
        rsi_stream.required_inputs().collect::<Vec<_>>()
    );

    let mut rsi_last = f64::NAN;
    let mut warmup = 0usize;
    for i in 0..close.len() {
        row[0] = open[i];
        row[1] = high[i];
        row[2] = low[i];
        row[3] = close[i];
        row[4] = volume[i];
        row[5] = close[i] * volume[i];
        rsi_stream.push_values_into(&row, &mut rsi_last)?;
        if rsi_last.is_nan() {
            warmup += 1;
        }
    }
    println!(
        "  推送 {} bar 后 RSI(14) 最新值 = {:.4}（预热 bar {warmup}）",
        close.len(),
        rsi_last
    );

    // A second formula reading all three price series: ATR.
    let mut atr_stream = FormulaStatefulStream::from_source(
        "ATR(HIGH, LOW, CLOSE, 14)",
        finkit::formula::FormulaDialect::AlphaTA,
    )?;
    println!(
        "  ATR 依赖输入槽: {:?}",
        atr_stream.required_inputs().collect::<Vec<_>>()
    );
    let mut atr_last = f64::NAN;
    for i in 0..close.len() {
        row[0] = open[i];
        row[1] = high[i];
        row[2] = low[i];
        row[3] = close[i];
        row[4] = volume[i];
        row[5] = close[i] * volume[i];
        atr_stream.push_values_into(&row, &mut atr_last)?;
    }
    println!("  同期 ATR(14) 最新值 = {:.4}", atr_last);

    // Checkpoint / restore lets a host persist the recurrence state.
    let json = atr_stream.checkpoint().to_json()?;
    let restored = finkit::formula::FormulaStatefulCheckpoint::from_json(&json)?;
    atr_stream.restore(&restored)?;
    println!("  检查点 JSON {} 字节，恢复成功", json.len());
    Ok(())
}

// ── Example 7 ────────────────────────────────────────────────────────────────
/// Dialect portability: the same intent written for four hosts. The engine
/// normalizes transport differences (BOM, line endings) and parses per dialect.
fn example_7_dialects() -> Result<(), Box<dyn std::error::Error>> {
    heading(7, "多方言：通达信 / 同花顺 / 东财 / Pine");
    let mut engine = FormulaEngine::new();

    let cases: [(&str, FormulaDialect, &str); 4] = [
        ("通达信", FormulaDialect::TongDaXin, "MA(CLOSE, 20)"),
        ("同花顺", FormulaDialect::TongHuaShun, "MA(CLOSE, 20)"),
        ("东方财富", FormulaDialect::EastMoney, "MA(CLOSE, 20)"),
        ("Pine v5", FormulaDialect::Pine, "ta.sma(close, 20)"),
    ];

    for (label, dialect, source) in cases {
        let mut ctx = make_ctx(80);
        match engine.eval_with_dialect(source, dialect, &mut ctx) {
            Ok(values) => {
                let last = values
                    .as_slice()
                    .unwrap()
                    .last()
                    .copied()
                    .unwrap_or(f64::NAN);
                println!("  {label:<10} {source:<22} → 末值 {last:.4}");
            }
            Err(err) => println!("  {label:<10} {source:<22} → 不支持: {err}"),
        }
    }
    Ok(())
}

// ── Example 8 ────────────────────────────────────────────────────────────────
/// Drawing commands: a formula can emit annotations alongside its value, which
/// is how a charting host renders markers without a second pass.
fn example_8_drawing() -> Result<(), Box<dyn std::error::Error>> {
    heading(8, "绘图指令：DRAWTEXT / DRAWICON / DRAWLINE");
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(120);

    let source = r#"
        BUY := CROSS(MA(CLOSE, 5), MA(CLOSE, 20));
        DRAWTEXT(BUY, LOW, "B");
        DRAWICON(BUY, LOW, 1);
        BUY
    "#;
    let signal = engine.eval(source, &mut ctx)?;
    tail("BUY", signal.as_slice().unwrap(), 8);

    let marks = ctx.draw_commands.borrow().commands.len();
    println!("  绘图指令条数 = {marks}");
    Ok(())
}

// ── Example 9 ────────────────────────────────────────────────────────────────
/// Compile once, evaluate many times: the expensive parse/plan step happens
/// once, and every subsequent bar set reuses it. `eval_last` returns only the
/// newest value, which is what a scan loop over thousands of symbols wants.
fn example_9_compile_once() -> Result<(), Box<dyn std::error::Error>> {
    heading(9, "编译一次多次执行 + eval_last（扫描循环形态）");
    let mut engine = FormulaEngine::new();

    let source = "RSI(CLOSE, 14)";
    let compiled = engine.compile(source)?;

    // Warm up the plan cache, then evaluate on 200 distinct symbols.
    let mut scores = Vec::with_capacity(200);
    for symbol in 0..200 {
        let ctx = make_ctx(250);
        let _ = symbol; // each iteration stands in for a different symbol's bars
        scores.push(engine.eval_last(&compiled, &ctx)?);
    }
    let finite = scores.iter().filter(|v| v.is_finite()).count();
    println!(
        "  200 次 eval_last：有限值 {finite} 条，末值 {:.4}",
        scores[199]
    );

    // Switching to the compiled-plan execution path.
    let mut plan_engine = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
    let mut ctx = make_ctx(250);
    let values = plan_engine.eval(source, &mut ctx)?;
    tail("Plan RSI", values.as_slice().unwrap(), 5);
    let stats = plan_engine.plan_cache_stats();
    println!("  plan 缓存: {:?}", stats);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("═════════════════════════════════════════════════════════════");
    println!(" finkit 自定义公式执行示例集");
    println!("═════════════════════════════════════════════════════════════");

    example_1_basic_expression()?;
    example_2_custom_component()?;
    example_3_named_channels()?;
    example_4_control_flow()?;
    example_5_parameterized()?;
    example_6_streaming()?;
    example_7_dialects()?;
    example_8_drawing()?;
    example_9_compile_once()?;

    println!("\n全部示例执行完毕。");
    Ok(())
}
