//! Trading-terminal compatibility routing for the formula engine.
//!
//! Finkit keeps one canonical AST/runtime. Terminal names are resolved to the
//! closest canonical parser instead of maintaining divergent executors. The
//! v0.1.2 China-terminal adapters intentionally target the common TDX-style
//! formula subset; terminal-specific extensions can be added without changing
//! the execution engine.

use super::FormulaDialect;

/// Formula source terminal understood by the compatibility layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormulaTerminal {
    /// Native Finkit / AlphaTA-compatible formula syntax.
    Finkit,
    /// 通达信 formula syntax.
    TongDaXin,
    /// 同花顺 common formula subset.
    TongHuaShun,
    /// 东方财富 common formula subset.
    EastMoney,
    /// TradingView Pine Script subset.
    TradingView,
}

/// Declared compatibility strength for a terminal adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityLevel {
    /// Native parser/runtime support.
    Native,
    /// A documented common syntax/function subset is supported.
    CommonSubset,
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
    pub const fn compatibility_level(self) -> CompatibilityLevel {
        match self {
            Self::Finkit | Self::TongDaXin | Self::TradingView => CompatibilityLevel::Native,
            Self::TongHuaShun | Self::EastMoney => CompatibilityLevel::CommonSubset,
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
}
