//! EMA factor factory.

use crate::factory::{
    positive_usize, reject_unknown, DynFactor, FactorFactoryError, FactorProvider,
};
use finkit_factor::Ema;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct EmaFactory;

impl FactorProvider for EmaFactory {
    fn name(&self) -> &str {
        "EMA"
    }

    fn create(&self, params: &HashMap<String, String>) -> Result<DynFactor, FactorFactoryError> {
        reject_unknown("EMA", params, &["period"])?;
        let period = positive_usize("EMA", params, "period", 20)?;
        Ok(Box::new(Ema::new(period)))
    }
}
