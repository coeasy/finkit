//! Formula expression engine for technical analysis.
//!
//! Parses and evaluates indicator expressions like `SMA(CLOSE, 20)` or
//! `CROSS(MA(C,5), MA(C,20))`. Supports bytecode compilation, JIT
//! optimization, SIMD acceleration, and zero-copy evaluation.
//!
//! # Features
//!
//! - Expression parser with operator precedence
//! - Bytecode compiler + VM executor
//! - JIT-optimized evaluation
//! - SIMD-accelerated operations
//! - Template library with 100+ pre-built formulas
//! - Memory pool for zero-copy evaluation

pub mod ast;
pub mod bytecode;
pub mod compat;
pub mod compiler;
pub mod compute_ir;
pub mod debugger;
pub mod drawing;
pub mod engine;
pub mod executor;
pub mod functions;
pub mod hot_plan;
pub mod jit;
pub mod memory_pool;
pub mod ops;
pub mod opt_level;
pub mod optimizer;
pub mod params;
pub mod parser;
pub mod pine;
mod range_zero_copy;
pub mod sandbox;
// AArch64 guarantees NEON in the Finkit dispatch model, so the scalar fallback
// statements that follow unconditional NEON returns are intentionally
// unreachable on that architecture. Keep unreachable-code diagnostics enabled
// everywhere else so genuine control-flow regressions are still reported.
#[cfg_attr(target_arch = "aarch64", allow(unreachable_code))]
pub mod simd;
pub mod templates;
pub mod types;
pub mod unified_dispatch;

pub use ast::*;
pub use bytecode::{compile_to_bytecode, Bytecode, BytecodeVM, ExecResult, OpCode};
pub use compat::{normalize_terminal_source, CompatibilityLevel, FormulaTerminal};
pub use compiler::{CompiledFormula, FormulaCache, FormulaCompiler};
pub use compute_ir::{lower_formula_ast, lower_formula_ast_with_registry, FormulaComputePlan};
pub use debugger::{DebugEvent, FormulaDebugger, FormulaErrorWithLocation};
pub use drawing::{DrawCommand, DrawResult};
pub use engine::{FormulaEngine, FormulaResult};
pub use executor::FormulaExecutor;
pub use functions::get_builtin_functions;
pub use hot_plan::{FormulaHotPlan, FormulaHotPlanError};
pub use jit::{JitCompiler, OptimizedBytecode};
pub use memory_pool::{BufferPool, ZeroCopyContext};
pub use ops::*;
pub use opt_level::OptLevel;
pub use optimizer::{DependencyAnalyzer, FormulaOptimizer};
pub use params::{
    apply_params, get_param_value, parse_params, validate_params, ParamDef, ParamValues,
};
pub use parser::parse_formula;
pub use pine::{map_pine_to_alphata, parse_pine, PineBuiltinTable, PineError, PineMapperError};
pub use sandbox::{ExecSandboxConfig, ExecSandboxState};
pub use simd::SimdOps;
pub use templates::{FormulaTemplate, FormulaTemplates, TemplateCategory};
pub use types::FormulaValue;
pub use types::*;
pub use unified_dispatch::{unified_formula_executor, FormulaKernelDispatcher};

/// Formula language dialect selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormulaDialect {
    /// Finkit / AlphaTA / TDX-style formula language (default).
    #[default]
    AlphaTA,
    /// TradingView Pine Script v5 subset.
    Pine,
}

impl FormulaDialect {
    /// Parse a dialect name from CLI / FFI / Python bindings.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "alpha_ta" | "alphata" | "finkit" | "tdx" | "tongdaxin" | "通达信" | "ths"
            | "tonghuashun" | "同花顺" | "eastmoney" | "em" | "dfcf" | "东方财富" | "default"
            | "" => Some(Self::AlphaTA),
            "pine" | "tradingview" | "tv" => Some(Self::Pine),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::AlphaTA => "alpha_ta",
            Self::Pine => "pine",
        }
    }
}

/// Parse a formula expression using the specified dialect.
///
/// `FormulaDialect::AlphaTA` delegates to [`parse_formula`].
/// `FormulaDialect::Pine` uses the Pine Script v5 subset parser and maps to AlphaTA AST.
pub fn parse_formula_with_dialect(
    source: &str,
    dialect: FormulaDialect,
) -> Result<AstNode, String> {
    match dialect {
        FormulaDialect::AlphaTA => parse_formula(source),
        FormulaDialect::Pine => {
            let pine = parse_pine(source).map_err(|e| format!("Pine parse error: {}", e))?;
            map_pine_to_alphata(&pine).map_err(|e| format!("Pine map error: {}", e.message))
        }
    }
}

/// Parse source using a named trading-terminal compatibility adapter.
pub fn parse_formula_for_terminal(
    source: &str,
    terminal: FormulaTerminal,
) -> Result<AstNode, String> {
    let normalized = normalize_terminal_source(source, terminal);
    parse_formula_with_dialect(&normalized, terminal.canonical_dialect())
}

#[cfg(test)]
mod dialect_tests {
    use super::*;

    #[test]
    fn alpha_ta_dialect_parses_tdx_formula() {
        let ast = parse_formula_with_dialect("CLOSE + OPEN", FormulaDialect::AlphaTA).unwrap();
        assert!(matches!(ast, AstNode::BinaryOp { .. }));
    }

    #[test]
    fn tdx_terminal_parses_assignment_and_cross() {
        let source = "MA5:=MA(CLOSE,5); CROSS(CLOSE,MA5);";
        let ast = parse_formula_for_terminal(source, FormulaTerminal::TongDaXin).unwrap();
        assert!(matches!(
            ast,
            AstNode::Statements(_) | AstNode::Assignment { .. }
        ));
    }

    #[test]
    fn pine_dialect_parses_rsi_script() {
        let src = r#"//@version=5
indicator("RSI")
length = input(14)
rsi = ta.rsi(close, length)
plot(rsi)
"#;
        let ast = parse_formula_with_dialect(src, FormulaDialect::Pine).unwrap();
        assert!(matches!(
            ast,
            AstNode::Statements(_) | AstNode::Assignment { .. }
        ));
    }

    #[test]
    fn dialect_from_str() {
        assert_eq!(FormulaDialect::from_str("pine"), Some(FormulaDialect::Pine));
        assert_eq!(
            FormulaDialect::from_str("alpha_ta"),
            Some(FormulaDialect::AlphaTA)
        );
        assert_eq!(FormulaDialect::from_str("unknown"), None);
    }
}
