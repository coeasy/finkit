//! Repository-level conformance vector for the language-neutral engine APIs.
//!
//! Every official binding is a transport adapter over these same contracts.
//! Keeping one checked-in vector here gives binding-host tests a stable request
//! and expected-result source instead of duplicating numeric examples.

#[cfg(test)]
mod tests {
    use crate::{evaluate_composite_json, evaluate_factor_json, evaluate_formula_json};
    use serde_json::Value;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/contracts/engine_contract_v1.json"
        )))
        .expect("engine contract fixture must be valid JSON")
    }

    #[test]
    fn formula_factor_and_composite_share_one_contract_vector() {
        let fixture = fixture();
        assert_eq!(fixture["schema_version"], 1);

        let formula = &fixture["formula"];
        let series = |name: &str| {
            formula[name]
                .as_array()
                .expect("formula fixture series must be arrays")
                .iter()
                .map(|value| {
                    value
                        .as_f64()
                        .expect("formula fixture value must be numeric")
                })
                .collect::<Vec<_>>()
        };
        let formula_payload: Value = serde_json::from_str(
            &evaluate_formula_json(
                formula["source"].as_str().unwrap(),
                formula["dialect"].as_str().unwrap(),
                &series("open"),
                &series("high"),
                &series("low"),
                &series("close"),
                &series("volume"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            formula_payload["values"]["__PRIMARY__"],
            formula["expected_primary"]
        );

        let factor_request = serde_json::to_string(&fixture["factor"]["request"]).unwrap();
        let factor_payload: Value =
            serde_json::from_str(&evaluate_factor_json(&factor_request).unwrap()).unwrap();
        assert_eq!(
            factor_payload["values"]["momentum_5"],
            fixture["factor"]["expected_primary"]
        );

        let composite_request = serde_json::to_string(&fixture["composite"]["request"]).unwrap();
        let composite_payload: Value =
            serde_json::from_str(&evaluate_composite_json(&composite_request).unwrap()).unwrap();
        assert_eq!(
            composite_payload["values"]["sma3"],
            fixture["composite"]["expected_primary"]
        );
    }

    #[test]
    fn fixture_is_transport_neutral() {
        let fixture = fixture();
        assert!(fixture["factor"]["request"]["inputs"].is_object());
        assert!(fixture["composite"]["request"]["definitions"].is_array());
        assert_eq!(fixture["formula"]["dialect"], "tdx");
    }
}
