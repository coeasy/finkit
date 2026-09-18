//! Repository-level conformance vector for the language-neutral engine APIs.
//!
//! Every official binding is a transport adapter over these same contracts.
//! Keeping one checked-in vector here gives binding-host tests a stable request
//! and expected-result source instead of duplicating numeric examples.

#[cfg(test)]
mod tests {
    use crate::{
        evaluate_composite_json, evaluate_composite_stream_json,
        evaluate_factor_cross_sectional_json, evaluate_factor_json, evaluate_factor_stream_json,
        evaluate_formula_cross_sectional_json, evaluate_formula_json, evaluate_formula_panel_json,
        evaluate_formula_stream_json, evaluate_formula_temporal_json,
    };
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;

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

        let formula_stateful = &fixture["formula_stateful_stream"];
        let formula_first: Value = serde_json::from_str(
            &evaluate_formula_stream_json(&formula_stateful["request"].to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            formula_first["values"]["__PRIMARY__"],
            formula_stateful["expected_primary"]
        );
        let formula_second_request = serde_json::json!({
            "schema_version": 1,
            "source": formula_stateful["request"]["source"].clone(),
            "dialect": formula_stateful["request"]["dialect"].clone(),
            "inputs": formula_stateful["next_inputs"].clone(),
            "checkpoint": formula_first["checkpoint"].clone(),
        });
        let formula_second: Value = serde_json::from_str(
            &evaluate_formula_stream_json(&formula_second_request.to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            formula_second["values"]["__PRIMARY__"],
            formula_stateful["expected_next"]
        );
        assert_eq!(
            formula_second["execution"]["total_rows"],
            formula_stateful["expected_total_rows"]
        );

        let formula_program = &fixture["formula_stateful_program"];
        let formula_program_payload: Value = serde_json::from_str(
            &evaluate_formula_stream_json(&formula_program["request"].to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            formula_program_payload["values"]["__PRIMARY__"],
            formula_program["expected_primary"]
        );

        let formula_loop = &fixture["formula_stateful_bounded_loop"];
        let formula_loop_payload: Value = serde_json::from_str(
            &evaluate_formula_stream_json(&formula_loop["request"].to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            formula_loop_payload["values"]["__PRIMARY__"],
            formula_loop["expected_primary"]
        );
    }

    #[test]
    fn formula_temporal_contract_matches_shared_fixture() {
        let fixture: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/contracts/formula_temporal_contract_v1.json"
        )))
        .expect("formula temporal contract fixture must be valid JSON");
        assert_eq!(fixture["schema_version"], 1);
        let payload: Value = serde_json::from_str(
            &evaluate_formula_temporal_json(&fixture["request"].to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(payload["contract"], fixture["expected"]["contract"]);
        assert_eq!(payload["frame"], fixture["expected"]["frame"]);
        assert_eq!(
            payload["values"]["__PRIMARY__"],
            fixture["expected"]["primary"]
        );
    }

    #[test]
    fn formula_pine_security_contract_matches_shared_fixture() {
        let fixture: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/contracts/formula_pine_security_contract_v1.json"
        )))
        .expect("Pine security contract fixture must be valid JSON");
        assert_eq!(fixture["schema_version"], 1);
        let payload: Value = serde_json::from_str(
            &evaluate_formula_temporal_json(&fixture["request"].to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(payload["contract"], fixture["expected"]["contract"]);
        assert_eq!(payload["dialect"], fixture["expected"]["dialect"]);
        assert_eq!(
            payload["values"]["__PRIMARY__"],
            fixture["expected"]["primary"]
        );
    }

    #[test]
    fn formula_panel_contract_matches_shared_fixture() {
        let fixture: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/contracts/formula_panel_contract_v1.json"
        )))
        .expect("formula panel contract fixture must be valid JSON");
        let payload: Value = serde_json::from_str(
            &evaluate_formula_panel_json(&fixture["request"].to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(payload["contract"], fixture["expected"]["contract"]);
        let actual = payload["frames"].as_array().unwrap();
        let expected = fixture["expected"]["frames"].as_array().unwrap();
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(expected) {
            assert_eq!(actual["symbol"], expected["symbol"]);
            assert_eq!(actual["timeframe"], expected["timeframe"]);
            assert_eq!(actual["values"]["__PRIMARY__"], expected["primary"]);
        }
    }

    #[test]
    fn cross_sectional_factor_contract_matches_shared_fixture() {
        let fixture: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/contracts/factor_cross_sectional_contract_v1.json"
        )))
        .expect("cross-sectional factor contract fixture must be valid JSON");
        let payload: Value = serde_json::from_str(
            &evaluate_factor_cross_sectional_json(&fixture["request"].to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(payload["contract"], fixture["expected"]["contract"]);
        assert_eq!(payload["target"], fixture["expected"]["target"]);
        assert_eq!(payload["timestamps"], fixture["expected"]["timestamps"]);
        assert_eq!(payload["symbols"], fixture["expected"]["symbols"]);
        assert_eq!(
            payload["values"]["cross_rank"],
            fixture["expected"]["primary"]
        );
    }

    #[test]
    fn cross_sectional_formula_contract_matches_shared_fixture() {
        let fixture: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/contracts/formula_cross_sectional_contract_v1.json"
        )))
        .expect("cross-sectional formula contract fixture must be valid JSON");
        let payload: Value = serde_json::from_str(
            &evaluate_formula_cross_sectional_json(&fixture["request"].to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(payload["contract"], fixture["expected"]["contract"]);
        assert_eq!(
            payload["values"]["__PRIMARY__"],
            fixture["expected"]["primary"]
        );
    }

    #[test]
    fn factor_and_composite_range_vectors_share_execution_contract() {
        let fixture = fixture();
        for (section, execute, output_name) in [
            (
                "factor_range",
                evaluate_factor_json as fn(&str) -> Result<String, String>,
                "momentum_5",
            ),
            (
                "composite_range",
                evaluate_composite_json as fn(&str) -> Result<String, String>,
                "sma3",
            ),
        ] {
            let request = serde_json::to_string(&fixture[section]["request"]).unwrap();
            let payload: Value = serde_json::from_str(&execute(&request).unwrap()).unwrap();
            assert_eq!(payload["execution"]["mode"], "range");
            assert_eq!(
                payload["execution"]["input_dirty"],
                fixture[section]["expected_execution"]["input_dirty"]
            );
            assert_eq!(
                payload["execution"]["affected"],
                fixture[section]["expected_execution"]["affected"]
            );
            let actual = payload["values"][output_name].as_array().unwrap();
            let expected = fixture[section]["expected_primary"].as_array().unwrap();
            assert_eq!(actual.len(), expected.len());
            for (actual, expected) in actual.iter().zip(expected) {
                let actual = actual.as_f64().unwrap();
                let expected = expected.as_f64().unwrap();
                assert!((actual - expected).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn factor_and_composite_stream_vectors_share_checkpoint_contract() {
        let fixture = fixture();

        let factor = &fixture["factor_stream"];
        let factor_first: Value = serde_json::from_str(
            &evaluate_factor_stream_json(&factor["request"].to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            factor_first["values"]["momentum_5"],
            factor["expected_primary"]
        );
        let factor_second_request = serde_json::json!({
            "schema_version": 1,
            "targets": ["momentum_5"],
            "inputs": factor["next_inputs"].clone(),
            "checkpoint": factor_first["checkpoint"].clone(),
        });
        let factor_second: Value = serde_json::from_str(
            &evaluate_factor_stream_json(&factor_second_request.to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            factor_second["values"]["momentum_5"],
            factor["expected_next"]
        );
        assert_eq!(
            factor_second["execution"]["total_rows"],
            factor["expected_total_rows"]
        );

        let stateful_factor = &fixture["factor_stateful_stream"];
        let stateful_factor_first: Value = serde_json::from_str(
            &evaluate_factor_stream_json(&stateful_factor["request"].to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            stateful_factor_first["values"]["momentum_5"],
            stateful_factor["expected_primary"]
        );
        assert_eq!(
            stateful_factor_first["execution"]["mode"],
            "stateful_streaming"
        );
        let stateful_factor_second_request = serde_json::json!({
            "schema_version": 1,
            "mode": "stateful",
            "targets": ["momentum_5"],
            "inputs": stateful_factor["next_inputs"].clone(),
            "checkpoint": stateful_factor_first["checkpoint"].clone(),
        });
        let stateful_factor_second: Value = serde_json::from_str(
            &evaluate_factor_stream_json(&stateful_factor_second_request.to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            stateful_factor_second["values"]["momentum_5"],
            stateful_factor["expected_next"]
        );
        assert_eq!(
            stateful_factor_second["execution"]["total_rows"],
            stateful_factor["expected_total_rows"]
        );

        let composite = &fixture["composite_stream"];
        let composite_first: Value = serde_json::from_str(
            &evaluate_composite_stream_json(&composite["request"].to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            composite_first["values"]["sma3"],
            composite["expected_primary"]
        );
        let composite_second_request = serde_json::json!({
            "schema_version": 1,
            "inputs": composite["next_inputs"].clone(),
            "definitions": composite["request"]["definitions"].clone(),
            "outputs": ["sma3"],
            "checkpoint": composite_first["checkpoint"].clone(),
        });
        let composite_second: Value = serde_json::from_str(
            &evaluate_composite_stream_json(&composite_second_request.to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            composite_second["values"]["sma3"],
            composite["expected_next"]
        );
        assert_eq!(
            composite_second["execution"]["total_rows"],
            composite["expected_total_rows"]
        );

        let stateful = &fixture["composite_stateful_stream"];
        let stateful_first: Value = serde_json::from_str(
            &evaluate_composite_stream_json(&stateful["request"].to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            stateful_first["values"]["ema"],
            stateful["expected_primary"]
        );
        assert_eq!(stateful_first["execution"]["mode"], "stateful_streaming");
        let stateful_second_request = serde_json::json!({
            "schema_version": 1,
            "mode": "stateful",
            "inputs": stateful["next_inputs"].clone(),
            "definitions": stateful["request"]["definitions"].clone(),
            "outputs": ["ema"],
            "checkpoint": stateful_first["checkpoint"].clone(),
        });
        let stateful_second: Value = serde_json::from_str(
            &evaluate_composite_stream_json(&stateful_second_request.to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(stateful_second["values"]["ema"], stateful["expected_next"]);
        assert_eq!(
            stateful_second["execution"]["total_rows"],
            stateful["expected_total_rows"]
        );
    }

    #[test]
    fn fixture_is_transport_neutral() {
        let fixture = fixture();
        assert!(fixture["factor"]["request"]["inputs"].is_object());
        assert!(fixture["composite"]["request"]["definitions"].is_array());
        assert_eq!(fixture["formula"]["dialect"], "tdx");
    }

    #[test]
    fn all_public_language_surfaces_expose_the_same_contract_capabilities() {
        let fixture: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/contracts/language_api_surface_v1.json"
        )))
        .expect("language API surface fixture must be valid JSON");
        assert_eq!(fixture["schema_version"], 1);
        let capabilities = fixture["capabilities"]
            .as_array()
            .expect("capabilities must be an array");
        let languages = fixture["languages"]
            .as_object()
            .expect("languages must be an object");
        assert_eq!(languages.len(), 8);

        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        for (language, surface) in languages {
            let entries = surface["surfaces"]
                .as_array()
                .unwrap_or_else(|| panic!("{language} surfaces must be an array"));
            assert_eq!(
                entries.len(),
                capabilities.len(),
                "{language} capability count"
            );
            for entry in entries {
                let path = entry["path"]
                    .as_str()
                    .expect("surface path must be a string");
                let marker = entry["marker"]
                    .as_str()
                    .expect("surface marker must be a string");
                let source = fs::read_to_string(repo_root.join(path))
                    .unwrap_or_else(|error| panic!("{language} surface {path}: {error}"));
                assert!(
                    source.contains(marker),
                    "{language} surface {path} is missing marker {marker}"
                );
            }
        }
    }
}
