//! Factor result model.

use finkit_series::QuantSeries;

#[derive(Clone, Debug)]
pub struct FactorResult {
    pub factor_name: String,
    pub series: QuantSeries,
}

impl FactorResult {
    pub fn new(factor_name: impl Into<String>, series: QuantSeries) -> Self {
        Self {
            factor_name: factor_name.into(),
            series,
        }
    }
}
