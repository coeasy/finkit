//! SMA factor factory.

use crate::factory::{
    positive_usize, reject_unknown, DynFactor, FactorFactoryError, FactorProvider,
};
use finkit_factor::Sma;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct SmaFactory;

impl FactorProvider for SmaFactory {
    fn name(&self) -> &str {
        "SMA"
    }

    fn create(&self, params: &HashMap<String, String>) -> Result<DynFactor, FactorFactoryError> {
        reject_unknown("SMA", params, &["period"])?;
        let period = positive_usize("SMA", params, "period", 20)?;
        Ok(Box::new(Sma::new(period)))
    }
}
