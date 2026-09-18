//! Executable compatibility-boundary contract.
//!
//! A parser success must not silently become a claim of complete terminal
//! compatibility. This fixture locks the diagnostic status for host data,
//! drawing, control flow, and the supported Pine subset.

use finkit::formula::{inspect_formula_compatibility, CompatibilityStatus, FormulaTerminal};
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/contracts/formula_compatibility_boundary_v1.json"
    )))
    .expect("compatibility boundary contract must be valid JSON")
}

fn status(value: &Value) -> CompatibilityStatus {
    match value.as_str().expect("status must be a string") {
        "exact" => CompatibilityStatus::Exact,
        "near" => CompatibilityStatus::Near,
        "approximate" => CompatibilityStatus::Approximate,
        "host_required" => CompatibilityStatus::HostRequired,
        "unsupported" => CompatibilityStatus::Unsupported,
        other => panic!("unknown compatibility status: {other}"),
    }
}

#[test]
fn compatibility_boundary_contract_reports_observed_limits() {
    let fixture = fixture();
    assert_eq!(fixture["schema_version"], 1);
    assert_eq!(fixture["contract"], "formula.compatibility-boundary.v1");

    for case in fixture["cases"].as_array().expect("cases must be an array") {
        let terminal = FormulaTerminal::from_str(
            case["terminal"]
                .as_str()
                .expect("terminal must be a string"),
        )
        .expect("contract terminal must be supported");
        let report = inspect_formula_compatibility(
            case["source"].as_str().expect("source must be a string"),
            terminal,
        )
        .unwrap_or_else(|error| panic!("{} failed: {error}", case["id"]));

        for expected in case["expected_capabilities"]
            .as_array()
            .expect("expected_capabilities must be an array")
        {
            let name = expected["name"].as_str().expect("capability name");
            let actual = report
                .capabilities
                .iter()
                .find(|capability| capability.name == name)
                .unwrap_or_else(|| panic!("{} missing capability {name}", case["id"]));
            assert_eq!(
                actual.status,
                status(&expected["status"]),
                "{} capability {name} status",
                case["id"]
            );
            assert_eq!(
                actual.supported,
                expected["supported"]
                    .as_bool()
                    .expect("supported must be bool"),
                "{} capability {name} supported",
                case["id"]
            );
            assert_eq!(
                actual.observed,
                expected["observed"]
                    .as_bool()
                    .expect("observed must be bool"),
                "{} capability {name} observed",
                case["id"]
            );
        }
    }
}
