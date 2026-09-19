//! Factor result model.

use finkit_series::QuantSeries;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct FactorResult {
    pub factor_name: String,
    pub series: QuantSeries,
    pub outputs: BTreeMap<String, QuantSeries>,
}

impl FactorResult {
    pub fn new(factor_name: impl Into<String>, series: QuantSeries) -> Self {
        Self {
            factor_name: factor_name.into(),
            series,
            outputs: BTreeMap::new(),
        }
    }

    pub fn with_output(mut self, name: impl Into<String>, series: QuantSeries) -> Self {
        self.outputs.insert(name.into(), series);
        self
    }

    pub fn output(&self, name: &str) -> Option<&QuantSeries> {
        self.outputs.get(name)
    }
}
