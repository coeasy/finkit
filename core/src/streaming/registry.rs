//! Static indicator metadata registry for discovery and JSON export.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Lazily-built, process-wide cache of the registered indicator metadata.
static INDICATOR_CACHE: OnceLock<Vec<IndicatorInfo>> = OnceLock::new();

/// Lazily-built `name -> index into INDICATOR_CACHE` lookup. Avoids repeated
/// linear scans from CLI / FFI code that resolves indicator names by string.
static INDICATOR_BY_ID: OnceLock<HashMap<&'static str, usize>> = OnceLock::new();

/// Lazily-built `category -> Vec<index into INDICATOR_CACHE>`. Group-by is O(n)
/// once; subsequent `by_category("momentum")` calls look up the index list in
/// O(1) and then build the result `Vec` of `&'a IndicatorInfo` (the `&'a` is
/// tied to the input lifetime so callers don't need `'static`).
static INDICATOR_BY_CATEGORY: OnceLock<HashMap<&'static str, Vec<usize>>> = OnceLock::new();

/// Machine-readable metadata for a single indicator.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct IndicatorInfo {
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub params: &'static [ParamInfo],
    pub convergence: usize,
    pub streaming: bool,
}

/// Parameter descriptor for an indicator.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ParamInfo {
    pub name: &'static str,
    pub param_type: &'static str,
    pub default: &'static str,
    pub description: &'static str,
}

/// Full registry document for JSON export.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct RegistryDocument {
    pub version: &'static str,
    pub generated_at: Option<&'static str>,
    pub indicators: &'static [IndicatorInfo],
}

/// Valid indicator category slugs.
pub const VALID_CATEGORIES: &[&str] = &[
    "overlap",
    "momentum",
    "volume",
    "volatility",
    "price_transform",
    "cycle",
    "statistics",
    "breadth",
    "sentiment",
    "math_transform",
    "math_operators",
    "fibonacci",
    "pattern",
    "astock",
];

const PERIOD: ParamInfo = ParamInfo {
    name: "period",
    param_type: "usize",
    default: "14",
    description: "Lookback period",
};

const PERIOD_20: ParamInfo = ParamInfo {
    name: "period",
    param_type: "usize",
    default: "20",
    description: "Lookback period",
};

const FAST_SLOW: [ParamInfo; 2] = [
    ParamInfo {
        name: "fast_period",
        param_type: "usize",
        default: "12",
        description: "Fast EMA period",
    },
    ParamInfo {
        name: "slow_period",
        param_type: "usize",
        default: "26",
        description: "Slow EMA period",
    },
];

static INDICATORS: &[IndicatorInfo] = &[
    // ── Overlap ──────────────────────────────────────────────
    IndicatorInfo {
        name: "SMA",
        category: "overlap",
        description: "Simple Moving Average",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "EMA",
        category: "overlap",
        description: "Exponential Moving Average",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "WMA",
        category: "overlap",
        description: "Weighted Moving Average",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "DEMA",
        category: "overlap",
        description: "Double Exponential Moving Average",
        params: &[PERIOD],
        convergence: 28,
        streaming: true,
    },
    IndicatorInfo {
        name: "TEMA",
        category: "overlap",
        description: "Triple Exponential Moving Average",
        params: &[PERIOD],
        convergence: 42,
        streaming: true,
    },
    IndicatorInfo {
        name: "KAMA",
        category: "overlap",
        description: "Kaufman Adaptive Moving Average",
        params: &[
            ParamInfo {
                name: "period",
                param_type: "usize",
                default: "10",
                description: "Efficiency ratio lookback",
            },
            ParamInfo {
                name: "fast",
                param_type: "usize",
                default: "2",
                description: "Fast smoothing constant",
            },
            ParamInfo {
                name: "slow",
                param_type: "usize",
                default: "30",
                description: "Slow smoothing constant",
            },
        ],
        convergence: 30,
        streaming: true,
    },
    IndicatorInfo {
        name: "T3",
        category: "overlap",
        description: "Triple Exponential Moving Average (T3)",
        params: &[
            ParamInfo {
                name: "period",
                param_type: "usize",
                default: "5",
                description: "Lookback period",
            },
            ParamInfo {
                name: "v_factor",
                param_type: "f64",
                default: "0.7",
                description: "Volume factor",
            },
        ],
        convergence: 15,
        streaming: true,
    },
    IndicatorInfo {
        name: "HMA",
        category: "overlap",
        description: "Hull Moving Average",
        params: &[PERIOD],
        convergence: 20,
        streaming: true,
    },
    IndicatorInfo {
        name: "ALMA",
        category: "overlap",
        description: "Arnaud Legoux Moving Average",
        params: &[
            ParamInfo {
                name: "period",
                param_type: "usize",
                default: "9",
                description: "Lookback period",
            },
            ParamInfo {
                name: "sigma",
                param_type: "f64",
                default: "6.0",
                description: "Sigma (Gaussian width)",
            },
            ParamInfo {
                name: "offset",
                param_type: "f64",
                default: "0.85",
                description: "Offset (0-1)",
            },
        ],
        convergence: 9,
        streaming: true,
    },
    IndicatorInfo {
        name: "Bollinger Bands",
        category: "overlap",
        description: "Bollinger Bands (upper, middle, lower)",
        params: &[
            PERIOD_20,
            ParamInfo {
                name: "stddev",
                param_type: "f64",
                default: "2.0",
                description: "Standard deviation multiplier",
            },
        ],
        convergence: 20,
        streaming: true,
    },
    IndicatorInfo {
        name: "SAR",
        category: "overlap",
        description: "Parabolic Stop and Reverse",
        params: &[
            ParamInfo {
                name: "acceleration",
                param_type: "f64",
                default: "0.02",
                description: "Acceleration factor step",
            },
            ParamInfo {
                name: "maximum",
                param_type: "f64",
                default: "0.2",
                description: "Maximum acceleration factor",
            },
        ],
        convergence: 2,
        streaming: true,
    },
    IndicatorInfo {
        name: "MAMA",
        category: "overlap",
        description: "MESA Adaptive Moving Average",
        params: &[
            ParamInfo {
                name: "fast_limit",
                param_type: "f64",
                default: "0.5",
                description: "Fast limit",
            },
            ParamInfo {
                name: "slow_limit",
                param_type: "f64",
                default: "0.05",
                description: "Slow limit",
            },
        ],
        convergence: 32,
        streaming: false,
    },
    IndicatorInfo {
        name: "MIDPOINT",
        category: "overlap",
        description: "MidPoint over period",
        params: &[PERIOD],
        convergence: 14,
        streaming: false,
    },
    IndicatorInfo {
        name: "MIDPRICE",
        category: "overlap",
        description: "Midpoint Price over period",
        params: &[PERIOD],
        convergence: 14,
        streaming: false,
    },
    IndicatorInfo {
        name: "TRIMA",
        category: "overlap",
        description: "Triangular Moving Average (double-smoothed SMA)",
        params: &[PERIOD],
        convergence: 21,
        streaming: true,
    },
    IndicatorInfo {
        name: "MAVP",
        category: "overlap",
        description: "Moving Average with Variable Period (dynamic per-bar period)",
        params: &[
            ParamInfo {
                name: "min_period",
                param_type: "usize",
                default: "2",
                description: "Minimum allowed period",
            },
            ParamInfo {
                name: "max_period",
                param_type: "usize",
                default: "30",
                description: "Maximum allowed period",
            },
            ParamInfo {
                name: "ma_type",
                param_type: "usize",
                default: "0",
                description: "MA type (0=SMA, 1=EMA)",
            },
        ],
        convergence: 30,
        streaming: false,
    },
    IndicatorInfo {
        name: "SAREXT",
        category: "overlap",
        description: "Parabolic SAR - Extended (separate long/short acceleration)",
        params: &[
            ParamInfo {
                name: "start_value",
                param_type: "f64",
                default: "0.0",
                description: "Starting SAR value",
            },
            ParamInfo {
                name: "offset_on_reverse",
                param_type: "f64",
                default: "0.0",
                description: "Offset on reverse",
            },
            ParamInfo {
                name: "af_init_long",
                param_type: "f64",
                default: "0.02",
                description: "Initial acceleration long",
            },
            ParamInfo {
                name: "af_long",
                param_type: "f64",
                default: "0.02",
                description: "Acceleration step long",
            },
            ParamInfo {
                name: "af_max_long",
                param_type: "f64",
                default: "0.2",
                description: "Max acceleration long",
            },
            ParamInfo {
                name: "af_init_short",
                param_type: "f64",
                default: "0.02",
                description: "Initial acceleration short",
            },
            ParamInfo {
                name: "af_short",
                param_type: "f64",
                default: "0.02",
                description: "Acceleration step short",
            },
            ParamInfo {
                name: "af_max_short",
                param_type: "f64",
                default: "0.2",
                description: "Max acceleration short",
            },
        ],
        convergence: 2,
        streaming: false,
    },
    IndicatorInfo {
        name: "Donchian Channel",
        category: "overlap",
        description: "Donchian Channel (upper, middle, lower)",
        params: &[PERIOD_20],
        convergence: 20,
        streaming: true,
    },
    IndicatorInfo {
        name: "SuperTrend",
        category: "overlap",
        description: "SuperTrend trend-following overlay",
        params: &[
            ParamInfo {
                name: "period",
                param_type: "usize",
                default: "10",
                description: "ATR lookback period",
            },
            ParamInfo {
                name: "multiplier",
                param_type: "f64",
                default: "3.0",
                description: "ATR multiplier",
            },
        ],
        convergence: 11,
        streaming: true,
    },
    IndicatorInfo {
        name: "Ichimoku Cloud",
        category: "overlap",
        description: "Ichimoku Kinko Hyo (conversion, base, spans, lagging)",
        params: &[
            ParamInfo {
                name: "tenkan",
                param_type: "usize",
                default: "9",
                description: "Conversion line period",
            },
            ParamInfo {
                name: "kijun",
                param_type: "usize",
                default: "26",
                description: "Base line period",
            },
            ParamInfo {
                name: "senkou_b",
                param_type: "usize",
                default: "52",
                description: "Leading span B period",
            },
        ],
        convergence: 52,
        streaming: true,
    },
    // ── Momentum ─────────────────────────────────────────────
    IndicatorInfo {
        name: "RSI",
        category: "momentum",
        description: "Relative Strength Index",
        params: &[PERIOD],
        convergence: 15,
        streaming: true,
    },
    IndicatorInfo {
        name: "MACD",
        category: "momentum",
        description: "Moving Average Convergence/Divergence",
        params: &[
            ParamInfo {
                name: "fast_period",
                param_type: "usize",
                default: "12",
                description: "Fast EMA period",
            },
            ParamInfo {
                name: "slow_period",
                param_type: "usize",
                default: "26",
                description: "Slow EMA period",
            },
            ParamInfo {
                name: "signal_period",
                param_type: "usize",
                default: "9",
                description: "Signal line EMA period",
            },
        ],
        convergence: 35,
        streaming: true,
    },
    IndicatorInfo {
        name: "Stochastic",
        category: "momentum",
        description: "Stochastic Oscillator (%K / %D)",
        params: &[
            ParamInfo {
                name: "k_period",
                param_type: "usize",
                default: "14",
                description: "%K lookback period",
            },
            ParamInfo {
                name: "d_period",
                param_type: "usize",
                default: "3",
                description: "%D smoothing period",
            },
        ],
        convergence: 16,
        streaming: true,
    },
    IndicatorInfo {
        name: "KDJ",
        category: "momentum",
        description: "Chinese Stochastic Oscillator (K/D/J)",
        params: &[
            ParamInfo {
                name: "n",
                param_type: "usize",
                default: "9",
                description: "RSV lookback period",
            },
            ParamInfo {
                name: "m1",
                param_type: "usize",
                default: "3",
                description: "K smoothing period",
            },
            ParamInfo {
                name: "m2",
                param_type: "usize",
                default: "3",
                description: "D smoothing period",
            },
        ],
        convergence: 9,
        streaming: true,
    },
    IndicatorInfo {
        name: "BIAS",
        category: "momentum",
        description: "Deviation Rate (乖离率)",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "Williams %R",
        category: "momentum",
        description: "Williams Percent Range",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "CCI",
        category: "momentum",
        description: "Commodity Channel Index",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "ADX",
        category: "momentum",
        description: "Average Directional Index",
        params: &[PERIOD],
        convergence: 28,
        streaming: true,
    },
    IndicatorInfo {
        name: "ADXR",
        category: "momentum",
        description: "Average Directional Movement Index Rating",
        params: &[PERIOD],
        convergence: 42,
        streaming: true,
    },
    IndicatorInfo {
        name: "MFI",
        category: "momentum",
        description: "Money Flow Index",
        params: &[PERIOD],
        convergence: 15,
        streaming: true,
    },
    IndicatorInfo {
        name: "MOM",
        category: "momentum",
        description: "Momentum",
        params: &[ParamInfo {
            name: "period",
            param_type: "usize",
            default: "10",
            description: "Lookback period",
        }],
        convergence: 11,
        streaming: true,
    },
    IndicatorInfo {
        name: "ROC",
        category: "momentum",
        description: "Rate of Change",
        params: &[ParamInfo {
            name: "period",
            param_type: "usize",
            default: "10",
            description: "Lookback period",
        }],
        convergence: 11,
        streaming: true,
    },
    IndicatorInfo {
        name: "ROCP",
        category: "momentum",
        description: "Rate of Change Percentage (decimal)",
        params: &[ParamInfo {
            name: "period",
            param_type: "usize",
            default: "10",
            description: "Lookback period",
        }],
        convergence: 11,
        streaming: true,
    },
    IndicatorInfo {
        name: "ROCR",
        category: "momentum",
        description: "Rate of Change Ratio",
        params: &[ParamInfo {
            name: "period",
            param_type: "usize",
            default: "10",
            description: "Lookback period",
        }],
        convergence: 11,
        streaming: true,
    },
    IndicatorInfo {
        name: "ROCR100",
        category: "momentum",
        description: "Rate of Change Ratio * 100",
        params: &[ParamInfo {
            name: "period",
            param_type: "usize",
            default: "10",
            description: "Lookback period",
        }],
        convergence: 11,
        streaming: true,
    },
    IndicatorInfo {
        name: "MACDEXT",
        category: "momentum",
        description: "MACD with controllable MA type per line",
        params: &[
            ParamInfo {
                name: "fast_period",
                param_type: "usize",
                default: "12",
                description: "Fast MA period",
            },
            ParamInfo {
                name: "fast_ma",
                param_type: "usize",
                default: "1",
                description: "Fast MA type (0=SMA, 1=EMA)",
            },
            ParamInfo {
                name: "slow_period",
                param_type: "usize",
                default: "26",
                description: "Slow MA period",
            },
            ParamInfo {
                name: "slow_ma",
                param_type: "usize",
                default: "1",
                description: "Slow MA type (0=SMA, 1=EMA)",
            },
            ParamInfo {
                name: "signal_period",
                param_type: "usize",
                default: "9",
                description: "Signal line period",
            },
            ParamInfo {
                name: "signal_ma",
                param_type: "usize",
                default: "1",
                description: "Signal MA type (0=SMA, 1=EMA)",
            },
        ],
        convergence: 35,
        streaming: true,
    },
    IndicatorInfo {
        name: "MACDFIX",
        category: "momentum",
        description: "MACD with fixed 12/26 fast/slow EMA",
        params: &[ParamInfo {
            name: "signal_period",
            param_type: "usize",
            default: "9",
            description: "Signal line EMA period",
        }],
        convergence: 35,
        streaming: true,
    },
    IndicatorInfo {
        name: "Aroon",
        category: "momentum",
        description: "Aroon Up/Down oscillator",
        params: &[PERIOD],
        convergence: 15,
        streaming: true,
    },
    IndicatorInfo {
        name: "AROONOSC",
        category: "momentum",
        description: "Aroon Oscillator",
        params: &[PERIOD],
        convergence: 15,
        streaming: true,
    },
    IndicatorInfo {
        name: "APO",
        category: "momentum",
        description: "Absolute Price Oscillator",
        params: &FAST_SLOW,
        convergence: 26,
        streaming: true,
    },
    IndicatorInfo {
        name: "PPO",
        category: "momentum",
        description: "Percentage Price Oscillator",
        params: &FAST_SLOW,
        convergence: 26,
        streaming: true,
    },
    IndicatorInfo {
        name: "BOP",
        category: "momentum",
        description: "Balance of Power",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CMO",
        category: "momentum",
        description: "Chande Momentum Oscillator",
        params: &[PERIOD],
        convergence: 15,
        streaming: true,
    },
    IndicatorInfo {
        name: "ELDERRAY",
        category: "momentum",
        description: "Elder Ray Index (Bull/Bear Power)",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "DX",
        category: "momentum",
        description: "Directional Movement Index",
        params: &[PERIOD],
        convergence: 15,
        streaming: true,
    },
    IndicatorInfo {
        name: "MINUS_DI",
        category: "momentum",
        description: "Minus Directional Indicator",
        params: &[PERIOD],
        convergence: 15,
        streaming: true,
    },
    IndicatorInfo {
        name: "MINUS_DM",
        category: "momentum",
        description: "Minus Directional Movement",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "PLUS_DI",
        category: "momentum",
        description: "Plus Directional Indicator",
        params: &[PERIOD],
        convergence: 15,
        streaming: true,
    },
    IndicatorInfo {
        name: "PLUS_DM",
        category: "momentum",
        description: "Plus Directional Movement",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "STOCHF",
        category: "momentum",
        description: "Stochastic Fast",
        params: &[
            ParamInfo {
                name: "fastk_period",
                param_type: "usize",
                default: "5",
                description: "Fast %K period",
            },
            ParamInfo {
                name: "fastd_period",
                param_type: "usize",
                default: "3",
                description: "Fast %D period",
            },
        ],
        convergence: 5,
        streaming: true,
    },
    IndicatorInfo {
        name: "STOCHRSI",
        category: "momentum",
        description: "Stochastic RSI",
        params: &[
            ParamInfo {
                name: "rsi_period",
                param_type: "usize",
                default: "14",
                description: "RSI period",
            },
            ParamInfo {
                name: "stoch_period",
                param_type: "usize",
                default: "14",
                description: "Stochastic period",
            },
            ParamInfo {
                name: "fastk_period",
                param_type: "usize",
                default: "3",
                description: "Fast %K period",
            },
            ParamInfo {
                name: "fastd_period",
                param_type: "usize",
                default: "3",
                description: "Fast %D period",
            },
        ],
        convergence: 28,
        streaming: true,
    },
    IndicatorInfo {
        name: "ULTOSC",
        category: "momentum",
        description: "Ultimate Oscillator",
        params: &[
            ParamInfo {
                name: "period1",
                param_type: "usize",
                default: "7",
                description: "Short period",
            },
            ParamInfo {
                name: "period2",
                param_type: "usize",
                default: "14",
                description: "Medium period",
            },
            ParamInfo {
                name: "period3",
                param_type: "usize",
                default: "28",
                description: "Long period",
            },
        ],
        convergence: 29,
        streaming: true,
    },
    IndicatorInfo {
        name: "TRIX",
        category: "momentum",
        description: "Triple Smooth EMA Rate of Change",
        params: &[PERIOD],
        convergence: 42,
        streaming: true,
    },
    IndicatorInfo {
        name: "Elder Ray",
        category: "momentum",
        description: "Elder Ray Index (bull power / bear power)",
        params: &[ParamInfo {
            name: "period",
            param_type: "usize",
            default: "13",
            description: "EMA period for bull/bear power baseline",
        }],
        convergence: 13,
        streaming: true,
    },
    IndicatorInfo {
        name: "Fisher",
        category: "momentum",
        description: "Ehlers Fisher Transform",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "TSI",
        category: "momentum",
        description: "True Strength Index",
        params: &[
            ParamInfo {
                name: "long_period",
                param_type: "usize",
                default: "25",
                description: "Long EMA period",
            },
            ParamInfo {
                name: "short_period",
                param_type: "usize",
                default: "13",
                description: "Short EMA period",
            },
        ],
        convergence: 37,
        streaming: true,
    },
    // ── Volume ───────────────────────────────────────────────
    IndicatorInfo {
        name: "OBV",
        category: "volume",
        description: "On Balance Volume",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "VWAP",
        category: "volume",
        description: "Volume Weighted Average Price",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "AD",
        category: "volume",
        description: "Accumulation/Distribution Line",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "ADOSC",
        category: "volume",
        description: "Accumulation/Distribution Oscillator",
        params: &FAST_SLOW,
        convergence: 26,
        streaming: true,
    },
    IndicatorInfo {
        name: "Volume Profile",
        category: "volume",
        description: "Price-volume distribution histogram",
        params: &[ParamInfo {
            name: "num_bins",
            param_type: "usize",
            default: "20",
            description: "Number of price bins",
        }],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "Anchored VWAP",
        category: "volume",
        description: "VWAP anchored to a specific bar",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "VWAP Bands",
        category: "volume",
        description: "VWAP with standard deviation bands",
        params: &[ParamInfo {
            name: "stddev",
            param_type: "f64",
            default: "2.0",
            description: "Standard deviation multiplier",
        }],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "CMF",
        category: "volume",
        description: "Chaikin Money Flow",
        params: &[PERIOD_20],
        convergence: 20,
        streaming: true,
    },
    // ── Volatility ───────────────────────────────────────────
    IndicatorInfo {
        name: "ATR",
        category: "volatility",
        description: "Average True Range",
        params: &[PERIOD],
        convergence: 15,
        streaming: true,
    },
    IndicatorInfo {
        name: "NATR",
        category: "volatility",
        description: "Normalized Average True Range",
        params: &[PERIOD],
        convergence: 15,
        streaming: true,
    },
    IndicatorInfo {
        name: "TRANGE",
        category: "volatility",
        description: "True Range",
        params: &[],
        convergence: 2,
        streaming: true,
    },
    IndicatorInfo {
        name: "CHOP",
        category: "volatility",
        description: "Choppiness Index",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    // ── Price Transform ──────────────────────────────────────
    IndicatorInfo {
        name: "AVGPRICE",
        category: "price_transform",
        description: "Average Price (O+H+L+C)/4",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "MEDPRICE",
        category: "price_transform",
        description: "Median Price (H+L)/2",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "TYPPRICE",
        category: "price_transform",
        description: "Typical Price (H+L+C)/3",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "WCLPRICE",
        category: "price_transform",
        description: "Weighted Close Price (H+L+2C)/4",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "Pivot Points",
        category: "price_transform",
        description: "Classic pivot point support/resistance levels",
        params: &[ParamInfo {
            name: "pivot_type",
            param_type: "str",
            default: "standard",
            description: "Pivot calculation method (standard, fibonacci, camarilla)",
        }],
        convergence: 1,
        streaming: false,
    },
    // ── Cycle ────────────────────────────────────────────────
    IndicatorInfo {
        name: "HT_DCPERIOD",
        category: "cycle",
        description: "Hilbert Transform - Dominant Cycle Period",
        params: &[],
        convergence: 32,
        streaming: true,
    },
    IndicatorInfo {
        name: "HT_DCPHASE",
        category: "cycle",
        description: "Hilbert Transform - Dominant Cycle Phase",
        params: &[],
        convergence: 32,
        streaming: true,
    },
    IndicatorInfo {
        name: "HT_PHASOR",
        category: "cycle",
        description: "Hilbert Transform - Phasor Components (inphase, quadrature)",
        params: &[],
        convergence: 32,
        streaming: false,
    },
    IndicatorInfo {
        name: "HT_SINE",
        category: "cycle",
        description: "Hilbert Transform - SineWave (sine, leadsine)",
        params: &[],
        convergence: 32,
        streaming: true,
    },
    IndicatorInfo {
        name: "HT_TRENDMODE",
        category: "cycle",
        description: "Hilbert Transform - Trend vs Cycle Mode",
        params: &[],
        convergence: 32,
        streaming: true,
    },
    IndicatorInfo {
        name: "HT_TRENDLINE",
        category: "cycle",
        description: "Hilbert Transform - Instantaneous Trendline",
        params: &[],
        convergence: 32,
        streaming: true,
    },
    // ── Statistics ───────────────────────────────────────────
    IndicatorInfo {
        name: "ZSCORE",
        category: "statistics",
        description: "Rolling Z-Score",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "PERCENT_RANK",
        category: "statistics",
        description: "Percentile rank over period",
        params: &[PERIOD],
        convergence: 14,
        streaming: false,
    },
    IndicatorInfo {
        name: "BETA",
        category: "statistics",
        description: "Beta (covariance vs benchmark)",
        params: &[ParamInfo {
            name: "period",
            param_type: "usize",
            default: "20",
            description: "Lookback period",
        }],
        convergence: 20,
        streaming: true,
    },
    IndicatorInfo {
        name: "CORREL",
        category: "statistics",
        description: "Pearson correlation coefficient",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "STDDEV",
        category: "statistics",
        description: "Rolling standard deviation",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "AVGDEV",
        category: "statistics",
        description: "Mean Absolute Deviation over period",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "LINEAR_REG",
        category: "statistics",
        description: "Linear regression value",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "TSF",
        category: "statistics",
        description: "Time Series Forecast",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "LINREG_SLOPE",
        category: "statistics",
        description: "Linear regression slope",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "LINREG_INTERCEPT",
        category: "statistics",
        description: "Linear regression intercept",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "LINREG_ANGLE",
        category: "statistics",
        description: "Linear regression angle (degrees)",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "VAR",
        category: "statistics",
        description: "Rolling population variance",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "SKEWNESS",
        category: "statistics",
        description: "Statistical skewness",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "KURTOSIS",
        category: "statistics",
        description: "Statistical kurtosis",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    // ── Breadth ──────────────────────────────────────────────
    IndicatorInfo {
        name: "AD_LINE",
        category: "breadth",
        description: "Advance/Decline Line",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "AD_RATIO",
        category: "breadth",
        description: "Advance/Decline Ratio",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "McClellan Oscillator",
        category: "breadth",
        description: "McClellan Oscillator (breadth momentum)",
        params: &FAST_SLOW,
        convergence: 26,
        streaming: false,
    },
    IndicatorInfo {
        name: "McClellan Summation",
        category: "breadth",
        description: "McClellan Summation Index",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "TRIN",
        category: "breadth",
        description: "Arms Index (TRIN)",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "New Highs/Lows",
        category: "breadth",
        description: "Net new highs minus new lows",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    // ── Sentiment ────────────────────────────────────────────
    IndicatorInfo {
        name: "VIX-like Volatility",
        category: "sentiment",
        description: "VIX-style implied volatility proxy",
        params: &[ParamInfo {
            name: "period",
            param_type: "usize",
            default: "20",
            description: "Lookback window",
        }],
        convergence: 20,
        streaming: false,
    },
    IndicatorInfo {
        name: "Fear & Greed Index",
        category: "sentiment",
        description: "Composite fear and greed score",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "Put/Call Ratio",
        category: "sentiment",
        description: "Put volume / Call volume ratio",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "Volatility Index",
        category: "sentiment",
        description: "Annualised volatility index",
        params: &[ParamInfo {
            name: "period",
            param_type: "usize",
            default: "20",
            description: "Lookback window",
        }],
        convergence: 20,
        streaming: false,
    },
    IndicatorInfo {
        name: "PSY",
        category: "sentiment",
        description: "Psychological Line (心理线)",
        params: &[PERIOD],
        convergence: 15,
        streaming: true,
    },
    // ── China Indicators ──────────────────────────────────────
    IndicatorInfo {
        name: "VR",
        category: "volume",
        description: "Volume Ratio (成交量比率)",
        params: &[PERIOD],
        convergence: 27,
        streaming: true,
    },
    IndicatorInfo {
        name: "CR",
        category: "momentum",
        description: "Energy Indicator (能量指标)",
        params: &[PERIOD],
        convergence: 27,
        streaming: true,
    },
    IndicatorInfo {
        name: "DPO",
        category: "momentum",
        description: "Detrended Price Oscillator (去趋势价格振荡器)",
        params: &[PERIOD],
        convergence: 28,
        streaming: true,
    },
    IndicatorInfo {
        name: "AR",
        category: "momentum",
        description: "Activity Ratio (人气指标)",
        params: &[PERIOD],
        convergence: 26,
        streaming: true,
    },
    IndicatorInfo {
        name: "BR",
        category: "momentum",
        description: "Bias Ratio (意愿指标)",
        params: &[PERIOD],
        convergence: 27,
        streaming: true,
    },
    IndicatorInfo {
        name: "DMA",
        category: "overlap",
        description: "Different of Moving Averages (平行线差)",
        params: &[
            ParamInfo {
                name: "short_period",
                param_type: "usize",
                default: "10",
                description: "Short MA period",
            },
            ParamInfo {
                name: "long_period",
                param_type: "usize",
                default: "50",
                description: "Long MA period",
            },
            ParamInfo {
                name: "ama_period",
                param_type: "usize",
                default: "10",
                description: "AMA smoothing period",
            },
        ],
        convergence: 60,
        streaming: true,
    },
    IndicatorInfo {
        name: "ENE",
        category: "overlap",
        description: "Envelope (轨道线)",
        params: &[
            ParamInfo {
                name: "period",
                param_type: "usize",
                default: "25",
                description: "MA period",
            },
            ParamInfo {
                name: "k1",
                param_type: "f64",
                default: "11.0",
                description: "Upper percentage",
            },
            ParamInfo {
                name: "k2",
                param_type: "f64",
                default: "9.0",
                description: "Lower percentage",
            },
        ],
        convergence: 25,
        streaming: true,
    },
    IndicatorInfo {
        name: "EXPMA",
        category: "overlap",
        description: "Exponential Moving Average Group (指数平滑均线)",
        params: &[
            ParamInfo {
                name: "short_period",
                param_type: "usize",
                default: "12",
                description: "Short EMA period",
            },
            ParamInfo {
                name: "long_period",
                param_type: "usize",
                default: "50",
                description: "Long EMA period",
            },
        ],
        convergence: 50,
        streaming: true,
    },
    // ── A-Share Specific Indicators ──────────────────────────
    IndicatorInfo {
        name: "WINNER",
        category: "astock",
        description: "Winner ratio (获利盘比例) at a given cost basis",
        params: &[ParamInfo {
            name: "cost",
            param_type: "f64",
            default: "10.0",
            description: "Cost basis price",
        }],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "COST",
        category: "astock",
        description: "Cost distribution (成本分布) — price at a given winner ratio",
        params: &[ParamInfo {
            name: "winpct",
            param_type: "f64",
            default: "0.5",
            description: "Target winner ratio (0-1)",
        }],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "MAIN_NET_INFLOW",
        category: "astock",
        description: "Main capital net inflow (主力净流入) via large-trade threshold",
        params: &[ParamInfo {
            name: "large_threshold",
            param_type: "f64",
            default: "1000000.0",
            description: "Trade amount threshold for large orders",
        }],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "MONEY_FLOW",
        category: "astock",
        description: "Rolling sum of typical price × volume (资金流量)",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "LIMIT_UP",
        category: "astock",
        description: "Limit-up detection (涨停检测) — 1.0 if change >= threshold",
        params: &[ParamInfo {
            name: "threshold",
            param_type: "f64",
            default: "0.10",
            description: "Daily price limit (0.10 main board, 0.20 ChiNext/STAR)",
        }],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "LIMIT_DOWN",
        category: "astock",
        description: "Limit-down detection (跌停检测) — 1.0 if change <= -threshold",
        params: &[ParamInfo {
            name: "threshold",
            param_type: "f64",
            default: "0.10",
            description: "Daily price limit",
        }],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CONSECUTIVE_LIMIT",
        category: "astock",
        description: "Consecutive limit-up count (连板数)",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "TURNOVER",
        category: "astock",
        description: "Turnover rate (换手率) = volume / free-float shares",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "RS_RATIO",
        category: "astock",
        description: "Relative strength ratio vs benchmark (相对强弱)",
        params: &[PERIOD],
        convergence: 14,
        streaming: false,
    },
    // ── Volatility Extended ───────────────────────────────────
    IndicatorInfo {
        name: "Mass Index",
        category: "volatility",
        description: "Mass Index",
        params: &[
            ParamInfo {
                name: "period",
                param_type: "usize",
                default: "25",
                description: "Summation period",
            },
            ParamInfo {
                name: "ema_period",
                param_type: "usize",
                default: "9",
                description: "EMA smoothing period",
            },
        ],
        convergence: 43,
        streaming: true,
    },
    IndicatorInfo {
        name: "Ulcer Index",
        category: "volatility",
        description: "Ulcer Index",
        params: &[ParamInfo {
            name: "period",
            param_type: "usize",
            default: "14",
            description: "Lookback period",
        }],
        convergence: 27,
        streaming: true,
    },
    IndicatorInfo {
        name: "RVI",
        category: "volatility",
        description: "Relative Vigor Index",
        params: &[ParamInfo {
            name: "period",
            param_type: "usize",
            default: "10",
            description: "SMA smoothing period",
        }],
        convergence: 16,
        streaming: true,
    },
    // ── Moving Average Extended ──────────────────────────────
    IndicatorInfo {
        name: "McGinley",
        category: "overlap",
        description: "McGinley Dynamic",
        params: &[ParamInfo {
            name: "period",
            param_type: "usize",
            default: "14",
            description: "Dynamic period",
        }],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "ZLEMA",
        category: "overlap",
        description: "Zero Lag EMA",
        params: &[ParamInfo {
            name: "period",
            param_type: "usize",
            default: "14",
            description: "EMA period",
        }],
        convergence: 21,
        streaming: true,
    },
    IndicatorInfo {
        name: "VIDYA",
        category: "overlap",
        description: "Variable Index Dynamic Average",
        params: &[
            ParamInfo {
                name: "period",
                param_type: "usize",
                default: "14",
                description: "VIDYA period",
            },
            ParamInfo {
                name: "cmo_period",
                param_type: "usize",
                default: "9",
                description: "CMO lookback period",
            },
        ],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "VWMA",
        category: "overlap",
        description: "Volume Weighted Moving Average",
        params: &[ParamInfo {
            name: "period",
            param_type: "usize",
            default: "20",
            description: "Lookback period",
        }],
        convergence: 20,
        streaming: true,
    },
    // ── Volume Extended ──────────────────────────────────────
    IndicatorInfo {
        name: "Force Index",
        category: "volume",
        description: "Force Index",
        params: &[ParamInfo {
            name: "period",
            param_type: "usize",
            default: "13",
            description: "EMA smoothing period",
        }],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "EOM",
        category: "volume",
        description: "Ease of Movement",
        params: &[ParamInfo {
            name: "period",
            param_type: "usize",
            default: "14",
            description: "SMA smoothing period",
        }],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "NVI",
        category: "volume",
        description: "Negative Volume Index",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "PVI",
        category: "volume",
        description: "Positive Volume Index",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "PVT",
        category: "volume",
        description: "Price Volume Trend",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "KVO",
        category: "volume",
        description: "Klinger Volume Oscillator",
        params: &[
            ParamInfo {
                name: "fast_period",
                param_type: "usize",
                default: "34",
                description: "Fast EMA period",
            },
            ParamInfo {
                name: "slow_period",
                param_type: "usize",
                default: "55",
                description: "Slow EMA period",
            },
            ParamInfo {
                name: "signal_period",
                param_type: "usize",
                default: "13",
                description: "Signal EMA period",
            },
        ],
        convergence: 68,
        streaming: true,
    },
    // ── Momentum Extended ────────────────────────────────────
    IndicatorInfo {
        name: "AO",
        category: "momentum",
        description: "Awesome Oscillator",
        params: &[
            ParamInfo {
                name: "fast_period",
                param_type: "usize",
                default: "5",
                description: "Fast SMA period",
            },
            ParamInfo {
                name: "slow_period",
                param_type: "usize",
                default: "34",
                description: "Slow SMA period",
            },
        ],
        convergence: 34,
        streaming: true,
    },
    IndicatorInfo {
        name: "Coppock",
        category: "momentum",
        description: "Coppock Curve",
        params: &[
            ParamInfo {
                name: "wma_period",
                param_type: "usize",
                default: "10",
                description: "WMA smoothing period",
            },
            ParamInfo {
                name: "long_roc",
                param_type: "usize",
                default: "14",
                description: "Long ROC period",
            },
            ParamInfo {
                name: "short_roc",
                param_type: "usize",
                default: "11",
                description: "Short ROC period",
            },
        ],
        convergence: 24,
        streaming: true,
    },
    IndicatorInfo {
        name: "KST",
        category: "momentum",
        description: "Know Sure Thing",
        params: &[
            ParamInfo {
                name: "roc1",
                param_type: "usize",
                default: "10",
                description: "ROC period 1",
            },
            ParamInfo {
                name: "roc2",
                param_type: "usize",
                default: "15",
                description: "ROC period 2",
            },
            ParamInfo {
                name: "roc3",
                param_type: "usize",
                default: "20",
                description: "ROC period 3",
            },
            ParamInfo {
                name: "roc4",
                param_type: "usize",
                default: "30",
                description: "ROC period 4",
            },
            ParamInfo {
                name: "sma1",
                param_type: "usize",
                default: "10",
                description: "SMA period 1",
            },
            ParamInfo {
                name: "sma2",
                param_type: "usize",
                default: "10",
                description: "SMA period 2",
            },
            ParamInfo {
                name: "sma3",
                param_type: "usize",
                default: "10",
                description: "SMA period 3",
            },
            ParamInfo {
                name: "sma4",
                param_type: "usize",
                default: "15",
                description: "SMA period 4",
            },
            ParamInfo {
                name: "signal_period",
                param_type: "usize",
                default: "9",
                description: "Signal line SMA period",
            },
        ],
        convergence: 54,
        streaming: true,
    },
    IndicatorInfo {
        name: "STC",
        category: "momentum",
        description: "Schaff Trend Cycle",
        params: &[
            ParamInfo {
                name: "fast_period",
                param_type: "usize",
                default: "23",
                description: "Fast EMA period",
            },
            ParamInfo {
                name: "slow_period",
                param_type: "usize",
                default: "50",
                description: "Slow EMA period",
            },
            ParamInfo {
                name: "cycle",
                param_type: "usize",
                default: "10",
                description: "Stochastic cycle period",
            },
        ],
        convergence: 70,
        streaming: true,
    },
    // ── New Indicators (TASK-166~180) ──────────────────────────
    IndicatorInfo {
        name: "Vortex",
        category: "momentum",
        description: "Vortex Indicator (VI+/VI-)",
        params: &[PERIOD],
        convergence: 15,
        streaming: true,
    },
    IndicatorInfo {
        name: "Inertia",
        category: "momentum",
        description: "Inertia Indicator (RVI + Linear Regression)",
        params: &[
            ParamInfo {
                name: "rvi_period",
                param_type: "usize",
                default: "10",
                description: "RVI period",
            },
            ParamInfo {
                name: "linreg_period",
                param_type: "usize",
                default: "14",
                description: "Linear regression period",
            },
        ],
        convergence: 24,
        streaming: true,
    },
    IndicatorInfo {
        name: "Squeeze Momentum",
        category: "momentum",
        description: "Squeeze Momentum (John Carter)",
        params: &[
            ParamInfo {
                name: "bb_period",
                param_type: "usize",
                default: "20",
                description: "Bollinger Bands period",
            },
            ParamInfo {
                name: "kc_period",
                param_type: "usize",
                default: "20",
                description: "Keltner Channel period",
            },
            ParamInfo {
                name: "mom_period",
                param_type: "usize",
                default: "12",
                description: "Momentum period",
            },
        ],
        convergence: 20,
        streaming: true,
    },
    IndicatorInfo {
        name: "QStick",
        category: "momentum",
        description: "QStick Indicator",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "JMA",
        category: "overlap",
        description: "Jurik Moving Average",
        params: &[
            ParamInfo {
                name: "period",
                param_type: "usize",
                default: "7",
                description: "Lookback period",
            },
            ParamInfo {
                name: "phase",
                param_type: "f64",
                default: "0.0",
                description: "Phase shift (-100 to 100)",
            },
            ParamInfo {
                name: "power",
                param_type: "f64",
                default: "2.0",
                description: "Smoothing power",
            },
        ],
        convergence: 7,
        streaming: true,
    },
    IndicatorInfo {
        name: "Efficiency Ratio",
        category: "statistics",
        description: "Kaufman Efficiency Ratio",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "CFO",
        category: "momentum",
        description: "Chande Forecast Oscillator",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "Twiggs MF",
        category: "volume",
        description: "Twiggs Money Flow",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "Keltner Channel",
        category: "volatility",
        description: "Keltner Channel (upper, middle, lower)",
        params: &[
            ParamInfo {
                name: "ema_period",
                param_type: "usize",
                default: "20",
                description: "EMA period",
            },
            ParamInfo {
                name: "atr_period",
                param_type: "usize",
                default: "10",
                description: "ATR period",
            },
            ParamInfo {
                name: "multiplier",
                param_type: "f64",
                default: "2.0",
                description: "ATR multiplier",
            },
        ],
        convergence: 20,
        streaming: true,
    },
    IndicatorInfo {
        name: "ADR",
        category: "volatility",
        description: "Average Day Range",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "Chaikin Volatility",
        category: "volatility",
        description: "Chaikin Volatility (EMA of H-L range change)",
        params: &[
            ParamInfo {
                name: "ema_period",
                param_type: "usize",
                default: "10",
                description: "EMA smoothing period",
            },
            ParamInfo {
                name: "roc_period",
                param_type: "usize",
                default: "10",
                description: "Rate of change period",
            },
        ],
        convergence: 20,
        streaming: true,
    },
    IndicatorInfo {
        name: "HV",
        category: "volatility",
        description: "Historical Volatility (Close-to-Close)",
        params: &[PERIOD_20],
        convergence: 21,
        streaming: true,
    },
    IndicatorInfo {
        name: "VZO",
        category: "volume",
        description: "Volume Zone Oscillator",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "VWAP MTF",
        category: "volume",
        description: "Multi-timeframe VWAP with session resets",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "Volume Momentum",
        category: "volume",
        description: "Volume Momentum (Volume - SMA(Volume))",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "Volume ROC",
        category: "volume",
        description: "Volume Rate of Change",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    // ── Fibonacci ────────────────────────────────────────────
    IndicatorInfo {
        name: "Fibonacci Retracement",
        category: "fibonacci",
        description: "Fibonacci retracement levels (23.6%, 38.2%, 50%, 61.8%, 78.6%)",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    // ── Math Transform ──────────────────────────────────────
    IndicatorInfo {
        name: "ACOS",
        category: "math_transform",
        description: "Vector Trigonometric ACos",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "ASIN",
        category: "math_transform",
        description: "Vector Trigonometric ASin",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "ATAN",
        category: "math_transform",
        description: "Vector Trigonometric ATan",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "CEIL",
        category: "math_transform",
        description: "Vector Ceil",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "COS",
        category: "math_transform",
        description: "Vector Trigonometric Cos",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "COSH",
        category: "math_transform",
        description: "Vector Trigonometric Cosh",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "EXP",
        category: "math_transform",
        description: "Vector Exp",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "FLOOR",
        category: "math_transform",
        description: "Vector Floor",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "LN",
        category: "math_transform",
        description: "Vector Natural Logarithm",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "LOG10",
        category: "math_transform",
        description: "Vector Log10",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "SIN",
        category: "math_transform",
        description: "Vector Trigonometric Sin",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "SINH",
        category: "math_transform",
        description: "Vector Trigonometric Sinh",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "SQRT",
        category: "math_transform",
        description: "Vector Square Root",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "TAN",
        category: "math_transform",
        description: "Vector Trigonometric Tan",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "TANH",
        category: "math_transform",
        description: "Vector Trigonometric Tanh",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    // ── Math Operators ──────────────────────────────────────
    IndicatorInfo {
        name: "ADD",
        category: "math_operators",
        description: "Vector Arithmetic Add",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "SUB",
        category: "math_operators",
        description: "Vector Arithmetic Sub",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "MULT",
        category: "math_operators",
        description: "Vector Arithmetic Mult",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "DIV",
        category: "math_operators",
        description: "Vector Arithmetic Div",
        params: &[],
        convergence: 1,
        streaming: true,
    },
    IndicatorInfo {
        name: "MAX",
        category: "math_operators",
        description: "Highest value over a period",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "MIN",
        category: "math_operators",
        description: "Lowest value over a period",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "MAXINDEX",
        category: "math_operators",
        description: "Index of highest value within period",
        params: &[PERIOD],
        convergence: 14,
        streaming: false,
    },
    IndicatorInfo {
        name: "MININDEX",
        category: "math_operators",
        description: "Index of lowest value within period",
        params: &[PERIOD],
        convergence: 14,
        streaming: false,
    },
    IndicatorInfo {
        name: "MINUS",
        category: "math_operators",
        description: "Vector Arithmetic Minus (period difference)",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    IndicatorInfo {
        name: "SUM",
        category: "math_operators",
        description: "Summation over a period",
        params: &[PERIOD],
        convergence: 14,
        streaming: true,
    },
    // ── Pattern (candlestick) ────────────────────────────────
    IndicatorInfo {
        name: "CDL_DOJI",
        category: "pattern",
        description: "Doji candlestick pattern",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_DRAGONFLY_DOJI",
        category: "pattern",
        description: "Dragonfly Doji pattern",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_GRAVESTONE_DOJI",
        category: "pattern",
        description: "Gravestone Doji pattern",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_LONGLEGGED_DOJI",
        category: "pattern",
        description: "Long-Legged Doji pattern",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_4PRICE_DOJI",
        category: "pattern",
        description: "Four-Price Doji pattern",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_MARUBOZU",
        category: "pattern",
        description: "Marubozu (full body, no wicks)",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_HAMMER",
        category: "pattern",
        description: "Hammer bullish reversal pattern",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_INVERTED_HAMMER",
        category: "pattern",
        description: "Inverted Hammer pattern",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_HANGING_MAN",
        category: "pattern",
        description: "Hanging Man bearish reversal",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_SHOOTING_STAR",
        category: "pattern",
        description: "Shooting Star bearish reversal",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_ENGULFING",
        category: "pattern",
        description: "Bullish/Bearish Engulfing pattern",
        params: &[],
        convergence: 2,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_HARAMI",
        category: "pattern",
        description: "Harami pattern",
        params: &[],
        convergence: 2,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_HARAMI_CROSS",
        category: "pattern",
        description: "Harami Cross pattern",
        params: &[],
        convergence: 2,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_MORNINGSTAR",
        category: "pattern",
        description: "Morning Star bullish reversal",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_EVENINGSTAR",
        category: "pattern",
        description: "Evening Star bearish reversal",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_MORNING_DOJI_STAR",
        category: "pattern",
        description: "Morning Doji Star pattern",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_EVENING_DOJI_STAR",
        category: "pattern",
        description: "Evening Doji Star pattern",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_3WHITE_SOLDIERS",
        category: "pattern",
        description: "Three White Soldiers bullish",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_3BLACK_CROWS",
        category: "pattern",
        description: "Three Black Crows bearish",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_3INSIDE_UP",
        category: "pattern",
        description: "Three Inside Up bullish confirmation",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_3OUTSIDE_UP",
        category: "pattern",
        description: "Three Outside Up bullish continuation",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_3INSIDE_DOWN",
        category: "pattern",
        description: "Three Inside Down bearish confirmation",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_3OUTSIDE_DOWN",
        category: "pattern",
        description: "Three Outside Down bearish continuation",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_3STARS_IN_SOUTH",
        category: "pattern",
        description: "Three Stars in the South pattern",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_3LINE_STRIKE",
        category: "pattern",
        description: "Three-Line Strike pattern",
        params: &[],
        convergence: 4,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_STICK_SANDWICH",
        category: "pattern",
        description: "Stick Sandwich pattern",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_BELT_HOLD",
        category: "pattern",
        description: "Belt Hold pattern",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_CLOSING_MARUBOZU",
        category: "pattern",
        description: "Closing Marubozu pattern",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_SPINNING_TOP",
        category: "pattern",
        description: "Spinning Top indecision pattern",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_HIGH_WAVE",
        category: "pattern",
        description: "High Wave candle pattern",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_RICKSHAW_MAN",
        category: "pattern",
        description: "Rickshaw Man pattern",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_SHORT_LINE",
        category: "pattern",
        description: "Short Line candle",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_LONG_LINE",
        category: "pattern",
        description: "Long Line candle",
        params: &[],
        convergence: 1,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_PIERCING",
        category: "pattern",
        description: "Piercing Line bullish reversal",
        params: &[],
        convergence: 2,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_DARK_CLOUD",
        category: "pattern",
        description: "Dark Cloud Cover bearish reversal",
        params: &[],
        convergence: 2,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_TWEEZER_TOP",
        category: "pattern",
        description: "Tweezer Top bearish reversal",
        params: &[],
        convergence: 2,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_TWEEZER_BOT",
        category: "pattern",
        description: "Tweezer Bottom bullish reversal",
        params: &[],
        convergence: 2,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_ABANDONED_BABY",
        category: "pattern",
        description: "Abandoned Baby reversal pattern",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_UPSIDE_GAP_2CROWS",
        category: "pattern",
        description: "Upside Gap Two Crows bearish",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_UPSIDE_GAP_3METHODS",
        category: "pattern",
        description: "Upside/Downside Gap Three Methods",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_MAT_HOLD",
        category: "pattern",
        description: "Mat Hold bullish continuation",
        params: &[],
        convergence: 5,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_TASUKI_GAP",
        category: "pattern",
        description: "Tasuki Gap continuation pattern",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_SEPARATING_LINES",
        category: "pattern",
        description: "Separating Lines pattern",
        params: &[],
        convergence: 2,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_COUNTER_ATTACK",
        category: "pattern",
        description: "Counter Attack pattern",
        params: &[],
        convergence: 2,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_MATCHING_LOW",
        category: "pattern",
        description: "Matching Low bullish pattern",
        params: &[],
        convergence: 2,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_IDENTICAL_3CROWS",
        category: "pattern",
        description: "Identical Three Crows bearish",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_UNIQUE_3RIVER",
        category: "pattern",
        description: "Unique Three River Bottom",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_BREAKAWAY",
        category: "pattern",
        description: "Breakaway pattern",
        params: &[],
        convergence: 5,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_CONCEALING_BABY",
        category: "pattern",
        description: "Concealing Baby Swallow bearish",
        params: &[],
        convergence: 4,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_KICKING",
        category: "pattern",
        description: "Kicking pattern",
        params: &[],
        convergence: 2,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_KICKING_BY_LENGTH",
        category: "pattern",
        description: "Kicking — determined by longer marubozu",
        params: &[],
        convergence: 2,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_ADVANCE_BLOCK",
        category: "pattern",
        description: "Advance Block bearish weakening",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_STALLED_PATTERN",
        category: "pattern",
        description: "Stalled Pattern (Deliberation)",
        params: &[],
        convergence: 3,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_THRUSTING",
        category: "pattern",
        description: "Thrusting Pattern",
        params: &[],
        convergence: 2,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_IN_NECK",
        category: "pattern",
        description: "In-Neck Pattern",
        params: &[],
        convergence: 2,
        streaming: false,
    },
    IndicatorInfo {
        name: "CDL_ON_NECK",
        category: "pattern",
        description: "On-Neck Pattern",
        params: &[],
        convergence: 2,
        streaming: false,
    },
];

/// Lazily-built, process-wide cache of the registered indicator metadata.
fn ensure_lookups() -> &'static [IndicatorInfo] {
    let slice = INDICATOR_CACHE.get_or_init(|| INDICATORS.to_vec());
    INDICATOR_BY_ID.get_or_init(|| {
        slice
            .iter()
            .enumerate()
            .map(|(i, ind)| (ind.name, i))
            .collect()
    });
    INDICATOR_BY_CATEGORY.get_or_init(|| {
        let mut map: HashMap<&'static str, Vec<usize>> = HashMap::new();
        for (i, ind) in slice.iter().enumerate() {
            map.entry(ind.category).or_default().push(i);
        }
        map
    });
    slice.as_slice()
}

/// Returns every registered indicator metadata entry.
///
/// The slice is backed by a `OnceLock`-cached `Vec` constructed on the first
/// call; subsequent calls reuse the cached allocation and return in O(1).
pub fn all_indicators() -> &'static [IndicatorInfo] {
    ensure_lookups()
}

/// Returns the indicator registered under the given name, if any.
///
/// Backed by a `OnceLock<HashMap>` lookup, so O(1) on the hot path (after the
/// first call that builds the cache).
pub fn by_id(name: &str) -> Option<&'static IndicatorInfo> {
    let slice = ensure_lookups();
    INDICATOR_BY_ID
        .get()
        .and_then(|m| m.get(name).map(|&i| &slice[i]))
}

/// Returns the slice of indicators belonging to the given category
/// (e.g. `"momentum"`, `"overlap"`).
///
/// Returns a `Vec` of `&IndicatorInfo` references.  The internal index lookup
/// (`INDICATOR_BY_CATEGORY`) is built once and cached, so the O(1) hash
/// lookup dominates; the per-call `Vec` allocation is a small `n` (≤ ~50).
pub fn by_category(category: &str) -> Vec<&'static IndicatorInfo> {
    let slice = ensure_lookups();
    INDICATOR_BY_CATEGORY
        .get()
        .and_then(|m| m.get(category))
        .map(|idxs| idxs.iter().map(|&i| &slice[i]).collect())
        .unwrap_or_default()
}

/// Builds the full registry document for JSON serialization.
pub fn registry_document() -> RegistryDocument {
    RegistryDocument {
        version: env!("CARGO_PKG_VERSION"),
        generated_at: None,
        indicators: all_indicators(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;

    fn docs_registry_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("docs")
            .join("indicator_registry.json")
    }

    #[test]
    fn test_registry_has_at_least_70() {
        assert!(
            all_indicators().len() >= 70,
            "expected >= 70 indicators, got {}",
            all_indicators().len()
        );
    }

    #[test]
    fn test_all_have_required_fields() {
        for indicator in all_indicators() {
            assert!(!indicator.name.is_empty(), "name must not be empty");
            assert!(!indicator.category.is_empty(), "category must not be empty");
            assert!(
                !indicator.description.is_empty(),
                "description must not be empty for {}",
                indicator.name
            );
            assert!(
                indicator.convergence > 0,
                "convergence must be > 0 for {}",
                indicator.name
            );
            for param in indicator.params {
                assert!(!param.name.is_empty());
                assert!(!param.param_type.is_empty());
                assert!(!param.default.is_empty());
            }
        }
    }

    #[test]
    fn test_unique_names() {
        let names: Vec<_> = all_indicators().iter().map(|i| i.name).collect();
        let unique: HashSet<_> = names.iter().copied().collect();
        assert_eq!(
            names.len(),
            unique.len(),
            "duplicate indicator names detected"
        );
    }

    #[test]
    fn test_valid_categories() {
        for indicator in all_indicators() {
            assert!(
                VALID_CATEGORIES.contains(&indicator.category),
                "invalid category '{}' for {}",
                indicator.category,
                indicator.name
            );
        }
    }

    #[test]
    fn test_json_roundtrip() {
        let json = serde_json::to_string(&registry_document()).expect("serialize registry");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse registry json");

        assert_eq!(parsed["version"], env!("CARGO_PKG_VERSION"));
        assert!(parsed["generated_at"].is_null());

        let indicators = parsed["indicators"]
            .as_array()
            .expect("indicators must be an array");
        assert!(indicators.len() >= 70);

        for entry in indicators {
            assert!(entry.get("name").is_some());
            assert!(entry.get("category").is_some());
            assert!(entry.get("params").is_some());
            assert!(entry.get("convergence").is_some());
        }
    }

    #[test]
    fn test_docs_json_matches_registry() {
        let path = docs_registry_path();
        assert!(
            path.exists(),
            "docs/indicator_registry.json must exist at {}",
            path.display()
        );

        let file_contents = fs::read_to_string(&path).expect("read docs/indicator_registry.json");
        let parsed: serde_json::Value =
            serde_json::from_str(&file_contents).expect("docs JSON must be valid");

        let expected = serde_json::to_value(registry_document()).expect("serialize registry");

        assert_eq!(
            parsed, expected,
            "docs/indicator_registry.json must match registry_document()"
        );
    }

    #[test]
    fn test_registry_coverage() {
        let streaming_indicators: Vec<_> = all_indicators()
            .into_iter()
            .filter(|i| i.streaming)
            .collect();
        assert!(
            streaming_indicators.len() >= 90,
            "expected >= 90 streaming indicators, got {}",
            streaming_indicators.len()
        );

        let non_streamable_categories = ["pattern", "breadth", "fibonacci"];
        for ind in all_indicators() {
            if non_streamable_categories.contains(&ind.category) {
                assert!(
                    !ind.streaming,
                    "{} in category {} should not be marked streaming",
                    ind.name, ind.category
                );
            }
        }
    }

    #[test]
    fn test_registry_document() {
        let doc = registry_document();
        let json = serde_json::to_value(&doc).expect("serialize registry");
        let indicators = json["indicators"].as_array().unwrap();

        for entry in indicators {
            let name = entry["name"].as_str().unwrap();
            let streaming = entry.get("streaming");
            assert!(
                streaming.is_some(),
                "indicator {} missing 'streaming' field",
                name
            );
            let streaming_val = streaming.unwrap().as_bool().unwrap();

            let category = entry["category"].as_str().unwrap();
            if category == "pattern" || category == "breadth" || category == "fibonacci" {
                assert!(
                    !streaming_val,
                    "{} in {} should have streaming=false",
                    name, category
                );
            }
        }

        assert!(
            indicators.len() >= 90,
            "registry must have >= 90 entries, got {}",
            indicators.len()
        );
    }

    /// Regenerate `docs/indicator_registry.json` when registry entries change.
    #[test]
    #[ignore]
    fn generate_indicator_registry_json() {
        let json = serde_json::to_string_pretty(&registry_document()).expect("serialize registry");
        fs::write(docs_registry_path(), json).expect("write indicator_registry.json");
    }
}
