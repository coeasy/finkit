//! Runtime factor output model.

use finkit_series::QuantSeries;

#[derive(Clone, Debug)]
pub struct FactorOutput {
    pub factor_name: String,
    pub series: QuantSeries,
}

impl FactorOutput {
    pub fn new(name: impl Into<String>, series: QuantSeries) -> Self {
        Self {
            factor_name: name.into(),
            series,
        }
    }
}
