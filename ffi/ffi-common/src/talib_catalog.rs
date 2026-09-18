//! TA-Lib profile-only names exposed by the shared dispatcher.
//!
//! The Core registry describes the native/core semantic surface.  The
//! versioned TA-Lib profile is a second, explicit public surface and must not
//! disappear from discovery merely because a function has no Core equivalent.
//! Some names are also present in the Core registry. The complete dispatcher
//! surface is the union of this profile-only list and Core registry names;
//! keeping the profile-only list here makes discovery deterministic across all
//! bindings.

/// Canonical semantic profile identifier for the current TA-Lib core contract.
///
/// Keep this in one module so the dispatcher, operation catalog and bindings
/// cannot silently drift to different version strings.
pub const TALIB_SEMANTIC_PROFILE: &str = "talib_0_7_1";

/// Upstream TA-Lib core version represented by [`TALIB_SEMANTIC_PROFILE`].
pub const TALIB_CORE_VERSION: &str = "0.7.1";

/// TA-Lib 0.7.1 profile-only names exposed by the versioned dispatcher.
pub const TALIB_PROFILE_CATALOG_NAMES: &[&str] = &[
    "ACOS",
    "ADX",
    "ADXR",
    "APO",
    "AROON",
    "AROONOSC",
    "ASIN",
    "ATAN",
    "AVGPRICE",
    "BOP",
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
    "CEIL",
    "COS",
    "COSH",
    "DX",
    "EXP",
    "FLOOR",
    "LN",
    "LOG10",
    "MEDPRICE",
    "MIDPOINT",
    "MIDPRICE",
    "MINUS_DI",
    "PLUS_DI",
    "ROCP",
    "ROCR",
    "ROCR100",
    "SAR",
    "SIN",
    "SINH",
    "SQRT",
    "STOCH",
    "STOCHF",
    "STOCHRSI",
    "TAN",
    "TANH",
    "TRANGE",
    "TRIX",
    "TYPPRICE",
    "WCLPRICE",
    "WILLR",
];

#[inline]
pub fn is_profile_catalog_name(name: &str) -> bool {
    TALIB_PROFILE_CATALOG_NAMES.contains(&name)
}
