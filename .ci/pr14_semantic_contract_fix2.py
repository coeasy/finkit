from pathlib import Path
import re


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, got {count}")
    return text.replace(old, new, 1)


def replace_function(text: str, name: str, new_source: str) -> str:
    pattern = re.compile(rf"(?ms)^fn {re.escape(name)}\(.*?(?=^fn [A-Za-z0-9_]+\()")
    matches = list(pattern.finditer(text))
    if len(matches) != 1:
        raise SystemExit(f"{name}: expected exactly one function body, got {len(matches)}")
    match = matches[0]
    return text[: match.start()] + new_source.rstrip() + "\n\n" + text[match.end() :]


functions_path = Path("core/src/formula/functions.rs")
functions = functions_path.read_text()
functions = replace_function(
    functions,
    "fn_adx",
    r'''fn fn_adx(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ADX", args, 4)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let di_n = extract_n(args, 3, "ADX")?;
    let adx_n = if args.len() > 4 {
        extract_n(args, 4, "ADX")?
    } else {
        di_n
    };

    let data_len = ctx.data_len;
    let mut plus_dm = Array1::zeros(data_len);
    let mut minus_dm = Array1::zeros(data_len);
    let mut tr = Array1::zeros(data_len);

    if data_len == 0 {
        return Ok(nan_vec(0));
    }

    tr[0] = high[0] - low[0];
    for i in 1..data_len {
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];
        plus_dm[i] = if up_move > down_move && up_move > 0.0 {
            up_move
        } else {
            0.0
        };
        minus_dm[i] = if down_move > up_move && down_move > 0.0 {
            down_move
        } else {
            0.0
        };
        let hl = high[i] - low[i];
        let hc = (high[i] - close[i - 1]).abs();
        let lc = (low[i] - close[i - 1]).abs();
        tr[i] = hl.max(hc).max(lc);
    }

    let atr_vals = match lib_ma::sma(tr.as_slice().unwrap(), di_n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let apdm_vals = match lib_ma::sma(plus_dm.as_slice().unwrap(), di_n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let amdm_vals = match lib_ma::sma(minus_dm.as_slice().unwrap(), di_n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut dx = nan_vec(data_len);
    for i in 0..data_len {
        if !atr_vals[i].is_nan() && atr_vals[i].abs() > 1e-15 {
            let pdi = apdm_vals[i] / atr_vals[i] * 100.0;
            let mdi = amdm_vals[i] / atr_vals[i] * 100.0;
            let sum = pdi + mdi;
            if sum.abs() > 1e-15 {
                dx[i] = (pdi - mdi).abs() / sum * 100.0;
            }
        }
    }

    let dx_vec: Vec<f64> = dx
        .iter()
        .map(|&v| if v.is_nan() { 0.0 } else { v })
        .collect();
    match lib_ma::sma(&dx_vec, adx_n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}''',
)

# Extend the semantic regression module created by fix1.
anchor = '''    #[test]\n    fn sar_keeps_legacy_four_arg_form_and_separate_increment() {'''
if anchor not in functions:
    raise SystemExit("semantic test module anchor not found")
insert = r'''    #[test]
    fn adx_keeps_di_length_distinct_from_adx_smoothing() {
        let close = ndarray::array![10.0, 10.5, 11.0, 10.7, 11.6, 12.2, 11.9, 12.8, 13.4, 13.1];
        let volume = Array1::ones(close.len());
        let ctx = context(close.clone(), volume);
        let high = close.mapv(|v| v + 0.8);
        let low = close.mapv(|v| v - 0.6);
        let len = close.len();
        let di3 = scalar(len, 3.0);
        let adx2 = scalar(len, 2.0);
        let adx5 = scalar(len, 5.0);
        let fast = fn_adx(
            &ctx,
            &[high.clone(), low.clone(), close.clone(), di3.clone(), adx2],
        )
        .unwrap();
        let slow = fn_adx(&ctx, &[high, low, close, di3, adx5]).unwrap();
        assert!(fast
            .iter()
            .zip(slow.iter())
            .any(|(a, b)| a.is_finite() && b.is_finite() && (*a - *b).abs() > 1e-12));
    }

'''
functions = functions.replace(anchor, insert + anchor, 1)
functions_path.write_text(functions)


mapper_path = Path("core/src/formula/pine/ast_mapper.rs")
mapper = mapper_path.read_text()
mapper = replace_once(
    mapper,
    '''                            AstNode::FunctionCall {
                                name: "ADX".to_string(),
                                args: vec![hi, lo, cl, l2],
                            },''',
    '''                            AstNode::FunctionCall {
                                name: "ADX".to_string(),
                                args: vec![hi, lo, cl, l1, l2],
                            },''',
    "Pine DMI preserves both lengths",
)
mapper_path.write_text(mapper)


table_path = Path("core/src/formula/pine/builtin_table.rs")
table = table_path.read_text()
table = replace_once(
    table,
    '''            pine_name: "stoch".to_string(),
            alpha_ta_name: "STOCH".to_string(),
            multi_return: true,
            return_names: vec!["K".to_string(), "D".to_string()],
            description: "Stochastic / KDJ — ta.stoch(...) → K, D (maps to KDJ/STOCH)".to_string(),''',
    '''            pine_name: "stoch".to_string(),
            alpha_ta_name: "STOCH".to_string(),
            multi_return: false,
            return_names: vec!["STOCH".to_string()],
            description: "Stochastic oscillator — ta.stoch(source, peak, valley, period) → STOCH".to_string(),''',
    "Pine stochastic return shape",
)
# Extend the test module created by fix1.
test_anchor = '''    #[test]\n    fn dmi_metadata_matches_tuple_lowering_order() {'''
if test_anchor not in table:
    raise SystemExit("mapping test anchor not found")
extra = r'''    #[test]
    fn stochastic_metadata_matches_single_series_lowering() {
        let table = PineBuiltinTable::new();
        let stoch = table.resolve(Some("ta"), "stoch").unwrap();
        assert!(!stoch.multi_return);
        assert_eq!(stoch.return_names, ["STOCH"]);
    }

'''
table = table.replace(test_anchor, extra + test_anchor, 1)
table_path.write_text(table)


golden_path = Path("core/tests/formula_terminal_golden.rs")
golden = golden_path.read_text()
# Existing DMI source fixture already carries two distinct lengths through parse/mapping.
# Lock the ADX call shape by ensuring both literals survive in the mapped AST.
needle = '''    assert!(debug.contains("SAR"));\n'''
if needle not in golden:
    raise SystemExit("golden semantic assertion anchor not found")
golden = golden.replace(
    needle,
    needle + '''    assert!(debug.contains("Number(3.0)"), "DMI lengths must survive tuple lowering");\n''',
    1,
)
golden_path.write_text(golden)

print("extended DMI/stochastic semantic repairs staged")
