//! MACD factor factory.

use crate::factory::FactorFactory;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct MacdFactory;

impl FactorFactory for MacdFactory {
    fn name(&self) -> &str {
        "MACD"
    }

    fn create(&self, params: &HashMap<String, String>) -> String {
        let fast = params.get("fast").cloned().unwrap_or_else(|| "12".into());
        let slow = params.get("slow").cloned().unwrap_or_else(|| "26".into());
        let signal = params.get("signal").cloned().unwrap_or_else(|| "9".into());
        format!("MACD(fast={fast},slow={slow},signal={signal})")
    }
}
