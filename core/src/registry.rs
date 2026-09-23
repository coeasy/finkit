//! Stable function metadata and introspection registry.
//!
//! The registry does not replace the existing indicator or formula executors.
//! It gives bindings, CLI tools, documentation generators, and compatibility
//! layers one canonical description of public functions.

use std::collections::BTreeMap;

/// High-level category used for discovery and documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FunctionCategory {
    /// Moving averages and trend overlays.
    Overlap,
    /// Momentum and oscillator indicators.
    Momentum,
    /// Volatility indicators.
    Volatility,
    /// Volume indicators.
    Volume,
    /// Statistical functions.
    Statistics,
    /// Formula time-series primitive.
    Formula,
    /// Factor transform or factor helper.
    Factor,
}

/// Required input shape for a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    /// A single numeric series.
    Series,
    /// High, low, close series.
    Hlc,
    /// High, low, close, volume series.
    Hlcv,
    /// Open, high, low, close, volume series.
    Ohlcv,
    /// Formula expression arguments determine the exact inputs.
    Dynamic,
}

/// Parameter metadata used by bindings and help output.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamSpec {
    /// Stable parameter name.
    pub name: &'static str,
    /// Human-readable type name.
    pub value_type: &'static str,
    /// Optional default value rendered as text.
    pub default: Option<&'static str>,
    /// Human-readable constraint.
    pub constraint: Option<&'static str>,
}

impl ParamSpec {
    /// Create a parameter description.
    pub const fn new(
        name: &'static str,
        value_type: &'static str,
        default: Option<&'static str>,
        constraint: Option<&'static str>,
    ) -> Self {
        Self {
            name,
            value_type,
            default,
            constraint,
        }
    }
}

/// Lookback contract for warm-up behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookbackSpec {
    /// No warm-up rows are required.
    None,
    /// Warm-up equals `period - 1` for the primary period argument.
    PeriodMinusOne,
    /// Warm-up equals the primary period argument.
    Period,
    /// Function-specific lookback; consult the implementation.
    Dynamic,
}

/// Canonical metadata for a public indicator/formula function.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSpec {
    /// Canonical uppercase name.
    pub name: &'static str,
    /// Accepted compatibility aliases.
    pub aliases: &'static [&'static str],
    /// Discovery category.
    pub category: FunctionCategory,
    /// Input shape.
    pub input: InputKind,
    /// Parameter declarations.
    pub params: &'static [ParamSpec],
    /// Number of output series.
    pub outputs: usize,
    /// Warm-up/lookback behavior.
    pub lookback: LookbackSpec,
    /// Whether the function can be evaluated incrementally.
    pub streaming: bool,
    /// Whether repeated execution with identical input is deterministic.
    pub deterministic: bool,
}

/// Deterministic registry for function metadata.
#[derive(Debug, Clone, Default)]
pub struct FunctionRegistry {
    specs: BTreeMap<String, FunctionSpec>,
    aliases: BTreeMap<String, String>,
}

impl FunctionRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a function and all of its aliases.
    pub fn register(&mut self, spec: FunctionSpec) -> Result<(), String> {
        let canonical = normalize_name(spec.name);
        if canonical.is_empty() {
            return Err("function name must not be empty".to_string());
        }
        if self.specs.contains_key(&canonical) {
            return Err(format!("function already registered: {}", spec.name));
        }
        if self.aliases.contains_key(&canonical) {
            return Err(format!("function name conflicts with alias: {}", spec.name));
        }

        // Validate every alias before mutating either map. This keeps the
        // operation atomic and prevents ambiguous canonical/alias lookups.
        let mut normalized_aliases = BTreeMap::new();
        for alias in spec.aliases {
            let normalized = normalize_name(alias);
            if normalized.is_empty() {
                return Err(format!("function alias must not be empty: {alias}"));
            }
            if normalized == canonical {
                return Err(format!("function alias matches canonical name: {alias}"));
            }
            if normalized_aliases.insert(normalized.clone(), ()).is_some() {
                return Err(format!("duplicate function alias: {alias}"));
            }
            if self.aliases.contains_key(&normalized) || self.specs.contains_key(&normalized) {
                return Err(format!("function alias already registered: {alias}"));
            }
        }

        for alias in normalized_aliases.keys() {
            self.aliases.insert(alias.clone(), canonical.clone());
        }
        self.specs.insert(canonical, spec);
        Ok(())
    }

    /// Resolve a canonical name or alias case-insensitively.
    pub fn get(&self, name: &str) -> Option<&FunctionSpec> {
        let normalized = normalize_name(name);
        if let Some(spec) = self.specs.get(&normalized) {
            return Some(spec);
        }
        self.aliases
            .get(&normalized)
            .and_then(|canonical| self.specs.get(canonical))
    }

    /// Iterate over canonical specs in stable name order.
    pub fn iter(&self) -> impl Iterator<Item = &FunctionSpec> {
        self.specs.values()
    }

    /// Return all functions in a category.
    pub fn by_category(&self, category: FunctionCategory) -> Vec<&FunctionSpec> {
        self.specs
            .values()
            .filter(|spec| spec.category == category)
            .collect()
    }

    /// Number of registered canonical functions.
    pub fn len(&self) -> usize {
        self.specs.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }
}

const PERIOD_14: &[ParamSpec] = &[ParamSpec::new("period", "usize", Some("14"), Some("> 0"))];
const PERIOD_REQUIRED: &[ParamSpec] = &[ParamSpec::new("period", "usize", None, Some("> 0"))];
const PERIOD_NBDEV: &[ParamSpec] = &[
    ParamSpec::new("period", "usize", Some("14"), Some("> 0")),
    ParamSpec::new("nb_dev", "f64", Some("1"), Some("finite")),
];
const SMA_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("period", "usize", None, Some("> 0")),
    ParamSpec::new("m", "f64", Some("1"), Some("> 0")),
];
const QUANTILE_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("period", "usize", None, Some("> 0")),
    ParamSpec::new("qscore", "f64", Some("0.5"), Some("0 <= q <= 1")),
];
const MACD_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("fast_period", "usize", Some("12"), Some("> 0")),
    ParamSpec::new("slow_period", "usize", Some("26"), Some("> fast_period")),
    ParamSpec::new("signal_period", "usize", Some("9"), Some("> 0")),
];
const ADOSC_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("fast_period", "usize", Some("3"), Some("> 0")),
    ParamSpec::new("slow_period", "usize", Some("10"), Some("> 0")),
];
/// `ADX(srcHigh, srcLow, srcClose, diLength, adxSmoothing)`.
///
/// The fourth argument is optional: the domestic four-argument form smooths DX
/// with the same length used for the directional movement, while Pine's
/// `ta.dmi(diLength, adxSmoothing)` keeps the two lengths distinct.
const ADX_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("di_length", "usize", Some("14"), Some("> 0")),
    ParamSpec::new("adx_smoothing", "usize", Some("14"), Some("> 0")),
];
const PPO_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("fast_period", "usize", Some("12"), Some("> 0")),
    ParamSpec::new("slow_period", "usize", Some("26"), Some("> fast_period")),
    ParamSpec::new("matype", "usize", Some("0"), Some("0..8")),
];
const DM_PARAMS: &[ParamSpec] = &[ParamSpec::new(
    "timeperiod",
    "usize",
    Some("14"),
    Some("> 1"),
)];
const ULTOSC_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("short_period", "usize", Some("7"), Some("> 0")),
    ParamSpec::new("medium_period", "usize", Some("14"), Some("> 0")),
    ParamSpec::new("long_period", "usize", Some("28"), Some("> 0")),
];
const T3_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("period", "usize", Some("5"), Some("> 0")),
    ParamSpec::new("vfactor", "f64", Some("0.7"), Some("between 0 and 1")),
];
const MAMA_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("fast_limit", "f64", Some("0.5"), Some("between 0 and 1")),
    ParamSpec::new("slow_limit", "f64", Some("0.05"), Some("between 0 and 1")),
];
const MAVP_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("min_period", "usize", Some("2"), Some("> 0")),
    ParamSpec::new("max_period", "usize", Some("30"), Some(">= min_period")),
    ParamSpec::new("matype", "usize", Some("0"), Some("0..8")),
];
const SAREXT_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("start_value", "f64", Some("0"), Some("finite")),
    ParamSpec::new("offset_on_reverse", "f64", Some("0"), Some("finite")),
    ParamSpec::new("af_init_long", "f64", Some("0.02"), Some("finite")),
    ParamSpec::new("af_long", "f64", Some("0.02"), Some("finite")),
    ParamSpec::new("af_max_long", "f64", Some("0.2"), Some("finite")),
    ParamSpec::new("af_init_short", "f64", Some("0.02"), Some("finite")),
    ParamSpec::new("af_short", "f64", Some("0.02"), Some("finite")),
    ParamSpec::new("af_max_short", "f64", Some("0.2"), Some("finite")),
];
const MACDEXT_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("fast_period", "usize", Some("12"), Some("> 0")),
    ParamSpec::new("fast_ma_type", "usize", Some("0"), Some("0..13")),
    ParamSpec::new("slow_period", "usize", Some("26"), Some("> 0")),
    ParamSpec::new("slow_ma_type", "usize", Some("0"), Some("0..13")),
    ParamSpec::new("signal_period", "usize", Some("9"), Some("> 0")),
    ParamSpec::new("signal_ma_type", "usize", Some("0"), Some("0..13")),
];
const MACDFIX_PARAMS: &[ParamSpec] = &[ParamSpec::new(
    "signal_period",
    "usize",
    Some("9"),
    Some("> 0"),
)];
const ACCBANDS_PARAMS: &[ParamSpec] = &[ParamSpec::new("period", "usize", Some("20"), Some("> 1"))];
const VWMA_PARAMS: &[ParamSpec] = &[ParamSpec::new("period", "usize", Some("20"), Some("> 0"))];
const ZSCORE_PARAMS: &[ParamSpec] = &[ParamSpec::new("period", "usize", Some("20"), Some("> 1"))];
const CMF_PARAMS: &[ParamSpec] = &[ParamSpec::new("period", "usize", Some("20"), Some("> 0"))];
const FISHER_PARAMS: &[ParamSpec] = &[ParamSpec::new("period", "usize", Some("10"), Some("> 0"))];
const TSI_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("long_period", "usize", Some("25"), Some("> 0")),
    ParamSpec::new("short_period", "usize", Some("13"), Some("> 0")),
];
const CHOP_PARAMS: &[ParamSpec] = &[ParamSpec::new("period", "usize", Some("14"), Some("> 0"))];
const KDJ_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("period", "usize", Some("9"), Some("> 0")),
    ParamSpec::new("k_smoothing", "usize", Some("3"), Some("> 0")),
    ParamSpec::new("d_smoothing", "usize", Some("3"), Some("> 0")),
];
const SUPERTREND_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("atr_period", "usize", Some("10"), Some("> 0")),
    ParamSpec::new("multiplier", "f64", Some("3.0"), Some("> 0")),
];
const DONCHIAN_PARAMS: &[ParamSpec] = &[ParamSpec::new("period", "usize", Some("20"), Some("> 0"))];
const TENKAN_PARAMS: &[ParamSpec] = &[ParamSpec::new("period", "usize", Some("9"), Some("> 0"))];
const KIJUN_PARAMS: &[ParamSpec] = &[ParamSpec::new("period", "usize", Some("26"), Some("> 0"))];
/// `SAR(high, low, start, increment, max)`.
///
/// The four-argument form omits `increment`, which then defaults to `start` —
/// the TA-Lib shape where one acceleration value serves both roles.
/// `STOCHF(high, low, close, fastK, fastD)`.
///
/// Pine's `ta.stoch` lowers to this with a fast-D period of 1, which is what
/// makes it the *unsmoothed* stochastic rather than the smoothed `STOCH`.
const STOCHF_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("fastk_period", "usize", Some("5"), Some("> 0")),
    ParamSpec::new("fastd_period", "usize", Some("3"), Some("> 0")),
];
const SAR_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("af_start", "f64", Some("0.02"), Some("> 0")),
    ParamSpec::new("af_increment", "f64", Some("0.02"), Some("> 0")),
    ParamSpec::new("af_max", "f64", Some("0.2"), Some(">= af_start")),
];
const BBANDS_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("period", "usize", Some("20"), Some("> 1")),
    ParamSpec::new("nbdevup", "f64", Some("2.0"), Some("finite")),
    ParamSpec::new("nbdevdn", "f64", Some("2.0"), Some("finite")),
    ParamSpec::new("matype", "usize", Some("0"), Some("0..8")),
];
const CROSS_SIGNAL_PARAMS: &[ParamSpec] = &[];
const BREAKOUT_PARAMS: &[ParamSpec] = &[ParamSpec::new("period", "usize", Some("20"), Some("> 0"))];
const VOLUME_SURGE_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("period", "usize", Some("20"), Some("> 0")),
    ParamSpec::new("multiplier", "f64", Some("1.5"), Some(">= 0")),
];
const MA_ALIGN_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("fast_period", "usize", Some("5"), Some("fast < mid < slow")),
    ParamSpec::new("mid_period", "usize", Some("20"), Some("fast < mid < slow")),
    ParamSpec::new(
        "slow_period",
        "usize",
        Some("60"),
        Some("fast < mid < slow"),
    ),
];
const RELATIVE_STRENGTH_PARAMS: &[ParamSpec] =
    &[ParamSpec::new("period", "usize", Some("20"), Some("> 0"))];
const GAP_SIGNAL_PARAMS: &[ParamSpec] = &[ParamSpec::new(
    "threshold",
    "f64",
    Some("0.02"),
    Some(">= 0"),
)];
const TREND_BREAKOUT_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("fast_period", "usize", Some("20"), Some("0 < fast < slow")),
    ParamSpec::new("slow_period", "usize", Some("60"), Some("0 < fast < slow")),
    ParamSpec::new("breakout_period", "usize", Some("20"), Some("> 0")),
    ParamSpec::new("volume_period", "usize", Some("20"), Some("> 0")),
    ParamSpec::new("volume_multiplier", "f64", Some("1.5"), Some(">= 0")),
];
const REF_PARAMS: &[ParamSpec] = &[ParamSpec::new("bars", "usize", None, Some(">= 0"))];
const TWO_SERIES: &[ParamSpec] = &[];
// Host-context functions: these read data the host supplies alongside the price
// series (chip distribution, chart period), so they are declared here to keep
// the plan-path kernel set a subset of the SSOT.
const WINNER_PARAMS: &[ParamSpec] = &[ParamSpec::new("price", "series", None, None)];
const COST_PARAMS: &[ParamSpec] = &[ParamSpec::new(
    "ratio",
    "f64",
    None,
    Some("0 <= ratio <= 100"),
)];
const REFDATE_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("source", "series", None, None),
    ParamSpec::new("date", "f64", None, None),
];

/// Build the stable v0.1.2 public function registry.
pub fn builtin_function_registry() -> FunctionRegistry {
    let mut registry = FunctionRegistry::new();
    let specs = [
        FunctionSpec {
            name: "MA",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "SMA",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: SMA_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "EMA",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "WMA",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "HMA",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            // Hull MA is `WMA(2*WMA(n/2) - WMA(n), sqrt(n))`, so its first finite
            // value sits at `period + round(sqrt(period)) - 2` — not a linear
            // function of the period. `Period`/`PeriodMinusOne` would understate
            // the warm-up and let a caller read NaN as a real value.
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "KAMA",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MACD",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: MACD_PARAMS,
            outputs: 3,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            // The signal leg of MACD, exposed as a standalone domestic function.
            name: "DEA",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: MACD_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "RSI",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: PERIOD_14,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "ROC",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MOM",
            aliases: &["MOMENTUM"],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "AROON_UP",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Hlc,
            params: PERIOD_14,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "AROON_DN",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Hlc,
            params: PERIOD_14,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "PLUS_DI",
            // Not aliased to `PDI`: that name is a thin wrapper in the legacy
            // table, and the alias invariant requires the *identical* function
            // item, not merely an equivalent one.
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Hlc,
            params: PERIOD_14,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MINUS_DI",
            // Same reasoning as `PLUS_DI` / `PDI`.
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Hlc,
            params: PERIOD_14,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "ADX",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Hlc,
            params: ADX_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "WILLR",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Hlc,
            params: PERIOD_14,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "STOCHF",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Hlc,
            params: STOCHF_PARAMS,
            outputs: 2,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "SAR",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Dynamic,
            params: SAR_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "CCI",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Dynamic,
            params: PERIOD_14,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "TRIX",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: PERIOD_14,
            // Warm-up is `3 * (period - 1)`: each of the three chained EMAs
            // consumes its own SMA seed before the rate of change is defined.
            lookback: LookbackSpec::Dynamic,
            outputs: 1,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "TRIMA",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "ATR",
            aliases: &[],
            category: FunctionCategory::Volatility,
            input: InputKind::Hlc,
            params: PERIOD_14,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "NATR",
            aliases: &[],
            category: FunctionCategory::Volatility,
            input: InputKind::Hlc,
            params: PERIOD_14,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        // True Range itself, without the Wilder smoothing `ATR` applies. It takes
        // no period, so the only lookback it needs is the previous close — which
        // is also why bar 0 is NaN rather than `high[0] - low[0]`.
        FunctionSpec {
            name: "TRANGE",
            aliases: &[],
            category: FunctionCategory::Volatility,
            input: InputKind::Hlc,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "BBANDS",
            aliases: &["BOLL", "BOLLINGER"],
            category: FunctionCategory::Volatility,
            input: InputKind::Series,
            params: BBANDS_PARAMS,
            outputs: 3,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        // The single-band projections of `BBANDS`. Each is one leg of the same
        // `overlap::bbands` call the tree path makes, not a separate formula.
        FunctionSpec {
            name: "BOLLUP",
            aliases: &[],
            category: FunctionCategory::Volatility,
            input: InputKind::Series,
            params: BBANDS_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "BOLLMID",
            aliases: &[],
            category: FunctionCategory::Volatility,
            input: InputKind::Series,
            params: PERIOD_14,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "BOLLDN",
            aliases: &[],
            category: FunctionCategory::Volatility,
            input: InputKind::Series,
            params: BBANDS_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "STD",
            aliases: &[],
            category: FunctionCategory::Statistics,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "OBV",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "AD",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Hlcv,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "ADOSC",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Hlcv,
            params: ADOSC_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MFI",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Hlcv,
            params: PERIOD_14,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "ZSCORE",
            aliases: &["Z_SCORE"],
            category: FunctionCategory::Statistics,
            input: InputKind::Series,
            params: ZSCORE_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "VWMA",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Dynamic,
            params: VWMA_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "CMF",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Hlcv,
            params: CMF_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "FISHER",
            aliases: &["FISHER_TRANSFORM"],
            category: FunctionCategory::Momentum,
            input: InputKind::Hlc,
            params: FISHER_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "FISHER_SIGNAL",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Hlc,
            params: FISHER_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "TSI",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: TSI_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "CHOP",
            aliases: &["CHOPPINESS"],
            category: FunctionCategory::Volatility,
            input: InputKind::Hlc,
            params: CHOP_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "KDJ",
            aliases: &["KD", "KDJ_K"],
            category: FunctionCategory::Momentum,
            input: InputKind::Hlc,
            params: KDJ_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "KDJ_D",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Hlc,
            params: KDJ_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "KDJ_J",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Hlc,
            params: KDJ_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "ICHIMOKU_TENKAN",
            aliases: &["TENKAN"],
            category: FunctionCategory::Overlap,
            input: InputKind::Hlc,
            params: TENKAN_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "ICHIMOKU_KIJUN",
            aliases: &["KIJUN", "KIJUN_SEN"],
            category: FunctionCategory::Overlap,
            input: InputKind::Hlc,
            params: KIJUN_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "SUPERTREND",
            aliases: &["SUPERTREND_LINE"],
            category: FunctionCategory::Overlap,
            input: InputKind::Hlc,
            params: SUPERTREND_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "VWAP",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "DONCHIAN",
            aliases: &[],
            category: FunctionCategory::Volatility,
            input: InputKind::Hlc,
            params: DONCHIAN_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "DONCHIAN_UPPER",
            aliases: &[],
            category: FunctionCategory::Volatility,
            input: InputKind::Hlc,
            params: DONCHIAN_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "DONCHIAN_LOWER",
            aliases: &[],
            category: FunctionCategory::Volatility,
            input: InputKind::Hlc,
            params: DONCHIAN_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "DONCHIAN_MIDDLE",
            aliases: &["DONCHIAN_MID"],
            category: FunctionCategory::Volatility,
            input: InputKind::Hlc,
            params: DONCHIAN_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "DONCHIAN_WIDTH",
            aliases: &[],
            category: FunctionCategory::Volatility,
            input: InputKind::Hlc,
            params: DONCHIAN_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "REF",
            aliases: &["SHIFT"],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: REF_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        // `MINUS(x, n)` is `x[i] - x[i - n]`, so unlike `REF` its warm-up is
        // exactly the period: the first `n` bars have no predecessor to
        // subtract and stay NaN.
        FunctionSpec {
            name: "MINUS",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "HHV",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "LLV",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        // `HHVBARS`/`LLVBARS` report how many bars back the window extreme sits.
        // Unlike the extreme itself they need no warm-up: the saturating window
        // start makes bar 0 well defined, so the lookback is `None`, not
        // `PeriodMinusOne`.
        FunctionSpec {
            name: "HHVBARS",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "LLVBARS",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "COUNT",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "BARSLAST",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        // `BARSLAST` counted *forward* from the series start instead of backward
        // from the current bar. Same windowless shape, same dynamic lookback.
        FunctionSpec {
            name: "BARSSINCE",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        // `SUMBARS(X, T)` scans backwards from each bar until the running total
        // of `X` reaches that bar's `T`, so its reach is unbounded — hence
        // `Dynamic`, not `PeriodMinusOne`, even though it takes two operands.
        FunctionSpec {
            name: "SUMBARS",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: TWO_SERIES,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: false,
            deterministic: true,
        },
        FunctionSpec {
            name: "CROSS",
            aliases: &["CROSSOVER"],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: TWO_SERIES,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        // The downward mirror of `CROSS`. Like it, it has no fixed lookback:
        // the crossing is a per-bar predicate, not a window statistic.
        FunctionSpec {
            name: "CROSSBELOW",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: TWO_SERIES,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "GOLDEN_CROSS",
            aliases: &["CROSSUP", "BULLISH_CROSS"],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: CROSS_SIGNAL_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "DEAD_CROSS",
            aliases: &["CROSSDOWN", "BEARISH_CROSS"],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: CROSS_SIGNAL_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "BREAKOUT",
            aliases: &["BREAKOUT_UP", "PRICE_BREAKOUT"],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: BREAKOUT_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "BREAKDOWN",
            aliases: &["BREAKOUT_DOWN", "PRICE_BREAKDOWN"],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: BREAKOUT_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "VOLUME_SURGE",
            aliases: &["VOLSURGE", "VOLUME_EXPANSION"],
            category: FunctionCategory::Formula,
            input: InputKind::Series,
            params: VOLUME_SURGE_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MA_ALIGN",
            aliases: &["MA_ALIGNMENT", "TREND_ALIGN"],
            category: FunctionCategory::Formula,
            input: InputKind::Series,
            params: MA_ALIGN_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "RELATIVE_STRENGTH",
            aliases: &["RELSTRENGTH", "RS_EXCESS_RETURN"],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: RELATIVE_STRENGTH_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "GAP_SIGNAL",
            aliases: &["GAP"],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: GAP_SIGNAL_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "TREND_BREAKOUT",
            aliases: &["TREND_SCREEN", "BREAKOUT_SCREEN"],
            category: FunctionCategory::Formula,
            input: InputKind::Hlcv,
            params: TREND_BREAKOUT_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "IF",
            aliases: &["IFF"],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        // --- Host-context functions -------------------------------------------
        // Deterministic given (inputs, host data), but not streamable: the host
        // supplies the chip distribution / chart period, so there is no
        // bar-by-bar state to carry forward.
        FunctionSpec {
            name: "WINNER",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: WINNER_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: false,
            deterministic: true,
        },
        FunctionSpec {
            name: "COST",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: COST_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: false,
            deterministic: true,
        },
        FunctionSpec {
            name: "PERIODTYPE",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: false,
            deterministic: true,
        },
        // The zero-argument host-data readers. None of them takes an operand
        // because the series each reads is supplied out of band through
        // `HostContext`, so `InputKind::Dynamic` describes them honestly: there
        // is no numeric signature to advertise.
        //
        // Declaring them here is not cosmetic. The SSOT gate requires every plan
        // kernel to be registered, and registration is also what makes the
        // planner treat them as pure -- unregistered, each would lower to a
        // stateful barrier with a phantom tail dependency and could never be
        // shared or reordered.
        //
        // `TR` reads the implicit OHLC and so needs one warm-up row (it uses the
        // previous close). It has no period operand to derive that from, which is
        // why its lookback is `Dynamic` rather than `PeriodMinusOne`.
        FunctionSpec {
            name: "TR",
            aliases: &[],
            category: FunctionCategory::Volatility,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        // The DZH money-flow family. Each is a straight read of one host series
        // with no warm-up at all, so `LookbackSpec::None` is exact rather than
        // conservative.
        FunctionSpec {
            name: "MONEYFLOW",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MAININFLOW",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MAININFLOWPCT",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "BIGORDER",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "SMALLORDER",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "SUPERBIGORDER",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        // `NETINFLOW` is the only member of the family taking an operand -- an
        // optional level selecting the order-size bucket -- so its arity is
        // 0-or-1 rather than strictly zero.
        FunctionSpec {
            name: "NETINFLOW",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "REFDATE",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: REFDATE_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: false,
            deterministic: true,
        },
        // --- Rate-of-change ratio variants ------------------------------------
        // Implemented and delegated to the canonical momentum functions long
        // before they were registered here; declaring them keeps the plan-path
        // kernel set a subset of the SSOT.
        FunctionSpec {
            name: "ROCP",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "ROCR",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "ROCR100",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
    ];

    let additional_specs = [
        FunctionSpec {
            name: "DEMA",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "TEMA",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "T3",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: T3_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MAMA",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: MAMA_PARAMS,
            outputs: 2,
            lookback: LookbackSpec::Dynamic,
            streaming: false,
            deterministic: true,
        },
        FunctionSpec {
            name: "MAVP",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Dynamic,
            params: MAVP_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: false,
            deterministic: true,
        },
        FunctionSpec {
            name: "SAREXT",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Dynamic,
            params: SAREXT_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "CMO",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: PERIOD_14,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MACDEXT",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: MACDEXT_PARAMS,
            outputs: 3,
            lookback: LookbackSpec::Dynamic,
            streaming: false,
            deterministic: true,
        },
        FunctionSpec {
            name: "MACDFIX",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: MACDFIX_PARAMS,
            outputs: 3,
            lookback: LookbackSpec::Dynamic,
            streaming: false,
            deterministic: true,
        },
        FunctionSpec {
            name: "ACCBANDS",
            aliases: &[],
            category: FunctionCategory::Volatility,
            input: InputKind::Hlc,
            params: ACCBANDS_PARAMS,
            outputs: 3,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: false,
            deterministic: true,
        },
        FunctionSpec {
            name: "AVGDEV",
            aliases: &[],
            category: FunctionCategory::Statistics,
            input: InputKind::Series,
            params: PERIOD_14,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "IMI",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Dynamic,
            params: PERIOD_14,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "HT_DCPERIOD",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: false,
            deterministic: true,
        },
        FunctionSpec {
            name: "HT_DCPHASE",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: false,
            deterministic: true,
        },
        FunctionSpec {
            name: "HT_PHASOR",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: &[],
            outputs: 2,
            lookback: LookbackSpec::Dynamic,
            streaming: false,
            deterministic: true,
        },
        FunctionSpec {
            name: "HT_SINE",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: &[],
            outputs: 2,
            lookback: LookbackSpec::Dynamic,
            streaming: false,
            deterministic: true,
        },
        FunctionSpec {
            name: "HT_TRENDMODE",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: false,
            deterministic: true,
        },
        FunctionSpec {
            name: "HT_TRENDLINE",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: false,
            deterministic: true,
        },
        FunctionSpec {
            name: "STDDEV",
            aliases: &[],
            category: FunctionCategory::Statistics,
            input: InputKind::Series,
            params: PERIOD_NBDEV,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "VAR",
            aliases: &[],
            category: FunctionCategory::Statistics,
            input: InputKind::Series,
            params: PERIOD_NBDEV,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "CORREL",
            aliases: &[],
            category: FunctionCategory::Statistics,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "BETA",
            aliases: &[],
            category: FunctionCategory::Statistics,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "LINEARREG",
            aliases: &[],
            category: FunctionCategory::Statistics,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "LINEARREG_ANGLE",
            aliases: &[],
            category: FunctionCategory::Statistics,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "LINEARREG_INTERCEPT",
            aliases: &[],
            category: FunctionCategory::Statistics,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "LINEARREG_SLOPE",
            aliases: &[],
            category: FunctionCategory::Statistics,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "TSF",
            aliases: &[],
            category: FunctionCategory::Statistics,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "PPO",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: PPO_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "ULTOSC",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Hlc,
            params: ULTOSC_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "PLUS_DM",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Hlc,
            params: DM_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MINUS_DM",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Hlc,
            params: DM_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "ADD",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "SUB",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MULT",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "DIV",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MATH_AVG",
            aliases: &["AVG"],
            category: FunctionCategory::Formula,
            // Variadic: Pine `math.avg(a, b, ...)` averages every argument.
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "ABS",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Series,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        // `sign(x)` is `-1`, `0` or `1`, deliberately *not* `f64::signum`
        // (which maps both zeros to `1`/`-1`). The WorldQuant alphas use it to
        // re-attach the direction of a differenced series, so `sign(0) == 0`
        // matters: a flat window must not acquire a direction.
        FunctionSpec {
            name: "SIGN",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Series,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "SUM",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "RMA",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "CUMSUM",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Series,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MEDIAN",
            aliases: &[],
            category: FunctionCategory::Statistics,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: false,
            deterministic: true,
        },
        FunctionSpec {
            name: "FIXNAN",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Series,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        // The predicate half of the NaN toolkit: `ISNA` reports *where* a gap is,
        // where `FIXNAN` erases it. Both back Pine's `nz`/`na`, which is why
        // neither takes a period and neither can be folded into the other.
        FunctionSpec {
            name: "ISNA",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Series,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "ROLLING_RANGE",
            aliases: &[],
            category: FunctionCategory::Statistics,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: false,
            deterministic: true,
        },
        // The DZH integer/fraction split. Neither is a rounding mode on the
        // other: `INTPART(-1.5)` is `-1.0` (truncation toward zero, not
        // `floor`), and `FRACPART(-1.5)` is `-0.5` because `f64::fract` keeps
        // the sign. They are also the two halves of the `INTPART(X) +
        // FRACPART(X) == X` identity that the DZH corpus checks.
        FunctionSpec {
            name: "INTPART",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Series,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "FRACPART",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Series,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        // `MOD` is registered as a *function* separately from the `%` operator
        // (`BINARY:Mod`) because the two disagree: the function uses Rust's
        // truncating remainder and returns NaN for a near-zero divisor, while the
        // operator uses a floor-based remainder. For negative operands they give
        // different answers, so one cannot stand in for the other.
        FunctionSpec {
            name: "MOD",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: TWO_SERIES,
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        // `REVERSE` mirrors the series end to end, so it is a whole-series
        // transform rather than a window: no warm-up, but its lookback is the
        // full length, which `Dynamic` expresses.
        FunctionSpec {
            name: "REVERSE",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: false,
            deterministic: true,
        },
        FunctionSpec {
            name: "MAX",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MIN",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MAXINDEX",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MININDEX",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MINMAX",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 2,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MINMAXINDEX",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 2,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
    ];

    let math_transform_specs = [
        "ACOS", "ASIN", "ATAN", "CEIL", "COS", "COSH", "EXP", "FLOOR", "LN", "LOG10", "SIN",
        "SINH", "SQRT", "TAN", "TANH",
    ]
    .into_iter()
    .map(|name| FunctionSpec {
        name,
        aliases: &[],
        category: FunctionCategory::Formula,
        input: InputKind::Series,
        params: &[],
        outputs: 1,
        lookback: LookbackSpec::None,
        streaming: true,
        deterministic: true,
    });

    let candlestick_specs = [
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
        "CDLEVENINGDOJISTAR",
        "CDLEVENINGSTAR",
        "CDLGAPSIDESIDEWHITE",
        "CDLGRAVESTONEDOJI",
        "CDLENGULFING",
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
    ]
    .into_iter()
    .map(|name| FunctionSpec {
        name,
        aliases: &[],
        category: FunctionCategory::Formula,
        input: InputKind::Dynamic,
        params: &[],
        outputs: 1,
        lookback: LookbackSpec::Dynamic,
        streaming: false,
        deterministic: true,
    });

    // Reference-parity primitives introduced for the Alpha158 factor library.
    //
    // Each one exists because Qlib's Alpha158 needs an operator that no
    // domestic/TALib spelling covers, and each delegates to an existing
    // `math::` kernel rather than re-deriving it:
    //
    // * `QUANTILE`  -> `math::quantile::quantile` (linear interpolation, which is
    //   pandas' default and therefore Qlib's `Quantile`);
    // * `RSQUARE`   -> `math::regression::simple_ols`'s `r_squared`;
    // * `RESI`      -> the same fit's last residual;
    // * `RANK_PCT`  -> `math::rank::ranks` with `TiePolicy::Average`, divided by
    //   the window length, i.e. pandas' `rolling.rank(pct=True)`;
    // * `STDDEV_SAMPLE` -> `math::rolling_stats::stddev_sample_into`, the same
    //   moment scan as `STDDEV` with pandas' `ddof = 1` denominator. It is a
    //   separate *name* rather than a change to `STDDEV` because TA-Lib's
    //   `STDDEV` is the population convention and the two differ by the exact
    //   factor `sqrt((n - 1) / n)` — a systematic ~10% scale error at `n = 5`,
    //   which is precisely why Alpha158 needs both spellings.
    //
    // `LINEARREG_SLOPE` is deliberately *not* here: it already had an SSOT entry
    // from the TA-Lib track, so only its plan kernel was missing.
    let alpha158_parity_specs = [
        FunctionSpec {
            name: "QUANTILE",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Series,
            params: QUANTILE_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "RSQUARE",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "RESI",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "RANK_PCT",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "STDDEV_SAMPLE",
            aliases: &[],
            category: FunctionCategory::Statistics,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
    ];

    for spec in specs
        .into_iter()
        .chain(additional_specs)
        .chain(math_transform_specs)
        .chain(candlestick_specs)
        .chain(alpha158_parity_specs)
    {
        registry
            .register(spec)
            .expect("built-in function names and aliases are unique");
    }
    registry
}

fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ma_and_sma_have_distinct_canonical_contracts() {
        let registry = builtin_function_registry();
        let ma = registry.get("MA").unwrap();
        let sma = registry.get("SMA").unwrap();
        assert_eq!(ma.name, "MA");
        assert_eq!(sma.name, "SMA");
        assert_eq!(ma.lookback, LookbackSpec::PeriodMinusOne);
        assert_eq!(sma.lookback, LookbackSpec::Dynamic);
        assert_eq!(ma.params.len(), 1);
        assert_eq!(sma.params.len(), 2);
    }

    #[test]
    fn aliases_resolve_case_insensitively() {
        let registry = builtin_function_registry();
        assert_eq!(registry.get("boll").unwrap().name, "BBANDS");
        assert_eq!(registry.get("shift").unwrap().name, "REF");
    }

    #[test]
    fn duplicate_aliases_are_rejected() {
        let mut registry = FunctionRegistry::new();
        registry
            .register(FunctionSpec {
                name: "ONE",
                aliases: &["ALIAS"],
                category: FunctionCategory::Formula,
                input: InputKind::Dynamic,
                params: &[],
                outputs: 1,
                lookback: LookbackSpec::None,
                streaming: true,
                deterministic: true,
            })
            .unwrap();
        let error = registry
            .register(FunctionSpec {
                name: "TWO",
                aliases: &["alias"],
                category: FunctionCategory::Formula,
                input: InputKind::Dynamic,
                params: &[],
                outputs: 1,
                lookback: LookbackSpec::None,
                streaming: true,
                deterministic: true,
            })
            .unwrap_err();
        assert!(error.contains("alias"));
    }

    #[test]
    fn canonical_name_cannot_shadow_an_existing_alias() {
        let mut registry = FunctionRegistry::new();
        registry
            .register(FunctionSpec {
                name: "ONE",
                aliases: &["ALIAS"],
                category: FunctionCategory::Formula,
                input: InputKind::Dynamic,
                params: &[],
                outputs: 1,
                lookback: LookbackSpec::None,
                streaming: true,
                deterministic: true,
            })
            .unwrap();
        let error = registry
            .register(FunctionSpec {
                name: "alias",
                aliases: &[],
                category: FunctionCategory::Formula,
                input: InputKind::Dynamic,
                params: &[],
                outputs: 1,
                lookback: LookbackSpec::None,
                streaming: true,
                deterministic: true,
            })
            .unwrap_err();
        assert!(error.contains("conflicts with alias"));
    }

    #[test]
    fn aliases_must_be_unique_within_one_spec() {
        let mut registry = FunctionRegistry::new();
        let error = registry
            .register(FunctionSpec {
                name: "ONE",
                aliases: &["ALIAS", "alias"],
                category: FunctionCategory::Formula,
                input: InputKind::Dynamic,
                params: &[],
                outputs: 1,
                lookback: LookbackSpec::None,
                streaming: true,
                deterministic: true,
            })
            .unwrap_err();
        assert!(error.contains("duplicate function alias"));
    }

    #[test]
    fn core_formula_primitives_are_discoverable() {
        let registry = builtin_function_registry();
        for name in ["REF", "HHV", "LLV", "COUNT", "BARSLAST", "CROSS", "IF"] {
            assert!(registry.get(name).is_some(), "missing metadata for {name}");
        }
    }

    #[test]
    fn pine_formula_runtime_extensions_are_discoverable() {
        let registry = builtin_function_registry();
        for name in ["RMA", "CUMSUM", "MEDIAN", "FIXNAN", "ROLLING_RANGE"] {
            let spec = registry
                .get(name)
                .unwrap_or_else(|| panic!("missing metadata for {name}"));
            assert!(spec.deterministic, "{name} must be deterministic");
        }
        assert_eq!(
            registry.get("RMA").unwrap().lookback,
            LookbackSpec::PeriodMinusOne
        );
        assert_eq!(registry.get("FIXNAN").unwrap().params.len(), 0);
    }

    #[test]
    fn modern_indicator_formulas_are_discoverable_and_pure() {
        let registry = builtin_function_registry();
        for name in [
            "ZSCORE",
            "VWMA",
            "CMF",
            "FISHER",
            "FISHER_SIGNAL",
            "TSI",
            "CHOP",
            "KDJ",
            "KDJ_D",
            "KDJ_J",
            "ICHIMOKU_TENKAN",
            "ICHIMOKU_KIJUN",
            "SUPERTREND",
            "VWAP",
            "DONCHIAN",
            "DONCHIAN_UPPER",
            "DONCHIAN_LOWER",
            "DONCHIAN_MIDDLE",
            "DONCHIAN_WIDTH",
            "GOLDEN_CROSS",
            "DEAD_CROSS",
            "BREAKOUT",
            "BREAKDOWN",
            "VOLUME_SURGE",
            "MA_ALIGN",
            "RELATIVE_STRENGTH",
            "GAP_SIGNAL",
            "TREND_BREAKOUT",
        ] {
            let spec = registry
                .get(name)
                .unwrap_or_else(|| panic!("missing metadata for {name}"));
            assert!(spec.deterministic, "{name} must be deterministic");
            assert!(spec.streaming, "{name} must be streamable");
        }
        assert_eq!(registry.get("z_score").unwrap().name, "ZSCORE");
        assert_eq!(registry.get("kijun_sen").unwrap().name, "ICHIMOKU_KIJUN");
        assert_eq!(
            registry.get("donchian_mid").unwrap().name,
            "DONCHIAN_MIDDLE"
        );
        assert_eq!(registry.get("golden_cross").unwrap().name, "GOLDEN_CROSS");
        assert_eq!(registry.get("volsurge").unwrap().name, "VOLUME_SURGE");
        assert_eq!(registry.get("trend_screen").unwrap().name, "TREND_BREAKOUT");
    }
}
