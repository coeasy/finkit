//! Trading-terminal compatibility routing for the formula engine.
//!
//! Finkit keeps one canonical AST/runtime. Terminal names are resolved to the
//! closest canonical parser instead of maintaining divergent executors. The
//! v0.1.2 external-terminal adapters intentionally target documented common
//! subsets; terminal-specific extensions can be added without changing the
//! execution engine.

use super::stateful::FormulaStatefulStream;
use super::FormulaDialect;
use crate::formula::analysis::{analyze_formula, FormulaAnalysis};
use crate::formula::ast::AstNode;
use crate::formula::contracts::{ta_lib_function_contracts, TA_LIB_CATALOG_VERSION};
use std::collections::HashMap;

/// Stable schema identifier for terminal compatibility discovery.
pub const FORMULA_TERMINAL_SCHEMA_VERSION: &str = "finkit.formula-terminal.v1";

/// Formula source terminal understood by the compatibility layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FormulaTerminal {
    /// Native Finkit / AlphaTA-compatible formula syntax.
    #[cfg_attr(feature = "serde", serde(rename = "finkit"))]
    Finkit,
    /// 通达信 common formula subset.
    #[cfg_attr(feature = "serde", serde(rename = "tdx"))]
    TongDaXin,
    /// 同花顺 common formula subset.
    #[cfg_attr(feature = "serde", serde(rename = "ths"))]
    TongHuaShun,
    /// 东方财富 common formula subset.
    #[cfg_attr(feature = "serde", serde(rename = "eastmoney"))]
    EastMoney,
    /// TradingView Pine Script subset.
    #[cfg_attr(feature = "serde", serde(rename = "pine"))]
    TradingView,
    /// 大智慧 common formula subset.
    ///
    /// Added as the sixth terminal so 大智慧 formulas are a *declared* adapter
    /// instead of being silently parsed as 同花顺 source. It deliberately has no
    /// dedicated dialect: 大智慧 shares the 同花顺 common subset for the shared
    /// core (MA/EMA/HHV/LLV/CROSS/...), and its特有 families are routed through
    /// the same canonical runtime. Promoting it to a full dialect is a separate
    /// change that would also have to update the exhaustive dialect matches in
    /// `engine.rs`.
    #[cfg_attr(feature = "serde", serde(rename = "dzh"))]
    DaZhiHui,
}

/// All declared formula terminals in stable discovery order.
pub const FORMULA_TERMINALS: &[FormulaTerminal] = &[
    FormulaTerminal::Finkit,
    FormulaTerminal::TongDaXin,
    FormulaTerminal::TongHuaShun,
    FormulaTerminal::EastMoney,
    FormulaTerminal::TradingView,
    FormulaTerminal::DaZhiHui,
];

/// Declared compatibility strength for a terminal adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityLevel {
    /// Native Finkit parser/runtime contract.
    Native,
    /// A documented common syntax/function subset is supported.
    CommonSubset,
}

/// Result of checking one formula feature against a terminal contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum CompatibilityStatus {
    Exact,
    Near,
    Approximate,
    HostRequired,
    Unsupported,
}

impl CompatibilityStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Near => "near",
            Self::Approximate => "approximate",
            Self::HostRequired => "host_required",
            Self::Unsupported => "unsupported",
        }
    }
}

/// Explicit semantic choices used by a terminal adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SemanticProfile {
    pub id: String,
    pub null_policy: String,
    pub boolean_numeric_policy: String,
    pub sma_policy: String,
    pub lookahead_policy: String,
    pub requires_session_metadata: bool,
}

/// Function-level compatibility result.  It is intentionally small and
/// serializable so Python/Node/CLI consumers can show the same report.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FunctionCompatibility {
    pub name: String,
    pub status: CompatibilityStatus,
    pub message: String,
    pub cataloged: bool,
    pub runtime_registered: bool,
    pub category: Option<String>,
    pub outputs: Option<usize>,
}

/// Capability-level compatibility result for one complete formula feature.
///
/// Function-level status answers whether a called name can be routed. This
/// separate matrix answers whether the complete source can use a capability
/// such as drawing, control flow, streaming, or host-provided timeframes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CapabilityCompatibility {
    /// Stable capability identifier used by bindings and CI gates.
    pub name: String,
    /// Compatibility level for the selected terminal profile.
    pub status: CompatibilityStatus,
    /// Whether the canonical runtime supports the capability in this profile.
    pub supported: bool,
    /// Whether this particular source uses the capability.
    pub observed: bool,
    /// Actionable explanation when the capability is partial or host-bound.
    pub message: String,
}

/// Complete source compatibility report.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FormulaCompatibilityReport {
    pub terminal: FormulaTerminal,
    pub normalized_source: String,
    pub profile: SemanticProfile,
    pub analysis: FormulaAnalysis,
    pub functions: Vec<FunctionCompatibility>,
    /// Source-level capability matrix used by production admission checks.
    pub capabilities: Vec<CapabilityCompatibility>,
    /// Catalog revision used when classifying TA-Lib names.
    pub ta_lib_catalog_version: String,
    pub ta_lib_function_count: usize,
    pub ta_lib_runtime_registered_count: usize,
}

impl CompatibilityLevel {
    /// Stable lowercase identifier for schema/CLI consumers.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::CommonSubset => "common_subset",
        }
    }
}

impl FormulaTerminal {
    /// Parse a user-facing terminal name and common aliases.
    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "finkit" | "alpha_ta" | "alphata" | "default" => Some(Self::Finkit),
            "tdx" | "tongdaxin" | "通达信" => Some(Self::TongDaXin),
            "ths" | "tonghuashun" | "同花顺" => Some(Self::TongHuaShun),
            "eastmoney" | "em" | "dfcf" | "东方财富" => Some(Self::EastMoney),
            "pine" | "tradingview" | "tv" => Some(Self::TradingView),
            "dzh" | "dazhihui" | "大智慧" => Some(Self::DaZhiHui),
            _ => None,
        }
    }

    /// Return every declared terminal in stable discovery order.
    pub const fn all() -> &'static [Self] {
        FORMULA_TERMINALS
    }

    /// Canonical parser used by this terminal.
    pub const fn canonical_dialect(self) -> FormulaDialect {
        match self {
            Self::TradingView => FormulaDialect::Pine,
            Self::Finkit => FormulaDialect::AlphaTA,
            Self::TongDaXin => FormulaDialect::TongDaXin,
            Self::TongHuaShun => FormulaDialect::TongHuaShun,
            Self::EastMoney => FormulaDialect::EastMoney,
            // 大智慧 shares the 同花顺 common subset; see the variant docs.
            Self::DaZhiHui => FormulaDialect::TongHuaShun,
        }
    }

    /// Compatibility strength shipped in v0.1.2.
    ///
    /// Only Finkit's own language is a native contract. External terminal
    /// adapters deliberately advertise subset compatibility until their
    /// terminal-specific golden matrices are complete.
    pub const fn compatibility_level(self) -> CompatibilityLevel {
        match self {
            Self::Finkit => CompatibilityLevel::Native,
            Self::TongDaXin
            | Self::TongHuaShun
            | Self::EastMoney
            | Self::TradingView
            | Self::DaZhiHui => CompatibilityLevel::CommonSubset,
        }
    }

    /// Stable lowercase identifier for bindings and CLI output.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Finkit => "finkit",
            Self::TongDaXin => "tdx",
            Self::TongHuaShun => "ths",
            Self::EastMoney => "eastmoney",
            Self::TradingView => "pine",
            Self::DaZhiHui => "dzh",
        }
    }

    /// Semantic contract used when importing formulas from this terminal.
    pub fn semantic_profile(self) -> SemanticProfile {
        match self {
            Self::Finkit => SemanticProfile {
                id: "finkit-native-v1".to_string(),
                null_policy: "nan-propagating".to_string(),
                boolean_numeric_policy: "true-is-1".to_string(),
                sma_policy: "simple-ma".to_string(),
                lookahead_policy: "disallow-future".to_string(),
                requires_session_metadata: false,
            },
            Self::TongDaXin => SemanticProfile {
                id: "tdx-v1".to_string(),
                null_policy: "nan-propagating".to_string(),
                boolean_numeric_policy: "true-is-1".to_string(),
                sma_policy: "recursive-sma".to_string(),
                lookahead_policy: "explicit-only".to_string(),
                requires_session_metadata: true,
            },
            Self::TongHuaShun => SemanticProfile {
                id: "ths-v1".to_string(),
                null_policy: "nan-propagating".to_string(),
                boolean_numeric_policy: "true-is-1".to_string(),
                sma_policy: "recursive-sma".to_string(),
                lookahead_policy: "explicit-only".to_string(),
                requires_session_metadata: true,
            },
            Self::EastMoney => SemanticProfile {
                id: "eastmoney-v1".to_string(),
                null_policy: "nan-propagating".to_string(),
                boolean_numeric_policy: "true-is-1".to_string(),
                sma_policy: "recursive-sma".to_string(),
                lookahead_policy: "explicit-only".to_string(),
                requires_session_metadata: true,
            },
            Self::TradingView => SemanticProfile {
                id: "pine-v5-subset-v1".to_string(),
                null_policy: "na-propagating".to_string(),
                boolean_numeric_policy: "bool-context".to_string(),
                sma_policy: "ta.sma".to_string(),
                lookahead_policy: "request-security-controlled".to_string(),
                requires_session_metadata: true,
            },
            Self::DaZhiHui => SemanticProfile {
                id: "dzh-v1".to_string(),
                null_policy: "nan-propagating".to_string(),
                boolean_numeric_policy: "true-is-1".to_string(),
                sma_policy: "recursive-sma".to_string(),
                lookahead_policy: "explicit-only".to_string(),
                requires_session_metadata: true,
            },
        }
    }
}

/// Parse and inspect a source formula for a declared terminal.
pub fn inspect_formula_compatibility(
    source: &str,
    terminal: FormulaTerminal,
) -> Result<FormulaCompatibilityReport, String> {
    let normalized_source = normalize_terminal_source(source, terminal);
    let ast = super::parse_formula_with_dialect(&normalized_source, terminal.canonical_dialect())?;
    let analysis = analyze_formula(&ast);
    let catalog = ta_lib_function_contracts();
    let catalog_by_name: HashMap<&str, _> = catalog
        .iter()
        .map(|item| (item.name.as_str(), item))
        .collect();
    let ta_lib_runtime_registered_count = catalog
        .iter()
        .filter(|item| item.runtime_registered)
        .count();
    let functions = analysis
        .called_functions
        .iter()
        .map(|name| {
            let contract = catalog_by_name.get(name.as_str()).copied();
            let runtime_registered = !analysis.unknown_functions.contains(name);
            let status = if analysis.unknown_functions.contains(name) {
                if matches!(
                    name.as_str(),
                    "SECURITY"
                        | "REQUEST.SECURITY"
                        | "CAPITAL"
                        | "FINANCE"
                        | "DYNAINFO"
                        | "WINNER"
                        | "COST"
                ) {
                    CompatibilityStatus::HostRequired
                } else {
                    CompatibilityStatus::Unsupported
                }
            } else if analysis.has_future_data
                && matches!(name.as_str(), "REFX" | "BACKSET" | "FUTURE")
            {
                CompatibilityStatus::Approximate
            } else if terminal == FormulaTerminal::Finkit {
                CompatibilityStatus::Exact
            } else {
                CompatibilityStatus::Near
            };
            let message = match status {
                CompatibilityStatus::Exact => "native runtime implementation".to_string(),
                CompatibilityStatus::Near => {
                    "mapped through the canonical AlphaTA runtime".to_string()
                }
                CompatibilityStatus::Approximate => {
                    "future-data semantics need explicit review".to_string()
                }
                CompatibilityStatus::HostRequired => {
                    "requires host-provided market/session metadata".to_string()
                }
                CompatibilityStatus::Unsupported => {
                    if contract.is_some() {
                        "TA-Lib function is cataloged, but no compatible formula adapter is registered".to_string()
                    } else {
                        "no compatible runtime implementation is registered".to_string()
                    }
                }
            };
            FunctionCompatibility {
                name: name.clone(),
                status,
                message,
                cataloged: contract.is_some(),
                runtime_registered,
                category: contract.as_ref().map(|item| item.category.clone()),
                outputs: contract.as_ref().map(|item| item.outputs),
            }
        })
        .collect::<Vec<_>>();
    let capabilities =
        compatibility_capabilities(&normalized_source, &ast, &analysis, terminal, &functions);
    Ok(FormulaCompatibilityReport {
        terminal,
        normalized_source,
        profile: terminal.semantic_profile(),
        analysis,
        functions,
        capabilities,
        ta_lib_catalog_version: TA_LIB_CATALOG_VERSION.to_string(),
        ta_lib_function_count: catalog.len(),
        ta_lib_runtime_registered_count,
    })
}

fn compatibility_capabilities(
    source: &str,
    ast: &AstNode,
    analysis: &FormulaAnalysis,
    terminal: FormulaTerminal,
    functions: &[FunctionCompatibility],
) -> Vec<CapabilityCompatibility> {
    let mapped_status = if terminal == FormulaTerminal::Finkit {
        CompatibilityStatus::Exact
    } else {
        CompatibilityStatus::Near
    };
    let has_host_requirement = functions
        .iter()
        .any(|item| item.status == CompatibilityStatus::HostRequired);
    let has_unsupported = functions
        .iter()
        .any(|item| item.status == CompatibilityStatus::Unsupported);
    let drawing = ast_has_draw_commands(ast)
        || (terminal == FormulaTerminal::TradingView && ast_has_plot_outputs(ast));
    let cross_timeframe = analysis
        .called_functions
        .iter()
        .any(|name| matches!(name.as_str(), "SECURITY" | "REQUEST.SECURITY"));
    let host_data = has_host_requirement;
    // Static analysis is useful for diagnostics, but the public capability
    // report must be gated by the same serializable state compiler used by the
    // Formula stream contract. This keeps assignments and nested expressions
    // from being advertised as executable merely because their AST looks
    // causal.
    let stateful_stream_supported =
        FormulaStatefulStream::from_source(source, terminal.canonical_dialect()).is_ok();

    let batch_supported = !has_host_requirement && !has_unsupported;
    let batch_status = if has_unsupported {
        CompatibilityStatus::Unsupported
    } else if has_host_requirement {
        CompatibilityStatus::HostRequired
    } else {
        mapped_status
    };
    let batch_message = if has_unsupported {
        "one or more called functions are unsupported".to_string()
    } else if has_host_requirement {
        "host-provided market or timeframe metadata is required".to_string()
    } else {
        "canonical batch executor is available".to_string()
    };

    let streaming_supported = stateful_stream_supported && batch_supported;
    let streaming_status = if has_unsupported {
        CompatibilityStatus::Unsupported
    } else if has_host_requirement {
        CompatibilityStatus::HostRequired
    } else if analysis.has_future_data {
        CompatibilityStatus::Approximate
    } else if streaming_supported {
        mapped_status
    } else {
        CompatibilityStatus::Unsupported
    };
    let streaming_message = if streaming_supported {
        "formula has a causal, registered streaming path".to_string()
    } else if analysis.has_future_data {
        "future-data semantics prevent causal streaming".to_string()
    } else if !stateful_stream_supported {
        "formula is outside the portable serialized stateful stream subset".to_string()
    } else if analysis.has_control_flow {
        "control flow requires conservative full-prefix evaluation".to_string()
    } else {
        "no complete streaming implementation is registered".to_string()
    };

    vec![
        CapabilityCompatibility {
            name: "parser".to_string(),
            status: mapped_status,
            supported: true,
            observed: true,
            message: "source parsed through the selected canonical dialect".to_string(),
        },
        CapabilityCompatibility {
            name: "batch_execution".to_string(),
            status: batch_status,
            supported: batch_supported,
            observed: true,
            message: batch_message,
        },
        CapabilityCompatibility {
            name: "streaming_execution".to_string(),
            status: streaming_status,
            supported: streaming_supported,
            observed: stateful_stream_supported,
            message: streaming_message,
        },
        CapabilityCompatibility {
            name: "control_flow".to_string(),
            status: mapped_status,
            supported: true,
            observed: analysis.has_control_flow,
            message: if analysis.has_control_flow {
                "IF/loop control flow is evaluated by the canonical VM".to_string()
            } else {
                "formula does not use control flow".to_string()
            },
        },
        CapabilityCompatibility {
            name: "drawing".to_string(),
            status: mapped_status,
            supported: true,
            observed: drawing,
            message: if drawing && terminal == FormulaTerminal::TradingView {
                "Pine plot/hline/fill outputs are exposed through canonical visual channels"
                    .to_string()
            } else if drawing {
                "draw commands are lowered to the canonical DrawResult".to_string()
            } else {
                "formula does not emit drawing commands".to_string()
            },
        },
        CapabilityCompatibility {
            name: "cross_timeframe".to_string(),
            status: if cross_timeframe {
                CompatibilityStatus::HostRequired
            } else {
                mapped_status
            },
            supported: !cross_timeframe,
            observed: cross_timeframe,
            message: if cross_timeframe {
                "request.security/SECURITY requires explicit host timeframe alignment".to_string()
            } else {
                "formula does not request another timeframe".to_string()
            },
        },
        CapabilityCompatibility {
            name: "lookahead".to_string(),
            status: if analysis.has_future_data {
                CompatibilityStatus::Approximate
            } else {
                mapped_status
            },
            supported: !analysis.has_future_data,
            observed: analysis.has_future_data,
            message: if analysis.has_future_data {
                "future-data or repaint semantics require explicit review".to_string()
            } else {
                "formula is causal and does not use future rows".to_string()
            },
        },
        CapabilityCompatibility {
            name: "host_data".to_string(),
            status: if host_data {
                CompatibilityStatus::HostRequired
            } else {
                mapped_status
            },
            supported: !host_data,
            observed: host_data,
            message: if host_data {
                "formula references data that must be supplied by the host".to_string()
            } else {
                "formula uses only canonical frame inputs".to_string()
            },
        },
    ]
}

fn ast_has_draw_commands(node: &AstNode) -> bool {
    match node {
        AstNode::DrawText { .. }
        | AstNode::DrawIcon { .. }
        | AstNode::StickLine { .. }
        | AstNode::DrawGeneric { .. } => true,
        AstNode::FunctionCall { args, .. } => args.iter().any(ast_has_draw_commands),
        AstNode::BinaryOp { left, right, .. } => {
            ast_has_draw_commands(left) || ast_has_draw_commands(right)
        }
        AstNode::UnaryOp { expr, .. } => ast_has_draw_commands(expr),
        AstNode::IndexAccess { array, index } => {
            ast_has_draw_commands(array) || ast_has_draw_commands(index)
        }
        AstNode::Assignment { expr, .. }
        | AstNode::CompoundAssignment { expr, .. }
        | AstNode::Output { expr, .. } => ast_has_draw_commands(expr),
        AstNode::Statements(statements) => statements.iter().any(ast_has_draw_commands),
        AstNode::IfThenElse {
            cond,
            then_branch,
            else_branch,
        } => {
            ast_has_draw_commands(cond)
                || ast_has_draw_commands(then_branch)
                || ast_has_draw_commands(else_branch)
        }
        AstNode::ForLoop {
            start, end, body, ..
        } => {
            ast_has_draw_commands(start)
                || ast_has_draw_commands(end)
                || body.iter().any(ast_has_draw_commands)
        }
        AstNode::WhileLoop { cond, body } => {
            ast_has_draw_commands(cond) || body.iter().any(ast_has_draw_commands)
        }
        AstNode::Number(_)
        | AstNode::StringLit(_)
        | AstNode::Variable(_)
        | AstNode::ParamDecl { .. } => false,
    }
}

fn ast_has_plot_outputs(node: &AstNode) -> bool {
    match node {
        AstNode::Output { .. } => true,
        AstNode::FunctionCall { args, .. } => args.iter().any(ast_has_plot_outputs),
        AstNode::BinaryOp { left, right, .. } => {
            ast_has_plot_outputs(left) || ast_has_plot_outputs(right)
        }
        AstNode::UnaryOp { expr, .. } => ast_has_plot_outputs(expr),
        AstNode::IndexAccess { array, index } => {
            ast_has_plot_outputs(array) || ast_has_plot_outputs(index)
        }
        AstNode::Assignment { expr, .. } | AstNode::CompoundAssignment { expr, .. } => {
            ast_has_plot_outputs(expr)
        }
        AstNode::Statements(statements) => statements.iter().any(ast_has_plot_outputs),
        AstNode::IfThenElse {
            cond,
            then_branch,
            else_branch,
        } => {
            ast_has_plot_outputs(cond)
                || ast_has_plot_outputs(then_branch)
                || ast_has_plot_outputs(else_branch)
        }
        AstNode::ForLoop {
            start, end, body, ..
        } => {
            ast_has_plot_outputs(start)
                || ast_has_plot_outputs(end)
                || body.iter().any(ast_has_plot_outputs)
        }
        AstNode::WhileLoop { cond, body } => {
            ast_has_plot_outputs(cond) || body.iter().any(ast_has_plot_outputs)
        }
        AstNode::DrawText { .. }
        | AstNode::DrawIcon { .. }
        | AstNode::StickLine { .. }
        | AstNode::DrawGeneric { .. }
        | AstNode::Number(_)
        | AstNode::StringLit(_)
        | AstNode::Variable(_)
        | AstNode::ParamDecl { .. } => false,
    }
}

/// Normalize transport-level source differences before parsing.
///
/// This deliberately avoids unsafe textual rewrites of function semantics.
/// It removes a UTF-8 BOM and normalizes CRLF while leaving identifiers and
/// expressions unchanged for the canonical parser.
pub fn normalize_terminal_source(source: &str, _terminal: FormulaTerminal) -> String {
    source
        .strip_prefix('\u{feff}')
        .unwrap_or(source)
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_cover_major_declared_terminals() {
        assert_eq!(
            FormulaTerminal::from_str("通达信"),
            Some(FormulaTerminal::TongDaXin)
        );
        assert_eq!(
            FormulaTerminal::from_str("ths"),
            Some(FormulaTerminal::TongHuaShun)
        );
        assert_eq!(
            FormulaTerminal::from_str("东方财富"),
            Some(FormulaTerminal::EastMoney)
        );
        assert_eq!(
            FormulaTerminal::from_str("TradingView"),
            Some(FormulaTerminal::TradingView)
        );
        assert_eq!(
            FormulaTerminal::from_str("大智慧"),
            Some(FormulaTerminal::DaZhiHui)
        );
        assert_eq!(
            FormulaTerminal::from_str("dzh"),
            Some(FormulaTerminal::DaZhiHui)
        );
    }

    #[test]
    fn dazhihui_routes_through_the_shared_common_subset() {
        // 大智慧 is the sixth terminal. It has no dedicated dialect yet: it
        // intentionally reuses the 同花顺 common subset so the shared core
        // (MA/EMA/HHV/LLV/CROSS/...) routes without touching the exhaustive
        // dialect matches in `engine.rs`.
        assert_eq!(
            FormulaTerminal::DaZhiHui.canonical_dialect(),
            FormulaDialect::TongHuaShun
        );
        assert_eq!(FormulaTerminal::DaZhiHui.as_str(), "dzh");
        assert_eq!(
            FormulaTerminal::DaZhiHui.compatibility_level(),
            CompatibilityLevel::CommonSubset
        );
        let report =
            inspect_formula_compatibility("MA5:=MA(CLOSE,5); MA5", FormulaTerminal::DaZhiHui)
                .unwrap();
        assert!(report
            .functions
            .iter()
            .any(|item| item.name == "MA" && item.status == CompatibilityStatus::Near));
    }

    #[test]
    fn terminal_discovery_is_stable_and_complete() {
        assert_eq!(
            FORMULA_TERMINAL_SCHEMA_VERSION,
            "finkit.formula-terminal.v1"
        );
        assert_eq!(
            FormulaTerminal::all(),
            &[
                FormulaTerminal::Finkit,
                FormulaTerminal::TongDaXin,
                FormulaTerminal::TongHuaShun,
                FormulaTerminal::EastMoney,
                FormulaTerminal::TradingView,
                FormulaTerminal::DaZhiHui,
            ]
        );
    }

    #[test]
    fn external_terminals_are_explicit_subset_contracts() {
        assert_eq!(
            FormulaTerminal::Finkit.compatibility_level(),
            CompatibilityLevel::Native
        );
        assert_eq!(CompatibilityLevel::Native.as_str(), "native");
        assert_eq!(CompatibilityLevel::CommonSubset.as_str(), "common_subset");
        for terminal in [
            FormulaTerminal::TongDaXin,
            FormulaTerminal::TongHuaShun,
            FormulaTerminal::EastMoney,
            FormulaTerminal::TradingView,
            FormulaTerminal::DaZhiHui,
        ] {
            assert_eq!(
                terminal.compatibility_level(),
                CompatibilityLevel::CommonSubset
            );
        }
    }

    #[test]
    fn china_terminals_have_distinct_profiles_over_shared_parser() {
        assert_eq!(
            FormulaTerminal::Finkit.canonical_dialect(),
            FormulaDialect::AlphaTA
        );
        assert_eq!(
            FormulaTerminal::TongDaXin.canonical_dialect(),
            FormulaDialect::TongDaXin
        );
        assert_eq!(
            FormulaTerminal::TongHuaShun.canonical_dialect(),
            FormulaDialect::TongHuaShun
        );
        assert_eq!(
            FormulaTerminal::EastMoney.canonical_dialect(),
            FormulaDialect::EastMoney
        );
    }

    #[test]
    fn normalizer_only_changes_transport_artifacts() {
        let source = "\u{feff}MA5:=MA(CLOSE,5);\r\nCROSS(CLOSE,MA5);\r";
        let normalized = normalize_terminal_source(source, FormulaTerminal::TongDaXin);
        assert_eq!(normalized, "MA5:=MA(CLOSE,5);\nCROSS(CLOSE,MA5);\n");
    }

    #[test]
    fn compatibility_report_exposes_semantics_and_host_requirements() {
        let report = inspect_formula_compatibility(
            "X:=MA(CLOSE,5); X + SECURITY(CLOSE, 'WEEK')",
            FormulaTerminal::TongDaXin,
        )
        .unwrap();
        assert_eq!(report.profile.sma_policy, "recursive-sma");
        assert!(report
            .functions
            .iter()
            .any(|item| item.name == "MA" && item.status == CompatibilityStatus::Near));
        assert!(report.functions.iter().any(|item| {
            item.name == "SECURITY" && item.status == CompatibilityStatus::HostRequired
        }));
        let timeframe = report
            .capabilities
            .iter()
            .find(|item| item.name == "cross_timeframe")
            .unwrap();
        assert!(timeframe.observed);
        assert!(!timeframe.supported);
        assert_eq!(timeframe.status, CompatibilityStatus::HostRequired);
    }

    #[test]
    fn capability_matrix_distinguishes_drawing_control_flow_and_streaming() {
        let report = inspect_formula_compatibility(
            "DRAWICON(CLOSE > OPEN, CLOSE, 1); IF CLOSE > OPEN THEN CLOSE ELSE OPEN",
            FormulaTerminal::TongDaXin,
        )
        .unwrap();
        let capability = |name: &str| report.capabilities.iter().find(|item| item.name == name);
        assert!(capability("drawing").unwrap().observed);
        assert!(capability("control_flow").unwrap().observed);
        assert!(!capability("streaming_execution").unwrap().supported);
        assert_eq!(
            capability("streaming_execution").unwrap().status,
            CompatibilityStatus::Unsupported
        );
    }

    #[test]
    fn capability_matrix_uses_the_real_stateful_program_admission_check() {
        let report = inspect_formula_compatibility(
            "MA3:MA(CLOSE,3); SIGNAL:=MA3+1; SIGNAL",
            FormulaTerminal::TongDaXin,
        )
        .unwrap();
        let capability = report
            .capabilities
            .iter()
            .find(|item| item.name == "streaming_execution")
            .unwrap();
        assert!(capability.observed);
        assert!(capability.supported);
        assert_eq!(capability.status, CompatibilityStatus::Near);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn compatibility_report_serializes_stable_wire_names() {
        let report =
            inspect_formula_compatibility("MA(CLOSE,5)", FormulaTerminal::TongDaXin).unwrap();
        let value = serde_json::to_value(report).unwrap();
        assert_eq!(value["terminal"], "tdx");
        assert_eq!(value["functions"][0]["status"], "near");
        assert_eq!(value["capabilities"][0]["status"], "near");
    }
}
