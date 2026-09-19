//! MACD factor factory.

use crate::factory::{
    positive_usize, reject_unknown, DynFactor, FactorFactoryError, FactorProvider,
};
use finkit_factor::Macd;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct MacdFactory;

impl FactorProvider for MacdFactory {
    fn name(&self) -> &str {
        "MACD"
    }

    fn create(&self, params: &HashMap<String, String>) -> Result<DynFactor, FactorFactoryError> {
        reject_unknown("MACD", params, &["fast", "slow", "signal"])?;
        let fast = positive_usize("MACD", params, "fast", 12)?;
        let slow = positive_usize("MACD", params, "slow", 26)?;
        let signal = positive_usize("MACD", params, "signal", 9)?;
        if fast >= slow {
            return Err(FactorFactoryError::InvalidParameter {
                factor: "MACD".to_string(),
                name: "fast".to_string(),
                value: fast.to_string(),
                reason: "must be less than slow".to_string(),
            });
        }
        Ok(Box::new(Macd::new(fast, slow, signal)))
    }
}
