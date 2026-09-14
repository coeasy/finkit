//! Static analysis for formula programs.
//!
//! The runtime intentionally remains permissive so existing terminal formulas
//! keep working.  This module provides the complementary, explicit contract:
//! callers can inspect dependencies, lookback, future-data usage and streaming
//! suitability before evaluating a formula.

use crate::formula::ast::AstNode;
use crate::formula::functions::get_builtin_functions;
use crate::formula::optimizer::FormulaOptimizer;
use std::collections::BTreeSet;

/// Severity of a static formula diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FormulaDiagnosticLevel {
    Info,
    Warning,
    Error,
}

/// A deterministic diagnostic emitted by [`analyze_formula`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FormulaDiagnostic {
    pub code: String,
    pub level: FormulaDiagnosticLevel,
    pub message: String,
}

/// Static facts about a formula before it is executed.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FormulaAnalysis {
    pub input_variables: Vec<String>,
    pub assigned_variables: Vec<String>,
    pub called_functions: Vec<String>,
    pub unknown_functions: Vec<String>,
    pub required_lookback: Option<usize>,
    pub estimated_nodes: usize,
    pub estimated_cost: usize,
    pub has_future_data: bool,
    pub has_stateful_functions: bool,
    pub has_observable_effects: bool,
    pub has_control_flow: bool,
    pub supports_streaming: bool,
    pub diagnostics: Vec<FormulaDiagnostic>,
}

/// Stable metadata describing the shape and validity contract of a formula
/// result.  The evaluator continues to return `f64` arrays for compatibility;
/// this sidecar makes warm-up rows, NaN semantics and named outputs explicit
/// for charting, bindings and downstream formula composition.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FormulaSeriesMetadata {
    /// Version of the result metadata contract.
    pub schema_version: String,
    /// Number of rows in the result.
    pub length: usize,
    /// Canonical scalar storage type.
    pub dtype: String,
    /// Named output variables plus the final expression result.
    pub output_names: Vec<String>,
    /// Null/invalid values are represented by IEEE NaN in the current ABI.
    pub null_policy: String,
    /// Conservative number of leading rows that may be unavailable.
    pub required_lookback: Option<usize>,
    pub warmup: usize,
    /// First row that may contain a valid value, if the result is non-empty.
    pub valid_start: Option<usize>,
    pub has_future_data: bool,
    pub supports_streaming: bool,
    pub has_observable_effects: bool,
}

impl FormulaAnalysis {
    pub fn is_safe_for_incremental_evaluation(&self) -> bool {
        self.supports_streaming && !self.has_future_data && !self.has_observable_effects
    }

    /// Build the result sidecar without executing the formula.
    pub fn result_metadata(&self, length: usize) -> FormulaSeriesMetadata {
        let warmup = self.required_lookback.unwrap_or(0);
        let mut output_names = self.assigned_variables.clone();
        output_names.push("__result__".to_string());
        FormulaSeriesMetadata {
            schema_version: "finkit.formula-series.v1".to_string(),
            length,
            dtype: "float64".to_string(),
            output_names,
            null_policy: "nan".to_string(),
            required_lookback: self.required_lookback,
            warmup,
            valid_start: (length > 0).then(|| warmup.min(length)),
            has_future_data: self.has_future_data,
            supports_streaming: self.supports_streaming,
            has_observable_effects: self.has_observable_effects,
        }
    }
}

/// Analyze an AST without evaluating it.
pub fn analyze_formula(ast: &AstNode) -> FormulaAnalysis {
    let builtins = get_builtin_functions();
    let mut input_variables = BTreeSet::new();
    let mut assigned_variables = BTreeSet::new();
    let mut called_functions = BTreeSet::new();
    let mut unknown_functions = BTreeSet::new();
    let mut diagnostics = Vec::new();
    let mut estimated_nodes = 0usize;
    let mut estimated_cost = 0usize;
    let mut has_future_data = false;
    let mut has_stateful_functions = false;
    let mut has_observable_effects = false;
    let mut has_control_flow = false;

    fn visit(
        node: &AstNode,
        builtins: &std::collections::HashMap<String, crate::formula::functions::FormulaFn>,
        inputs: &mut BTreeSet<String>,
        assigned: &mut BTreeSet<String>,
        calls: &mut BTreeSet<String>,
        unknown: &mut BTreeSet<String>,
        diagnostics: &mut Vec<FormulaDiagnostic>,
        nodes: &mut usize,
        cost: &mut usize,
        future: &mut bool,
        stateful: &mut bool,
        effects: &mut bool,
        control_flow: &mut bool,
    ) {
        *nodes += 1;
        match node {
            AstNode::Variable(name) => {
                if !assigned.contains(name) {
                    inputs.insert(name.clone());
                }
            }
            AstNode::FunctionCall { name, args } => {
                let upper = name.to_ascii_uppercase();
                calls.insert(upper.clone());
                *cost += match upper.as_str() {
                    "MA" | "SMA" | "WMA" | "HHV" | "LLV" | "SUM" | "COUNT" => 2,
                    "EMA" | "DMA" | "DEMA" | "TEMA" | "KAMA" | "MAMA" | "MACD" => 3,
                    "SECURITY" | "BACKSET" | "FILTER" | "BARSLAST" => 4,
                    _ => 1,
                };
                if !builtins.contains_key(&upper) {
                    unknown.insert(upper.clone());
                    diagnostics.push(FormulaDiagnostic {
                        code: "UNKNOWN_FUNCTION".to_string(),
                        level: FormulaDiagnosticLevel::Warning,
                        message: format!("function `{name}` is not in the built-in registry"),
                    });
                }
                if matches!(
                    upper.as_str(),
                    "REFX" | "BACKSET" | "FUTURE" | "ZIG" | "ZIGZAG"
                ) {
                    *future = true;
                    diagnostics.push(FormulaDiagnostic {
                        code: "FUTURE_DATA".to_string(),
                        level: FormulaDiagnosticLevel::Warning,
                        message: format!("function `{name}` may read or rewrite future bars"),
                    });
                }
                if matches!(
                    upper.as_str(),
                    "EMA" | "DMA" | "DEMA" | "TEMA" | "KAMA" | "MAMA" | "SAR"
                ) {
                    *stateful = true;
                }
                if matches!(upper.as_str(), "SECURITY" | "REQUEST.SECURITY") {
                    diagnostics.push(FormulaDiagnostic {
                        code: "HOST_TIMEFRAME_REQUIRED".to_string(),
                        level: FormulaDiagnosticLevel::Info,
                        message: "cross-timeframe evaluation requires host alignment metadata"
                            .to_string(),
                    });
                }
                for arg in args {
                    visit(
                        arg,
                        builtins,
                        inputs,
                        assigned,
                        calls,
                        unknown,
                        diagnostics,
                        nodes,
                        cost,
                        future,
                        stateful,
                        effects,
                        control_flow,
                    );
                }
            }
            AstNode::Assignment { name, expr }
            | AstNode::Output { name, expr, .. }
            | AstNode::CompoundAssignment { name, expr, .. } => {
                assigned.insert(name.clone());
                // Assignments are observable through FormulaContext::variables
                // and therefore cannot be silently deduplicated in a batch.
                *effects = true;
                visit(
                    expr,
                    builtins,
                    inputs,
                    assigned,
                    calls,
                    unknown,
                    diagnostics,
                    nodes,
                    cost,
                    future,
                    stateful,
                    effects,
                    control_flow,
                );
            }
            AstNode::Statements(stmts) => {
                for stmt in stmts {
                    visit(
                        stmt,
                        builtins,
                        inputs,
                        assigned,
                        calls,
                        unknown,
                        diagnostics,
                        nodes,
                        cost,
                        future,
                        stateful,
                        effects,
                        control_flow,
                    );
                }
            }
            AstNode::BinaryOp { left, right, .. } => {
                visit(
                    left,
                    builtins,
                    inputs,
                    assigned,
                    calls,
                    unknown,
                    diagnostics,
                    nodes,
                    cost,
                    future,
                    stateful,
                    effects,
                    control_flow,
                );
                visit(
                    right,
                    builtins,
                    inputs,
                    assigned,
                    calls,
                    unknown,
                    diagnostics,
                    nodes,
                    cost,
                    future,
                    stateful,
                    effects,
                    control_flow,
                );
            }
            AstNode::UnaryOp { expr, .. } => visit(
                expr,
                builtins,
                inputs,
                assigned,
                calls,
                unknown,
                diagnostics,
                nodes,
                cost,
                future,
                stateful,
                effects,
                control_flow,
            ),
            AstNode::IndexAccess { array, index } => {
                visit(
                    array,
                    builtins,
                    inputs,
                    assigned,
                    calls,
                    unknown,
                    diagnostics,
                    nodes,
                    cost,
                    future,
                    stateful,
                    effects,
                    control_flow,
                );
                visit(
                    index,
                    builtins,
                    inputs,
                    assigned,
                    calls,
                    unknown,
                    diagnostics,
                    nodes,
                    cost,
                    future,
                    stateful,
                    effects,
                    control_flow,
                );
            }
            AstNode::IfThenElse {
                cond,
                then_branch,
                else_branch,
            } => {
                *control_flow = true;
                for branch in [cond, then_branch, else_branch] {
                    visit(
                        branch,
                        builtins,
                        inputs,
                        assigned,
                        calls,
                        unknown,
                        diagnostics,
                        nodes,
                        cost,
                        future,
                        stateful,
                        effects,
                        control_flow,
                    );
                }
            }
            AstNode::ForLoop {
                var,
                start,
                end,
                body,
            } => {
                *control_flow = true;
                assigned.insert(var.clone());
                visit(
                    start,
                    builtins,
                    inputs,
                    assigned,
                    calls,
                    unknown,
                    diagnostics,
                    nodes,
                    cost,
                    future,
                    stateful,
                    effects,
                    control_flow,
                );
                visit(
                    end,
                    builtins,
                    inputs,
                    assigned,
                    calls,
                    unknown,
                    diagnostics,
                    nodes,
                    cost,
                    future,
                    stateful,
                    effects,
                    control_flow,
                );
                for stmt in body {
                    visit(
                        stmt,
                        builtins,
                        inputs,
                        assigned,
                        calls,
                        unknown,
                        diagnostics,
                        nodes,
                        cost,
                        future,
                        stateful,
                        effects,
                        control_flow,
                    );
                }
                diagnostics.push(FormulaDiagnostic {
                    code: "LOOP_REQUIRES_FULL_EVAL".to_string(),
                    level: FormulaDiagnosticLevel::Warning,
                    message: "loops require conservative full-prefix evaluation".to_string(),
                });
            }
            AstNode::WhileLoop { cond, body } => {
                *control_flow = true;
                visit(
                    cond,
                    builtins,
                    inputs,
                    assigned,
                    calls,
                    unknown,
                    diagnostics,
                    nodes,
                    cost,
                    future,
                    stateful,
                    effects,
                    control_flow,
                );
                for stmt in body {
                    visit(
                        stmt,
                        builtins,
                        inputs,
                        assigned,
                        calls,
                        unknown,
                        diagnostics,
                        nodes,
                        cost,
                        future,
                        stateful,
                        effects,
                        control_flow,
                    );
                }
                diagnostics.push(FormulaDiagnostic {
                    code: "LOOP_REQUIRES_FULL_EVAL".to_string(),
                    level: FormulaDiagnosticLevel::Warning,
                    message: "loops require conservative full-prefix evaluation".to_string(),
                });
            }
            AstNode::DrawText { cond, price, .. } => {
                *effects = true;
                visit(
                    cond,
                    builtins,
                    inputs,
                    assigned,
                    calls,
                    unknown,
                    diagnostics,
                    nodes,
                    cost,
                    future,
                    stateful,
                    effects,
                    control_flow,
                );
                visit(
                    price,
                    builtins,
                    inputs,
                    assigned,
                    calls,
                    unknown,
                    diagnostics,
                    nodes,
                    cost,
                    future,
                    stateful,
                    effects,
                    control_flow,
                );
            }
            AstNode::DrawIcon {
                cond, price, icon, ..
            } => {
                *effects = true;
                for expr in [cond, price, icon] {
                    visit(
                        expr,
                        builtins,
                        inputs,
                        assigned,
                        calls,
                        unknown,
                        diagnostics,
                        nodes,
                        cost,
                        future,
                        stateful,
                        effects,
                        control_flow,
                    );
                }
            }
            AstNode::StickLine {
                cond,
                price1,
                price2,
                width,
                ..
            } => {
                *effects = true;
                for expr in [cond, price1, price2, width] {
                    visit(
                        expr,
                        builtins,
                        inputs,
                        assigned,
                        calls,
                        unknown,
                        diagnostics,
                        nodes,
                        cost,
                        future,
                        stateful,
                        effects,
                        control_flow,
                    );
                }
            }
            AstNode::DrawGeneric { args, .. } => {
                *effects = true;
                for expr in args {
                    visit(
                        expr,
                        builtins,
                        inputs,
                        assigned,
                        calls,
                        unknown,
                        diagnostics,
                        nodes,
                        cost,
                        future,
                        stateful,
                        effects,
                        control_flow,
                    );
                }
            }
            AstNode::Number(_) | AstNode::StringLit(_) | AstNode::ParamDecl { .. } => {}
        }
    }

    visit(
        ast,
        &builtins,
        &mut input_variables,
        &mut assigned_variables,
        &mut called_functions,
        &mut unknown_functions,
        &mut diagnostics,
        &mut estimated_nodes,
        &mut estimated_cost,
        &mut has_future_data,
        &mut has_stateful_functions,
        &mut has_observable_effects,
        &mut has_control_flow,
    );

    let required_lookback = FormulaOptimizer::required_lookback(ast);
    let registered_streaming = called_functions.iter().all(|name| {
        matches!(
            name.as_str(),
            "MA" | "SMA" | "WMA" | "EMA" | "RSI" | "MACD" | "ATR" | "BOLL" | "BOLLINGER"
        )
    });
    let supports_streaming = !has_future_data
        && !has_control_flow
        && unknown_functions.is_empty()
        && (required_lookback.is_some() || registered_streaming);
    if has_stateful_functions && !supports_streaming {
        diagnostics.push(FormulaDiagnostic {
            code: "STREAMING_CONSERVATIVE".to_string(),
            level: FormulaDiagnosticLevel::Info,
            message: "stateful nodes require a registered streaming implementation".to_string(),
        });
    }

    let mut input_variables: Vec<String> = input_variables.into_iter().collect();
    input_variables.retain(|name| !assigned_variables.contains(name));

    FormulaAnalysis {
        input_variables,
        assigned_variables: assigned_variables.into_iter().collect(),
        called_functions: called_functions.into_iter().collect(),
        unknown_functions: unknown_functions.into_iter().collect(),
        required_lookback,
        estimated_nodes,
        estimated_cost,
        has_future_data,
        has_stateful_functions,
        has_observable_effects,
        has_control_flow,
        supports_streaming,
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::parser::parse_formula;

    #[test]
    fn reports_dependencies_and_lookback() {
        let ast = parse_formula("X:=MA(CLOSE,5); X + OPEN").unwrap();
        let report = analyze_formula(&ast);
        assert_eq!(report.input_variables, vec!["CLOSE", "OPEN"]);
        assert_eq!(report.assigned_variables, vec!["X"]);
        assert_eq!(report.required_lookback, Some(4));
        assert!(report.called_functions.contains(&"MA".to_string()));
    }

    #[test]
    fn marks_future_and_unknown_functions() {
        let ast = parse_formula("REFX(CLOSE,1) + CUSTOM(CLOSE)").unwrap();
        let report = analyze_formula(&ast);
        assert!(report.has_future_data);
        assert_eq!(report.unknown_functions, vec!["CUSTOM", "REFX"]);
        assert!(!report.supports_streaming);
    }

    #[test]
    fn exposes_result_metadata_contract() {
        let ast = parse_formula("X:=MA(CLOSE,5); X").unwrap();
        let report = analyze_formula(&ast);
        let metadata = report.result_metadata(20);
        assert_eq!(metadata.schema_version, "finkit.formula-series.v1");
        assert_eq!(metadata.dtype, "float64");
        assert_eq!(metadata.output_names, vec!["X", "__result__"]);
        assert_eq!(metadata.required_lookback, Some(4));
        assert_eq!(metadata.valid_start, Some(4));
        assert_eq!(metadata.null_policy, "nan");
    }
}
