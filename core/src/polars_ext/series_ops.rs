use polars::prelude::*;
use crate::math::moving_avg;
use crate::indicators;

/// Extension trait for Polars `Series` providing technical analysis operations.
pub trait TaSeries {
    /// Simple Moving Average over `period` bars.
    fn ta_sma(&self, period: usize) -> PolarsResult<Series>;
    /// Exponential Moving Average over `period` bars.
    fn ta_ema(&self, period: usize) -> PolarsResult<Series>;
    /// Relative Strength Index over `period` bars.
    fn ta_rsi(&self, period: usize) -> PolarsResult<Series>;
    /// Bollinger Bands (returns 3 series: upper, middle, lower).
    fn ta_bbands(&self, period: usize, num_std: f64) -> PolarsResult<(Series, Series, Series)>;
}

impl TaSeries for Series {
    fn ta_sma(&self, period: usize) -> PolarsResult<Series> {
        let ca = self.f64()?;
        let values: Vec<f64> = ca.into_no_null_iter().collect();
        let result = moving_avg::sma(&values, period)
            .map_err(|e| PolarsError::ComputeError(format!("{}", e).into()))?;
        let out: Float64Chunked = Float64Chunked::from_vec(self.name().clone(), result.to_vec());
        Ok(out.into_series())
    }

    fn ta_ema(&self, period: usize) -> PolarsResult<Series> {
        let ca = self.f64()?;
        let values: Vec<f64> = ca.into_no_null_iter().collect();
        let result = moving_avg::ema(&values, period)
            .map_err(|e| PolarsError::ComputeError(format!("{}", e).into()))?;
        let out: Float64Chunked = Float64Chunked::from_vec(self.name().clone(), result.to_vec());
        Ok(out.into_series())
    }

    fn ta_rsi(&self, period: usize) -> PolarsResult<Series> {
        let ca = self.f64()?;
        let values: Vec<f64> = ca.into_no_null_iter().collect();
        let result = indicators::rsi(&values, period)
            .map_err(|e| PolarsError::ComputeError(format!("{}", e).into()))?;
        let out: Float64Chunked = Float64Chunked::from_vec(self.name().clone(), result.to_vec());
        Ok(out.into_series())
    }

    fn ta_bbands(&self, period: usize, num_std: f64) -> PolarsResult<(Series, Series, Series)> {
        let ca = self.f64()?;
        let values: Vec<f64> = ca.into_no_null_iter().collect();
        let result = indicators::bbands(&values, period, num_std, num_std)
            .map_err(|e| PolarsError::ComputeError(format!("{}", e).into()))?;
        let upper = Float64Chunked::from_vec("upper".into(), result.upper.into_raw_vec_and_offset().0);
        let middle = Float64Chunked::from_vec("middle".into(), result.middle.into_raw_vec_and_offset().0);
        let lower = Float64Chunked::from_vec("lower".into(), result.lower.into_raw_vec_and_offset().0);
        Ok((upper.into_series(), middle.into_series(), lower.into_series()))
    }
}
