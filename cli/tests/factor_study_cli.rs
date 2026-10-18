use assert_cmd::Command;
use serde_json::Value;
use std::fs;

#[test]
fn factor_study_binary_runs_the_csv_to_report_workflow() {
    let temp = tempfile::tempdir().expect("temporary workspace");
    let input = temp.path().join("panel.csv");
    fs::write(
        &input,
        concat!(
            "timestamp,asset,factor,price\n",
            "1,A,1,10\n1,B,2,10\n1,C,3,10\n",
            "2,A,1,11\n2,B,2,12\n2,C,3,13\n",
            "3,A,1,12\n3,B,2,14\n3,C,3,16\n",
        ),
    )
    .expect("write panel fixture");

    let output = Command::cargo_bin("finkit-factor-study")
        .expect("factor-study binary")
        .args([
            "--input",
            input.to_str().unwrap(),
            "--periods",
            "1",
            "--quantiles",
            "3",
        ])
        .output()
        .expect("run factor-study binary");

    assert!(
        output.status.success(),
        "factor study failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("valid JSON report");
    assert_eq!(report["periods"], serde_json::json!([1]));
    assert_eq!(report["quantiles"], 3);
    assert_eq!(
        report["returns"]["factor_returns"]["1"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert!(report["performance"]["by_holding_period"]["1"].is_object());
}
