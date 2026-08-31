//! Pine Script built-in function mapping table.
//!
//! Maps Pine namespace functions to AlphaTA formula function names.
//! Multi-return functions (e.g. MACD) map to separate AlphaTA outputs.

use std::collections::HashMap;

/// A single built-in function mapping entry.
#[derive(Debug, Clone)]
pub struct BuiltinMapping {
    /// Pine namespace (e.g. "ta", "math"). None for unqualified calls.
    pub namespace: Option<String>,
    /// Pine function name (e.g. "sma", "ema").
    pub pine_name: String,
    /// AlphaTA target function name (e.g. "SMA", "EMA").
    pub alpha_ta_name: String,
    /// Whether this function returns multiple values.
    pub multi_return: bool,
    /// AlphaTA names for each return value (MACD → DIF, DEA, MACD).
    pub return_names: Vec<String>,
    /// Human-readable description.
    pub description: String,
}

/// Lookup table for Pine → AlphaTA built-in functions.
#[derive(Debug, Clone)]
pub struct PineBuiltinTable {
    entries: Vec<BuiltinMapping>,
    lookup: HashMap<String, BuiltinMapping>,
}

impl PineBuiltinTable {
    /// Create the default mapping table with all documented entries.
    pub fn new() -> Self {
        let entries = default_mappings();
        let lookup = build_lookup(&entries);
        Self { entries, lookup }
    }

    /// All mapping entries (for documentation / introspection).
    pub fn entries(&self) -> &[BuiltinMapping] {
        &self.entries
    }

    /// Resolve a Pine call to AlphaTA function name.
    ///
    /// `namespace` is e.g. "ta", `name` is e.g. "sma".
    /// Returns None if no mapping exists.
    pub fn resolve(&self, namespace: Option<&str>, name: &str) -> Option<&BuiltinMapping> {
        let key = make_key(namespace, name);
        self.lookup.get(&key)
    }

    /// Resolve with fallback: unqualified name lookup.
    pub fn resolve_any(&self, namespace: Option<&str>, name: &str) -> Option<&BuiltinMapping> {
        self.resolve(namespace, name)
            .or_else(|| self.resolve(None, name))
    }
}

impl Default for PineBuiltinTable {
    fn default() -> Self {
        Self::new()
    }
}

fn make_key(namespace: Option<&str>, name: &str) -> String {
    match namespace {
        Some(ns) => format!("{}::{}", ns, name),
        None => name.to_string(),
    }
}

fn build_lookup(entries: &[BuiltinMapping]) -> HashMap<String, BuiltinMapping> {
    let mut map = HashMap::new();
    for entry in entries {
        let key = make_key(entry.namespace.as_deref(), &entry.pine_name);
        map.insert(key, entry.clone());
    }
    map
}

/// Documented default mappings per TASK-240 spec.
fn default_mappings() -> Vec<BuiltinMapping> {
    vec![
        // --- ta.* technical analysis ---
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "sma".to_string(),
            alpha_ta_name: "SMA".to_string(),
            multi_return: false,
            return_names: vec!["SMA".to_string()],
            description: "Simple Moving Average — ta.sma(source, length) → SMA".to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "ema".to_string(),
            alpha_ta_name: "EMA".to_string(),
            multi_return: false,
            return_names: vec!["EMA".to_string()],
            description: "Exponential Moving Average — ta.ema(source, length) → EMA".to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "rsi".to_string(),
            alpha_ta_name: "RSI".to_string(),
            multi_return: false,
            return_names: vec!["RSI".to_string()],
            description: "Relative Strength Index — ta.rsi(source, length) → RSI".to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "macd".to_string(),
            alpha_ta_name: "MACD".to_string(),
            multi_return: true,
            return_names: vec!["DIF".to_string(), "DEA".to_string(), "MACD".to_string()],
            description:
                "MACD — ta.macd(source, fast, slow, signal) → DIF, DEA, MACD (multi-return)"
                    .to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "atr".to_string(),
            alpha_ta_name: "ATR".to_string(),
            multi_return: false,
            return_names: vec!["ATR".to_string()],
            description: "Average True Range — ta.atr(length) → ATR".to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "stoch".to_string(),
            alpha_ta_name: "STOCH".to_string(),
            multi_return: true,
            return_names: vec!["K".to_string(), "D".to_string()],
            description: "Stochastic / KDJ — ta.stoch(...) → K, D (maps to KDJ/STOCH)".to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "cci".to_string(),
            alpha_ta_name: "CCI".to_string(),
            multi_return: false,
            return_names: vec!["CCI".to_string()],
            description: "Commodity Channel Index — ta.cci(source, length) → CCI".to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "bb".to_string(),
            alpha_ta_name: "BOLL".to_string(),
            multi_return: true,
            return_names: vec![
                "BOLLMID".to_string(),
                "BOLLUP".to_string(),
                "BOLLDN".to_string(),
            ],
            description: "Bollinger Bands — ta.bb(source, length, mult) → BOLLMID, BOLLUP, BOLLDN"
                .to_string(),
        },
        // --- math.* ---
        BuiltinMapping {
            namespace: Some("math".to_string()),
            pine_name: "abs".to_string(),
            alpha_ta_name: "ABS".to_string(),
            multi_return: false,
            return_names: vec!["ABS".to_string()],
            description: "Absolute value — math.abs(x) → ABS".to_string(),
        },
        BuiltinMapping {
            namespace: Some("math".to_string()),
            pine_name: "log".to_string(),
            alpha_ta_name: "LOG".to_string(),
            multi_return: false,
            return_names: vec!["LOG".to_string()],
            description: "Natural logarithm — math.log(x) → LOG".to_string(),
        },
        BuiltinMapping {
            namespace: Some("math".to_string()),
            pine_name: "max".to_string(),
            alpha_ta_name: "MAX".to_string(),
            multi_return: false,
            return_names: vec!["MAX".to_string()],
            description: "Maximum — math.max(a, b) → MAX".to_string(),
        },
        BuiltinMapping {
            namespace: Some("math".to_string()),
            pine_name: "min".to_string(),
            alpha_ta_name: "MIN".to_string(),
            multi_return: false,
            return_names: vec!["MIN".to_string()],
            description: "Minimum — math.min(a, b) → MIN".to_string(),
        },
        BuiltinMapping {
            namespace: Some("math".to_string()),
            pine_name: "pow".to_string(),
            alpha_ta_name: "POW".to_string(),
            multi_return: false,
            return_names: vec!["POW".to_string()],
            description: "Power — math.pow(base, exp) → POW".to_string(),
        },
        BuiltinMapping {
            namespace: Some("math".to_string()),
            pine_name: "sqrt".to_string(),
            alpha_ta_name: "SQRT".to_string(),
            multi_return: false,
            return_names: vec!["SQRT".to_string()],
            description: "Square root — math.sqrt(x) → SQRT".to_string(),
        },
        // --- na helpers (Pine builtins, mapped to AlphaTA equivalents) ---
        BuiltinMapping {
            namespace: None,
            pine_name: "nz".to_string(),
            alpha_ta_name: "IF".to_string(),
            multi_return: false,
            return_names: vec!["IF".to_string()],
            description: "Replace na — nz(x, y) → IF(ISNA(x), y, x)".to_string(),
        },
        BuiltinMapping {
            namespace: None,
            pine_name: "na".to_string(),
            alpha_ta_name: "ISNA".to_string(),
            multi_return: false,
            return_names: vec!["ISNA".to_string()],
            description: "Check na — na(x) → ISNA(x)".to_string(),
        },
        BuiltinMapping {
            namespace: None,
            pine_name: "fixnan".to_string(),
            alpha_ta_name: "FIXNAN".to_string(),
            multi_return: false,
            return_names: vec!["FIXNAN".to_string()],
            description: "Forward-fill na — fixnan(x) → FIXNAN(x)".to_string(),
        },
        // --- request.security ---
        BuiltinMapping {
            namespace: Some("request".to_string()),
            pine_name: "security".to_string(),
            alpha_ta_name: "SECURITY".to_string(),
            multi_return: false,
            return_names: vec!["SECURITY".to_string()],
            description: "Cross-timeframe data — request.security(sym, tf, expr) → SECURITY"
                .to_string(),
        },
        // --- ta.* additional technical analysis mappings ---
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "supertrend".to_string(),
            alpha_ta_name: "SUPERTREND".to_string(),
            multi_return: true,
            return_names: vec!["SUPERTREND".to_string(), "DIRECTION".to_string()],
            description: "SuperTrend — ta.supertrend(factor, atrPeriod) → SUPERTREND, DIRECTION"
                .to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "vwap".to_string(),
            alpha_ta_name: "VWAP".to_string(),
            multi_return: false,
            return_names: vec!["VWAP".to_string()],
            description: "Volume Weighted Average Price — ta.vwap(source) → VWAP".to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "wpr".to_string(),
            alpha_ta_name: "WILLR".to_string(),
            multi_return: false,
            return_names: vec!["WILLR".to_string()],
            description: "Williams %R — ta.wpr(length) → WILLR".to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "obv".to_string(),
            alpha_ta_name: "OBV".to_string(),
            multi_return: false,
            return_names: vec!["OBV".to_string()],
            description: "On Balance Volume — ta.obv → OBV".to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "dmi".to_string(),
            alpha_ta_name: "ADX".to_string(),
            multi_return: true,
            return_names: vec![
                "ADX".to_string(),
                "PLUS_DI".to_string(),
                "MINUS_DI".to_string(),
            ],
            description:
                "Directional Movement Index — ta.dmi(diLength, adxSmoothing) → ADX, +DI, -DI"
                    .to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "sar".to_string(),
            alpha_ta_name: "SAR".to_string(),
            multi_return: false,
            return_names: vec!["SAR".to_string()],
            description: "Parabolic SAR — ta.sar(start, inc, max) → SAR".to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "mom".to_string(),
            alpha_ta_name: "MOM".to_string(),
            multi_return: false,
            return_names: vec!["MOM".to_string()],
            description: "Momentum — ta.mom(source, length) → MOM".to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "roc".to_string(),
            alpha_ta_name: "ROC".to_string(),
            multi_return: false,
            return_names: vec!["ROC".to_string()],
            description: "Rate of Change — ta.roc(source, length) → ROC".to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "aroon".to_string(),
            alpha_ta_name: "AROON".to_string(),
            multi_return: true,
            return_names: vec!["AROON_UP".to_string(), "AROON_DN".to_string()],
            description: "Aroon — ta.aroon(length) → AROON_UP, AROON_DN".to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "trix".to_string(),
            alpha_ta_name: "TRIX".to_string(),
            multi_return: false,
            return_names: vec!["TRIX".to_string()],
            description: "TRIX — ta.trix(length) → TRIX".to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "vwma".to_string(),
            alpha_ta_name: "VWMA".to_string(),
            multi_return: false,
            return_names: vec!["VWMA".to_string()],
            description: "Volume Weighted Moving Average — ta.vwma(source, length) → VWMA"
                .to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "highest".to_string(),
            alpha_ta_name: "MAX".to_string(),
            multi_return: false,
            return_names: vec!["MAX".to_string()],
            description: "Highest value — ta.highest(source, length) → HHV / MAX".to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "lowest".to_string(),
            alpha_ta_name: "MIN".to_string(),
            multi_return: false,
            return_names: vec!["MIN".to_string()],
            description: "Lowest value — ta.lowest(source, length) → LLV / MIN".to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "change".to_string(),
            alpha_ta_name: "MOM".to_string(),
            multi_return: false,
            return_names: vec!["MOM".to_string()],
            description: "Change (difference) — ta.change(source, length) → MOM".to_string(),
        },
        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "crossover".to_string(),
            alpha_ta_name: "CROSS".to_string(),
            multi_return: false,
            return_names: vec!["CROSS".to_string()],
            description: "Crossover detection — ta.crossover(a, b) → CROSS".to_string(),
        },
    ]
}

/// Generate a markdown documentation table of all mappings.
pub fn mapping_doc() -> String {
    let table = PineBuiltinTable::new();
    let mut doc = String::from("# Pine → AlphaTA Built-in Function Mapping\n\n");
    doc.push_str("| Namespace | Pine | AlphaTA | Multi-return | Returns | Description |\n");
    doc.push_str("|-----------|------|--------|--------------|---------|-------------|\n");
    for e in table.entries() {
        let ns = e.namespace.as_deref().unwrap_or("-");
        let returns = e.return_names.join(", ");
        doc.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            ns, e.pine_name, e.alpha_ta_name, e.multi_return, returns, e.description
        ));
    }
    doc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_ta_sma() {
        let table = PineBuiltinTable::new();
        let m = table.resolve(Some("ta"), "sma").unwrap();
        assert_eq!(m.alpha_ta_name, "SMA");
    }

    #[test]
    fn test_resolve_macd_multi_return() {
        let table = PineBuiltinTable::new();
        let m = table.resolve(Some("ta"), "macd").unwrap();
        assert!(m.multi_return);
        assert_eq!(m.return_names.len(), 3);
    }

    #[test]
    fn test_resolve_math_abs() {
        let table = PineBuiltinTable::new();
        let m = table.resolve(Some("math"), "abs").unwrap();
        assert_eq!(m.alpha_ta_name, "ABS");
    }

    #[test]
    fn test_mapping_doc_not_empty() {
        let doc = mapping_doc();
        assert!(doc.contains("ta.sma"));
        assert!(doc.contains("SMA"));
    }
}
