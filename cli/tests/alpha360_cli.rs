use assert_cmd::Command;
use serde_json::Value;
use std::fs;

#[test]
fn alpha360_cli_outputs_named_360_column_json_and_rejects_missing_vwap() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let input = temp.path().join("ohlcv-vwap.csv");
    fs::write(
        &input,
        concat!(
            "open,high,low,close,vwap,volume\n",
            "9,12,8,10,9.5,100\n",
            "10,13,9,11,10.5,200\n",
        ),
    )
    .expect("write Alpha360 input");

    let output = Command::cargo_bin("finkit-cli")
        .expect("finkit binary")
        .args([
            "features",
            "alpha360",
            "--input",
            input.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("run Alpha360 CLI");
    assert!(
        output.status.success(),
        "Alpha360 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let values: Value = serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(values.as_object().unwrap().len(), 360);
    let close0 = values["CLOSE0"].as_array().unwrap();
    let open0 = values["OPEN0"].as_array().unwrap();
    let volume0 = values["VOLUME0"].as_array().unwrap();
    assert_eq!(close0[0].as_f64(), Some(1.0));
    assert_eq!(close0[1].as_f64(), Some(1.0));
    assert_eq!(open0[0].as_f64(), Some(0.9));
    assert!((open0[1].as_f64().unwrap() - (10.0 / 11.0)).abs() < 1e-15);
    assert!(volume0
        .iter()
        .all(|value| (value.as_f64().unwrap() - 1.0).abs() < 1e-12));

    let missing_vwap = temp.path().join("ohlcv.csv");
    fs::write(&missing_vwap, "open,high,low,close,volume\n9,12,8,10,100\n")
        .expect("write incomplete input");
    let rejected = Command::cargo_bin("finkit-cli")
        .expect("finkit binary")
        .args([
            "features",
            "alpha360",
            "--input",
            missing_vwap.to_str().unwrap(),
        ])
        .output()
        .expect("run Alpha360 with missing VWAP");
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("vwap"));
}
