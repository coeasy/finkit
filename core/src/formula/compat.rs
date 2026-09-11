//! Trading-terminal compatibility routing for the formula engine.
//!
//! Finkit keeps one canonical AST/runtime. Terminal names are resolved to the
//! closest canonical parser instead of maintaining divergent executors. The
//! v0.1.2 external-terminal adapters intentionally target documented common
//! subsets; terminal-specific extensions can be added without changing the
//! execution engine.

use super::FormulaDialect;
use crate::formula::analysis::{analyze_formula, FormulaAnalysis};
use crate::formula::contracts::{ta_lib_function_contracts, TA_LIB_CATALOG_VERSION};
use std::collections::HashMap;

/// Stable schema identifier for terminal compatibility discovery.
pub const FORMULA_TERMINAL_SCHEMA_VERSION: &str = "finkit.formula-terminal.v1";

/// Formula source terminal understood by the compatibility layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FormulaTerminal {
    /// Native Finkit / AlphaTA-compatible formula syntax.
    Finkit,
    /// 通达信 common formula subset.
    TongDaXin,
    /// 同花顺 common formula subset.
    TongHuaShun,
    /// 东方财富 common formula subset.
    EastMoney,
    /// TradingView Pine Script subset.
    TradingView,
}

/// All declared formula terminals in stable discovery order.
pub const FORMULA_TERMINALS: &[FormulaTerminal] = &[
    FormulaTerminal::Finkit,
    FormulaTerminal::TongDaXin,
    FormulaTerminal::TongHuaShun,
    FormulaTerminal::EastMoney,
    FormulaTerminal::TradingView,
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

/// Complete source compatibility report.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FormulaCompatibilityReport {
    pub terminal: FormulaTerminal,
    pub normalized_source: String,
    pub profile: SemanticProfile,
    pub analysis: FormulaAnalysis,
    pub functions: Vec<FunctionCompatibility>,
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
            Self::Finkit | Self::TongDaXin | Self::TongHuaShun | Self::EastMoney => {
                FormulaDialect::AlphaTA
            }
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
            Self::TongDaXin | Self::TongHuaShun | Self::EastMoney | Self::TradingView => {
                CompatibilityLevel::CommonSubset
            }
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
        .collect();
    Ok(FormulaCompatibilityReport {
        terminal,
        normalized_source,
        profile: terminal.semantic_profile(),
        analysis,
        functions,
        ta_lib_catalog_version: TA_LIB_CATALOG_VERSION.to_string(),
        ta_lib_function_count: catalog.len(),
        ta_lib_runtime_registered_count,
    })
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
        ] {
            assert_eq!(
                terminal.compatibility_level(),
                CompatibilityLevel::CommonSubset
            );
        }
    }

    #[test]
    fn china_terminals_share_canonical_tdx_style_parser() {
        for terminal in [
            FormulaTerminal::Finkit,
            FormulaTerminal::TongDaXin,
            FormulaTerminal::TongHuaShun,
            FormulaTerminal::EastMoney,
        ] {
            assert_eq!(terminal.canonical_dialect(), FormulaDialect::AlphaTA);
        }
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
    }
}
