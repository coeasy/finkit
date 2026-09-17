//! RSI factor factory.

use std::collections::HashMap;
use crate::factory::FactorFactory;

#[derive(Clone, Debug, Default)]
pub struct RsiFactory;

impl FactorFactory for RsiFactory {
    fn name(&self) -> &str {
        "RSI"
    }

    fn create(&self, params: &HashMap<String, String>) -> String {
        let period = params
            .get("period")
            .cloned()
            .unwrap_or_else(|| "14".to_string());

        format!("RSI(period={period})")
    }
}
