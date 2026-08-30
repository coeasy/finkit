use super::traits::Ohlcv;

/// Unified price source selector for all indicators.
///
/// When an indicator needs a single price value from an OHLCV bar,
/// `PriceSource` determines which derived price to use.
///
/// # Example
///
/// ```
/// use alpha_ta_core::streaming::{Ohlcv, OhlcvBar, PriceSource};
///
/// let bar = OhlcvBar::new(100.0, 110.0, 90.0, 105.0, 50000.0);
/// assert_eq!(PriceSource::Close.extract(&bar), 105.0);
/// assert_eq!(PriceSource::HL2.extract(&bar), 100.0);
/// ```
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PriceSource {
    /// Bar open price.
    Open,
    /// Bar high price.
    High,
    /// Bar low price.
    Low,
    /// Bar close price (default for most indicators).
    #[default]
    Close,
    /// (High + Low) / 2
    HL2,
    /// (High + Low + Close) / 3
    HLC3,
    /// (Open + High + Low + Close) / 4
    OHLC4,
    /// Alias for HLC3: (High + Low + Close) / 3
    Typical,
    /// (High + Low + 2 * Close) / 4
    Weighted,
    /// Alias for HL2: (High + Low) / 2
    Median,
    /// Bar volume.
    Volume,
}

impl PriceSource {
    /// Extract the selected price from an OHLCV bar.
    #[inline]
    pub fn extract(&self, bar: &dyn Ohlcv) -> f64 {
        match self {
            Self::Open => bar.open(),
            Self::High => bar.high(),
            Self::Low => bar.low(),
            Self::Close => bar.close(),
            Self::HL2 => (bar.high() + bar.low()) * 0.5,
            Self::HLC3 => (bar.high() + bar.low() + bar.close()) / 3.0,
            Self::OHLC4 => (bar.open() + bar.high() + bar.low() + bar.close()) * 0.25,
            Self::Typical => (bar.high() + bar.low() + bar.close()) / 3.0,
            Self::Weighted => (bar.high() + bar.low() + 2.0 * bar.close()) * 0.25,
            Self::Median => (bar.high() + bar.low()) * 0.5,
            Self::Volume => bar.volume(),
        }
    }
}

impl std::fmt::Display for PriceSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::High => write!(f, "high"),
            Self::Low => write!(f, "low"),
            Self::Close => write!(f, "close"),
            Self::HL2 => write!(f, "hl2"),
            Self::HLC3 => write!(f, "hlc3"),
            Self::OHLC4 => write!(f, "ohlc4"),
            Self::Typical => write!(f, "typical"),
            Self::Weighted => write!(f, "weighted"),
            Self::Median => write!(f, "median"),
            Self::Volume => write!(f, "volume"),
        }
    }
}
