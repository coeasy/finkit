//! Layer 1 — native golden contract gate (V4 plan §10.5).
//!
//! Executes all 201 TA-Lib numeric-contract vectors directly against the
//! shared dispatcher (`execute_operation_json`) using the *same* request JSON
//! shape the C/C++ installed tests build. This proves the chain
//! `golden contract → shared runtime` is intact independently of any FFI
//! serialization layer, so a C/C++ failure can be attributed to the FFI
//! boundary rather than the shared runtime.

use serde_json::Value;

use finkit_ffi_common::execute::execute_operation_json;

const CONTRACT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/contracts/talib_numeric_contract_v1.json"
);

struct Vector {
    request: String,
    operation: String,
    expected: Value,
    atol: f64,
    rtol: f64,
}

fn load_vectors() -> Vec<Vector> {
    let payload: Value = serde_json::from_str(
        &std::fs::read_to_string(CONTRACT_PATH)
            .unwrap_or_else(|error| panic!("read contract: {error}")),
    )
    .expect("parse contract JSON");

    let inputs = &payload["inputs"];
    let semantic_profile = &payload["semantic_profile"];
    let mut vectors = Vec::new();
    for vector in payload["vectors"].as_array().expect("vectors array") {
        let request = format!(
            concat!(
                r#"{{"operation":{},"semantic_profile":{},"#,
                r#""input_order":{},"inputs":{},"params":{}}}"#
            ),
            serde_json::to_string(&vector["operation"]).expect("operation"),
            serde_json::to_string(semantic_profile).expect("semantic_profile"),
            serde_json::to_string(&vector["input_order"]).expect("input_order"),
            serde_json::to_string(inputs).expect("inputs"),
            serde_json::to_string(&vector["params"]).expect("params"),
        );
        vectors.push(Vector {
            request,
            operation: vector["operation"]
                .as_str()
                .expect("operation str")
                .to_string(),
            expected: vector["expected"].clone(),
            atol: vector["tolerance"]["atol"].as_f64().expect("atol"),
            rtol: vector["tolerance"]["rtol"].as_f64().expect("rtol"),
        });
    }
    vectors
}

fn value_matches(want: &Value, got: &Value, atol: f64, rtol: f64) -> bool {
    match (want.is_null(), got.is_null()) {
        (true, true) => true,
        (true, false) | (false, true) => false,
        (false, false) => {
            let want = want.as_f64().expect("finite expected value");
            let got = got.as_f64().expect("finite actual value");
            (got - want).abs() <= atol + rtol * want.abs()
        }
    }
}

#[test]
fn talib_numeric_contract_201_vectors_execute_natively() {
    let vectors = load_vectors();
    assert_eq!(
        vectors.len(),
        201,
        "contract must contain exactly 201 vectors"
    );

    for vector in &vectors {
        let response = execute_operation_json(&vector.request);
        let parsed: Value = serde_json::from_str(&response)
            .unwrap_or_else(|error| panic!("{}: invalid response JSON: {error}", vector.operation));

        let failure = |context: &str| -> String {
            format!(
                "{} ({context})\nrequest: {}\nresponse: {}",
                vector.operation, vector.request, response
            )
        };

        assert!(
            parsed.get("error").is_none(),
            "{}",
            failure("shared dispatcher returned an error")
        );

        let values = parsed
            .get("values")
            .and_then(Value::as_object)
            .expect(&failure("missing values envelope")[..]);
        let expected = vector
            .expected
            .as_object()
            .expect("expected outputs object");
        assert_eq!(
            values.len(),
            expected.len(),
            "{}",
            failure("output count mismatch")
        );

        for (name, wanted) in expected {
            let got = values
                .get(name)
                .unwrap_or_else(|| panic!("{}", failure(&format!("missing output {name}"))));
            let got = got
                .as_array()
                .expect(&failure("output is not an array")[..]);
            let wanted = wanted.as_array().expect("expected output is not an array");
            assert_eq!(
                got.len(),
                wanted.len(),
                "{}",
                failure(&format!("length mismatch for {name}"))
            );
            for (point, (want, have)) in wanted.iter().zip(got.iter()).enumerate() {
                assert!(
                    value_matches(want, have, vector.atol, vector.rtol),
                    "{}",
                    failure(&format!("{name}[{point}] numeric mismatch"))
                );
            }
        }
    }
}
