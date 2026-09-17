//! EMA factor factory.

use crate::factory::FactorFactory;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct EmaFactory;

impl FactorFactory for EmaFactory {
    fn name(&self) -> &str {
        "EMA"
    }

    fn create(&self, params: &HashMap<String, String>) -> String {
        let period = params
            .get("period")
            .cloned()
            .unwrap_or_else(|| "20".to_string());

        format!("EMA(period={period})")
    }
}
