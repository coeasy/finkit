use assert_cmd::Command;
use serde_json::Value;

fn run_schema(args: &[&str]) -> Value {
    let output = Command::cargo_bin("finkit-schema")
        .expect("finkit-schema binary")
        .args(args)
        .output()
        .expect("run finkit-schema");
    assert!(
        output.status.success(),
        "schema command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("valid schema JSON")
}

#[test]
fn full_schema_is_machine_readable_and_versioned() {
    let schema = run_schema(&["--compact"]);
    assert_eq!(schema["schema_version"], "finkit.function.v1");
    assert!(
        schema["functions"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
}

#[test]
fn schema_cli_resolves_compatibility_aliases() {
    let schema = run_schema(&["--function", "ma", "--compact"]);
    assert_eq!(schema["schema_version"], "finkit.function.v1");
    assert_eq!(schema["function"]["name"], "SMA");
    assert_eq!(schema["function"]["effect"], "pure");
    assert_eq!(schema["function"]["lookback"], "period_minus_one");
}

#[test]
fn terminal_schema_reports_real_compatibility_strength() {
    let schema = run_schema(&["--terminals", "--compact"]);
    assert_eq!(
        schema["schema_version"],
        "finkit.formula-terminal.v1"
    );
    let terminals = schema["terminals"].as_array().expect("terminal array");

    let finkit = terminals
        .iter()
        .find(|item| item["terminal"] == "finkit")
        .expect("finkit terminal");
    assert_eq!(finkit["dialect"], "alpha_ta");
    assert_eq!(finkit["compatibility"], "native");

    let tdx = terminals
        .iter()
        .find(|item| item["terminal"] == "tdx")
        .expect("tdx terminal");
    assert_eq!(tdx["dialect"], "alpha_ta");
    assert_eq!(tdx["compatibility"], "common_subset");

    let pine = terminals
        .iter()
        .find(|item| item["terminal"] == "pine")
        .expect("pine terminal");
    assert_eq!(pine["dialect"], "pine");
    assert_eq!(pine["compatibility"], "common_subset");
}
