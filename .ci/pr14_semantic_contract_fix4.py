from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, got {count}")
    return text.replace(old, new, 1)


# fix3 intentionally runs before this script and stages the final ADX body.
# Keep the body independent of ndarray's s! macro so the Formula function
# does not gain a hidden import dependency just for a small seed window.
functions_path = Path("core/src/formula/functions.rs")
functions = functions_path.read_text()
functions = replace_once(
    functions,
    '''    if seed_end >= data_len || dx.slice(s![first_valid..=seed_end]).iter().any(|v| !v.is_finite()) {
        return Ok(output);
    }

    let seed = dx
        .slice(s![first_valid..=seed_end])
        .iter()
        .copied()
        .sum::<f64>()
        / adx_n as f64;''',
    '''    if seed_end >= data_len || (first_valid..=seed_end).any(|i| !dx[i].is_finite()) {
        return Ok(output);
    }

    let seed = (first_valid..=seed_end).map(|i| dx[i]).sum::<f64>() / adx_n as f64;''',
    "ADX seed window without ndarray s macro",
)
functions_path.write_text(functions)


# This typo made every Polars integration test silently disappear because the
# declared feature is finkit-polars, not fta-polars.  Activate the real gate.
polars_test_path = Path("core/tests/polars_integration_tests.rs")
polars_test = polars_test_path.read_text()
polars_test = replace_once(
    polars_test,
    '#![cfg(feature = "fta-polars")]',
    '#![cfg(feature = "finkit-polars")]',
    "Polars integration feature gate",
)
polars_test_path.write_text(polars_test)

print("ADX hardening and Polars test activation staged")
