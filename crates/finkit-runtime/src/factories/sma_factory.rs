//! SMA factor factory.

use std::collections::HashMap;
use crate::factory::FactorFactory;

#[derive(Clone, Debug, Default)]
pub struct SmaFactory;

impl FactorFactory for SmaFactory {
    fn name(&self) -> &str {
        "SMA"
    }

    fn create(&self, params: &HashMap<String, String>) -> String {
        let period = params
            .get("period")
            .cloned()
            .unwrap_or_else(|| "20".to_string());

        format!("SMA(period={period})")
    }
}
