use assert_cmd::Command;
use std::fs;

#[test]
fn factor_panel_cli_sorts_axes_and_writes_long_csv() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let input = temp.path().join("panel.csv");
    let output = temp.path().join("result.csv");
    fs::write(
        &input,
        concat!(
            "timestamp,symbol,close\n",
            "2,BBB,20\n",
            "1,AAA,1\n",
            "1,BBB,10\n",
            "2,AAA,2\n",
        ),
    )
    .expect("write panel input");

    let run = Command::cargo_bin("finkit-cli")
        .expect("finkit binary")
        .args([
            "factor-panel",
            "--input",
            input.to_str().unwrap(),
            "--expr",
            "RANK(CLOSE)",
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .expect("run factor panel CLI");
    assert!(
        run.status.success(),
        "panel evaluation failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let result = fs::read_to_string(output).expect("read panel result");
    assert_eq!(
        result,
        concat!(
            "timestamp,symbol,value\n",
            "1,AAA,0\n",
            "1,BBB,1\n",
            "2,AAA,0\n",
            "2,BBB,1\n",
        )
    );
}
