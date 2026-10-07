//! Surface audit: what the formula engine exposes, and how much of the
//! public TA-Lib catalog that covers.
//!
//! Answers "is the core complete?" with numbers rather than impressions:
//! the built-in formula function count, the TA-Lib catalog coverage derived
//! from the *runtime* function map (not a hand-maintained list), and the
//! dialect/execution-mode matrix.

use finkit::formula::{get_builtin_functions, ta_lib_function_contracts};

fn main() {
    let builtins = get_builtin_functions();
    let mut names: Vec<&String> = builtins.keys().collect();
    names.sort();

    println!("═════════════════════════════════════════════════════════════");
    println!(" finkit 公式引擎能力审计");
    println!("═════════════════════════════════════════════════════════════");
    println!("\n内置公式函数总数: {}", names.len());

    // TA-Lib coverage, derived from the live function map.
    let contracts = ta_lib_function_contracts();
    let registered = contracts.iter().filter(|c| c.runtime_registered).count();
    println!(
        "TA-Lib 公开目录函数: {} 项，运行时已注册 {} 项（{:.1}%）",
        contracts.len(),
        registered,
        registered as f64 / contracts.len() as f64 * 100.0
    );

    let missing: Vec<&str> = contracts
        .iter()
        .filter(|c| !c.runtime_registered)
        .map(|c| c.name.as_str())
        .collect();
    if missing.is_empty() {
        println!("未注册项: 无（TA-Lib 公开目录 100% 覆盖）");
    } else {
        println!("未注册项 ({}): {}", missing.len(), missing.join(", "));
    }

    // Category histogram over the live map.
    let categories: std::collections::BTreeMap<&str, usize> =
        contracts.iter().fold(Default::default(), |mut acc, c| {
            *acc.entry(c.category.as_str()).or_default() += 1;
            acc
        });
    println!("\nTA-Lib 目录分类分布:");
    for (category, count) in categories {
        println!("  {:<16}{:>4}", category, count);
    }

    // Capability matrix: what a host can actually select.
    println!("\n执行与方言矩阵:");
    println!(
        "  {:<22}{}",
        "方言", "AlphaTA / TongDaXin / TongHuaShun / EastMoney / Pine"
    );
    println!(
        "  {:<22}{}",
        "执行模式", "Interpreter(默认) / Plan / Bytecode / JIT"
    );
    println!(
        "  {:<22}{}",
        "自定义组件", "register_custom_formula（表达式级宏，可组合）"
    );
    println!(
        "  {:<22}{}",
        "多输出", "eval_multi / 命名通道 / DRAWTEXT-DRAWICON-DRAWLINE"
    );
    println!(
        "  {:<22}{}",
        "增量流式", "FormulaStatefulStream（O(1)/bar，检查点可序列化）"
    );
    println!(
        "  {:<22}{}",
        "零拷贝", "eval_range_zero_copy_inputs / BufferPool"
    );

    // A sample of the surface, so the number is not the only evidence.
    println!("\n内置函数抽样（前 40 项，共 {}）:", names.len());
    for chunk in names.chunks(8).take(5) {
        println!(
            "  {}",
            chunk
                .iter()
                .map(|n| format!("{:<10}", n))
                .collect::<String>()
        );
    }
}
