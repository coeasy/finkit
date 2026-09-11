//! Conservative TA-Lib compatibility catalog for formula tooling.
//!
//! The catalog is deliberately separate from the executor.  A name being in
//! the TA-Lib catalog must not imply that Finkit silently emulates it: the
//! `runtime_registered` flag is derived from the actual formula function map
//! and compatibility reports can therefore distinguish implemented, host
//! supplied and unsupported functions.

use super::functions::get_builtin_functions;

/// Catalog revision used by compatibility reports and generated clients.
pub const TA_LIB_CATALOG_VERSION: &str = "ta-lib-python-0.6.x-public-v1";

/// Machine-readable contract for one public TA-Lib function name.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FormulaFunctionContract {
    pub name: String,
    pub category: String,
    pub input_shape: String,
    pub outputs: usize,
    pub lookback: String,
    pub warmup_policy: String,
    pub nan_policy: String,
    pub runtime_registered: bool,
}

const TA_LIB_PUBLIC_FUNCTIONS: &[&str] = &[
    "BBANDS",
    "DEMA",
    "EMA",
    "HT_TRENDLINE",
    "KAMA",
    "MA",
    "MAMA",
    "MAVP",
    "MIDPOINT",
    "MIDPRICE",
    "SAR",
    "SAREXT",
    "SMA",
    "T3",
    "TEMA",
    "TRIMA",
    "WMA",
    "ADX",
    "ADXR",
    "APO",
    "AROON",
    "AROONOSC",
    "BOP",
    "CCI",
    "CMO",
    "DX",
    "MACD",
    "MACDEXT",
    "MACDFIX",
    "MFI",
    "MINUS_DI",
    "MINUS_DM",
    "MOM",
    "PLUS_DI",
    "PLUS_DM",
    "PPO",
    "ROC",
    "ROCP",
    "ROCR",
    "ROCR100",
    "RSI",
    "STOCH",
    "STOCHF",
    "STOCHRSI",
    "TRIX",
    "ULTOSC",
    "WILLR",
    "AD",
    "ADOSC",
    "OBV",
    "ATR",
    "NATR",
    "TRANGE",
    "AVGPRICE",
    "MEDPRICE",
    "TYPPRICE",
    "WCLPRICE",
    "HT_DCPERIOD",
    "HT_DCPHASE",
    "HT_PHASOR",
    "HT_SINE",
    "HT_TRENDMODE",
    "CDL2CROWS",
    "CDL3BLACKCROWS",
    "CDL3INSIDE",
    "CDL3LINESTRIKE",
    "CDL3OUTSIDE",
    "CDL3STARSINSOUTH",
    "CDL3WHITESOLDIERS",
    "CDLABANDONEDBABY",
    "CDLADVANCEBLOCK",
    "CDLBELTHOLD",
    "CDLBREAKAWAY",
    "CDLCLOSINGMARUBOZU",
    "CDLCONCEALBABYSWALL",
    "CDLCOUNTERATTACK",
    "CDLDARKCLOUDCOVER",
    "CDLDOJI",
    "CDLDOJISTAR",
    "CDLDRAGONFLYDOJI",
    "CDLENGULFING",
    "CDLEVENINGDOJISTAR",
    "CDLEVENINGSTAR",
    "CDLGAPSIDESIDEWHITE",
    "CDLGRAVESTONEDOJI",
    "CDLHAMMER",
    "CDLHANGINGMAN",
    "CDLHARAMI",
    "CDLHARAMICROSS",
    "CDLHIGHWAVE",
    "CDLHIKKAKE",
    "CDLHIKKAKEMOD",
    "CDLHOMINGPIGEON",
    "CDLIDENTICAL3CROWS",
    "CDLINNECK",
    "CDLINVERTEDHAMMER",
    "CDLKICKING",
    "CDLKICKINGBYLENGTH",
    "CDLLADDERBOTTOM",
    "CDLLONGLEGGEDDOJI",
    "CDLLONGLINE",
    "CDLMARUBOZU",
    "CDLMATCHINGLOW",
    "CDLMATHOLD",
    "CDLMORNINGDOJISTAR",
    "CDLMORNINGSTAR",
    "CDLONNECK",
    "CDLPIERCING",
    "CDLRICKSHAWMAN",
    "CDLRISEFALL3METHODS",
    "CDLSEPARATINGLINES",
    "CDLSHOOTINGSTAR",
    "CDLSHORTLINE",
    "CDLSPINNINGTOP",
    "CDLSTALLEDPATTERN",
    "CDLSTICKSANDWICH",
    "CDLTAKURI",
    "CDLTASUKIGAP",
    "CDLTHRUSTING",
    "CDLTRISTAR",
    "CDLUNIQUE3RIVER",
    "CDLUPSIDEGAP2CROWS",
    "CDLXSIDEGAP3METHODS",
    "BETA",
    "CORREL",
    "LINEARREG",
    "LINEARREG_ANGLE",
    "LINEARREG_INTERCEPT",
    "LINEARREG_SLOPE",
    "STDDEV",
    "TSF",
    "VAR",
    "ACOS",
    "ASIN",
    "ATAN",
    "CEIL",
    "COS",
    "COSH",
    "EXP",
    "FLOOR",
    "LN",
    "LOG10",
    "SIN",
    "SINH",
    "SQRT",
    "TAN",
    "TANH",
    "ADD",
    "DIV",
    "MAX",
    "MAXINDEX",
    "MIN",
    "MININDEX",
    "MINMAX",
    "MINMAXINDEX",
    "MULT",
    "SUB",
    "SUM",
    "ACCBANDS",
    "AVGDEV",
    "IMI",
];

fn category(name: &str) -> &'static str {
    if name.starts_with("CDL") {
        "pattern"
    } else if matches!(
        name,
        "ACOS"
            | "ASIN"
            | "ATAN"
            | "CEIL"
            | "COS"
            | "COSH"
            | "EXP"
            | "FLOOR"
            | "LN"
            | "LOG10"
            | "SIN"
            | "SINH"
            | "SQRT"
            | "TAN"
            | "TANH"
            | "ADD"
            | "DIV"
            | "MAX"
            | "MAXINDEX"
            | "MIN"
            | "MININDEX"
            | "MINMAX"
            | "MINMAXINDEX"
            | "MULT"
            | "SUB"
            | "SUM"
    ) {
        "math"
    } else if matches!(name, "AD" | "ADOSC" | "OBV" | "MFI") {
        "volume"
    } else if matches!(name, "ATR" | "NATR" | "TRANGE" | "BBANDS" | "ACCBANDS") {
        "volatility"
    } else if matches!(name, "AVGPRICE" | "MEDPRICE" | "TYPPRICE" | "WCLPRICE") {
        "price_transform"
    } else if name.starts_with("HT_") {
        "cycle"
    } else if matches!(
        name,
        "MA" | "SMA"
            | "EMA"
            | "DEMA"
            | "TEMA"
            | "TRIMA"
            | "WMA"
            | "T3"
            | "KAMA"
            | "MAMA"
            | "MAVP"
            | "MIDPOINT"
            | "MIDPRICE"
            | "SAR"
            | "SAREXT"
            | "HT_TRENDLINE"
    ) {
        "overlap"
    } else if matches!(
        name,
        "BETA"
            | "CORREL"
            | "LINEARREG"
            | "LINEARREG_ANGLE"
            | "LINEARREG_INTERCEPT"
            | "LINEARREG_SLOPE"
            | "STDDEV"
            | "TSF"
            | "VAR"
            | "AVGDEV"
    ) {
        "statistics"
    } else {
        "momentum"
    }
}

fn output_count(name: &str) -> usize {
    match name {
        "MACD" | "MACDEXT" | "MACDFIX" => 3,
        "BBANDS" | "AROON" | "STOCH" | "STOCHF" | "STOCHRSI" | "HT_PHASOR" | "HT_SINE"
        | "MINMAX" | "MINMAXINDEX" => 2,
        _ => 1,
    }
}

/// Return the complete public TA-Lib catalog in deterministic order.
pub fn ta_lib_function_contracts() -> Vec<FormulaFunctionContract> {
    let builtins = get_builtin_functions();
    TA_LIB_PUBLIC_FUNCTIONS
        .iter()
        .map(|name| FormulaFunctionContract {
            name: (*name).to_string(),
            category: category(name).to_string(),
            input_shape: "dynamic".to_string(),
            outputs: output_count(name),
            lookback: "dynamic".to_string(),
            warmup_policy: "implementation-defined; verify with golden vectors".to_string(),
            nan_policy: "NaN for unavailable rows; differential-check exact edge cases".to_string(),
            runtime_registered: builtins.contains_key(*name),
        })
        .collect()
}

/// Look up one catalog entry without exposing the internal static slice.
pub fn ta_lib_function_contract(name: &str) -> Option<FormulaFunctionContract> {
    let requested = name.trim().to_ascii_uppercase();
    let name = TA_LIB_PUBLIC_FUNCTIONS
        .iter()
        .copied()
        .find(|item| *item == requested)?;
    let runtime_registered = get_builtin_functions().contains_key(name);
    Some(FormulaFunctionContract {
        name: name.to_string(),
        category: category(name).to_string(),
        input_shape: "dynamic".to_string(),
        outputs: output_count(name),
        lookback: "dynamic".to_string(),
        warmup_policy: "implementation-defined; verify with golden vectors".to_string(),
        nan_policy: "NaN for unavailable rows; differential-check exact edge cases".to_string(),
        runtime_registered,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_complete_and_unique() {
        let contracts = ta_lib_function_contracts();
        assert_eq!(contracts.len(), 161);
        let mut names: Vec<_> = contracts.iter().map(|item| item.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), contracts.len());
        assert_eq!(contracts[0].name, "BBANDS");
        assert!(contracts.iter().any(|item| item.name == "CDLDOJI"));
    }
}
