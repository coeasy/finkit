//! RSI factor factory.

use crate::factory::{
    positive_usize, reject_unknown, DynFactor, FactorFactoryError, FactorProvider,
};
use finkit_factor::Rsi;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct RsiFactory;

impl FactorProvider for RsiFactory {
    fn name(&self) -> &str {
        "RSI"
    }

    fn create(&self, params: &HashMap<String, String>) -> Result<DynFactor, FactorFactoryError> {
        reject_unknown("RSI", params, &["period"])?;
        let period = positive_usize("RSI", params, "period", 14)?;
        Ok(Box::new(Rsi::new(period)))
    }
}
