use polars::prelude::*;
use super::series_ops::TaSeries;

/// Accessor struct for chaining TA operations on a DataFrame.
pub struct TaAccessor<'a> {
    df: &'a DataFrame,
}

impl<'a> TaAccessor<'a> {
    /// Compute SMA on the named column.
    pub fn sma(&self, column: &str, period: usize) -> PolarsResult<Series> {
        let series = self.df.column(column)?.as_materialized_series();
        series.ta_sma(period)
    }

    /// Compute EMA on the named column.
    pub fn ema(&self, column: &str, period: usize) -> PolarsResult<Series> {
        let series = self.df.column(column)?.as_materialized_series();
        series.ta_ema(period)
    }

    /// Compute RSI on the named column.
    pub fn rsi(&self, column: &str, period: usize) -> PolarsResult<Series> {
        let series = self.df.column(column)?.as_materialized_series();
        series.ta_rsi(period)
    }

    /// Compute Bollinger Bands on the named column.
    pub fn bbands(&self, column: &str, period: usize, num_std: f64) -> PolarsResult<(Series, Series, Series)> {
        let series = self.df.column(column)?.as_materialized_series();
        series.ta_bbands(period, num_std)
    }
}

/// Extension trait for DataFrame to create a TA accessor.
pub trait TaDataFrame {
    /// Get a TA accessor for this DataFrame.
    fn ta(&self) -> TaAccessor<'_>;
}

impl TaDataFrame for DataFrame {
    fn ta(&self) -> TaAccessor<'_> {
        TaAccessor { df: self }
    }
}
