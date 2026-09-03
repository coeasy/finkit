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


# ---------------------------------------------------------------------------
# ADX/DMI: keep legacy ADX exact, but honor Pine's independent DI/ADX lengths.
# ---------------------------------------------------------------------------
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

    let high_values = high.as_slice().unwrap();
    let low_values = low.as_slice().unwrap();
    let close_values = close.as_slice().unwrap();
    let data_len = ctx.data_len;

    // Preserve the established four-argument ADX contract exactly.  Pine's
    // ta.dmi(diLength, adxSmoothing) uses the five-argument form below.
    if args.len() == 4 {
        return match lib_momentum::adx(high_values, low_values, close_values, di_n) {
            Ok(result) => Ok(result),
            Err(_) => Ok(nan_vec(data_len)),
        };
    }

    let adx_n = extract_n(args, 4, "ADX")?;
    let plus_di = match lib_momentum::plus_di(high_values, low_values, close_values, di_n) {
        Ok(result) => result,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let minus_di = match lib_momentum::minus_di(high_values, low_values, close_values, di_n) {
        Ok(result) => result,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut dx = nan_vec(data_len);
    for i in 0..data_len {
        let plus = plus_di[i];
        let minus = minus_di[i];
        if plus.is_finite() && minus.is_finite() {
            let sum = plus + minus;
            dx[i] = if sum.abs() > 1e-15 {
                (plus - minus).abs() / sum * 100.0
            } else {
                0.0
            };
        }
    }

    // Pine ta.dmi smooths DX with Wilder/RMA using adxSmoothing.  Seed the
    // recursion with the arithmetic mean of the first adx_n valid DX values,
    // then use alpha = 1/adx_n.  This keeps diLength and adxSmoothing distinct.
    let mut output = nan_vec(data_len);
    let Some(first_valid) = dx.iter().position(|value| value.is_finite()) else {
        return Ok(output);
    };
    let Some(seed_end) = first_valid.checked_add(adx_n - 1) else {
        return Ok(output);
    };
    if seed_end >= data_len || dx.slice(s![first_valid..=seed_end]).iter().any(|v| !v.is_finite()) {
        return Ok(output);
    }

    let seed = dx
        .slice(s![first_valid..=seed_end])
        .iter()
        .copied()
        .sum::<f64>()
        / adx_n as f64;
    output[seed_end] = seed;
    let mut previous = seed;
    for i in (seed_end + 1)..data_len {
        let value = dx[i];
        if value.is_finite() {
            previous = (value + (adx_n as f64 - 1.0) * previous) / adx_n as f64;
            output[i] = previous;
        }
    }

    Ok(output)
}''',
)
functions_path.write_text(functions)


# ---------------------------------------------------------------------------
# Pine mapper: do not lose/default the wrong arguments and do not reuse a
# similarly named function when the numerical contract differs.
# ---------------------------------------------------------------------------
mapper_path = Path("core/src/formula/pine/ast_mapper.rs")
mapper = mapper_path.read_text()
mapper = replace_once(
    mapper,
    '''                                name: "MINUS_DI".to_string(),
                                args: vec![cl.clone(), l1],
                            },''',
    '''                                name: "MINUS_DI".to_string(),
                                args: vec![cl.clone(), l1.clone()],
                            },''',
    "DMI keeps diLength alive for ADX",
)
mapper = replace_once(
    mapper,
    '''                            AstNode::FunctionCall {
                                name: "AROON_UP".to_string(),
                                args: vec![cl.clone(), length.clone()],
                            },
                        ),
                        assignment(
                            &names[1],
                            AstNode::FunctionCall {
                                name: "AROON_DN".to_string(),
                                args: vec![cl, length],
                            },
                        ),''',
    '''                            AstNode::FunctionCall {
                                name: "AROON_UP".to_string(),
                                args: vec![hi, length.clone()],
                            },
                        ),
                        assignment(
                            &names[1],
                            AstNode::FunctionCall {
                                name: "AROON_DN".to_string(),
                                args: vec![lo, length],
                            },
                        ),''',
    "Pine Aroon uses HIGH for up and LOW for down",
)
mapper = replace_once(
    mapper,
    '''                "stoch" => {
                    let source = mapped_args.get(0).cloned().unwrap_or(v("CLOSE"));
                    let high = mapped_args.get(1).cloned().unwrap_or(v("HIGH"));
                    let low = mapped_args.get(2).cloned().unwrap_or(v("LOW"));
                    let n = mapped_args.get(3).cloned().unwrap_or(AstNode::Number(14.0));
                    return Ok(AstNode::FunctionCall {
                        name: "STOCH".to_string(),
                        args: vec![high, low, source, n],
                    });
                }
                "sma" => {''',
    '''                "stoch" => {
                    let source = mapped_args.get(0).cloned().unwrap_or(v("CLOSE"));
                    let high = mapped_args.get(1).cloned().unwrap_or(v("HIGH"));
                    let low = mapped_args.get(2).cloned().unwrap_or(v("LOW"));
                    let n = mapped_args.get(3).cloned().unwrap_or(AstNode::Number(14.0));
                    return Ok(AstNode::FunctionCall {
                        // Pine ta.stoch is the unsmoothed stochastic value.  STOCHF
                        // with fast-D period 1 preserves that contract; generic STOCH
                        // keeps its terminal slow-K defaults independently.
                        name: "STOCHF".to_string(),
                        args: vec![high, low, source, n, AstNode::Number(1.0)],
                    });
                }
                "change" => {
                    let source = mapped_args.get(0).cloned().unwrap_or(v("CLOSE"));
                    let n = mapped_args.get(1).cloned().unwrap_or(AstNode::Number(1.0));
                    return Ok(AstNode::FunctionCall {
                        name: "MOM".to_string(),
                        args: vec![source, n],
                    });
                }
                "sma" => {''',
    "Pine stochastic/change contracts",
)
if "mod pr14_semantic_mapper_v3_tests" not in mapper:
    mapper += r'''

#[cfg(test)]
mod pr14_semantic_mapper_v3_tests {
    use super::*;
    use crate::formula::pine::parser::parse_pine;

    fn mapped(source: &str) -> String {
        let pine = parse_pine(source).unwrap();
        format!("{:?}", map_pine_to_alphata(&pine).unwrap())
    }

    #[test]
    fn pine_change_defaults_to_one_bar_momentum() {
        let debug = mapped("//@version=5\nindicator(\"C\")\nc = ta.change(close)\n");
        assert!(debug.contains("FunctionCall { name: \\\"MOM\\\""));
        assert!(debug.contains("Number(1.0)"));
    }

    #[test]
    fn pine_stoch_uses_unsmoothed_fast_k_contract() {
        let debug = mapped(
            "//@version=5\nindicator(\"S\")\ns = ta.stoch(close, high, low, 3)\n",
        );
        assert!(debug.contains("FunctionCall { name: \\\"STOCHF\\\""));
        assert!(debug.contains("Number(1.0)"));
    }

    #[test]
    fn pine_aroon_preserves_high_low_sources() {
        let debug = mapped("//@version=5\nindicator(\"A\")\n[u, d] = ta.aroon(3)\n");
        assert!(debug.contains("AROON_UP"));
        assert!(debug.contains("AROON_DN"));
        assert!(debug.contains("Variable(\\\"HIGH\\\")"));
        assert!(debug.contains("Variable(\\\"LOW\\\")"));
    }
}
'''
mapper_path.write_text(mapper)


# ---------------------------------------------------------------------------
# Pine metadata must describe the same callable/return shape as the mapper.
# ---------------------------------------------------------------------------
table_path = Path("core/src/formula/pine/builtin_table.rs")
table = table_path.read_text()
table = replace_once(
    table,
    '''        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "stoch".to_string(),
            alpha_ta_name: "STOCH".to_string(),
            multi_return: false,
            return_names: vec!["STOCH".to_string()],
            description: "Stochastic oscillator — ta.stoch(source, peak, valley, period) → STOCH".to_string(),
        },''',
    '''        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "stoch".to_string(),
            alpha_ta_name: "STOCHF".to_string(),
            multi_return: false,
            return_names: vec!["STOCHF".to_string()],
            description: "Stochastic oscillator — ta.stoch(source, peak, valley, period) → unsmoothed STOCHF K".to_string(),
        },''',
    "Pine stochastic metadata target",
)
if "mod pr14_semantic_mapping_v3_tests" not in table:
    table += r'''

#[cfg(test)]
mod pr14_semantic_mapping_v3_tests {
    use super::*;

    #[test]
    fn stochastic_metadata_matches_unsmoothed_mapper_target() {
        let table = PineBuiltinTable::new();
        let stoch = table.resolve(Some("ta"), "stoch").unwrap();
        assert!(!stoch.multi_return);
        assert_eq!(stoch.alpha_ta_name, "STOCHF");
        assert_eq!(stoch.return_names, ["STOCHF"]);
    }
}
'''
table_path.write_text(table)


# ---------------------------------------------------------------------------
# Golden fixture: exercise the corrected contracts in the real terminal path.
# ---------------------------------------------------------------------------
fixture_path = Path("core/tests/fixtures/formula_compat/pine/basic.txt")
fixture = fixture_path.read_text()
for line in [
    "change1 = ta.change(close)",
    "[aroon_up, aroon_down] = ta.aroon(3)",
    "stoch_fast = ta.stoch(close, high, low, 3)",
]:
    if line not in fixture:
        fixture = fixture.rstrip() + "\n" + line + "\n"
fixture_path.write_text(fixture)


golden_path = Path("core/tests/formula_terminal_golden.rs")
golden = golden_path.read_text()
anchor = '''    assert!(debug.contains("Number(3.0)"), "DMI lengths must survive tuple lowering");\n'''
if anchor not in golden:
    raise SystemExit("golden DMI assertion anchor not found")
golden = golden.replace(
    anchor,
    anchor
    + '''    assert!(debug.contains("STOCHF"), "ta.stoch must keep Pine fast-K semantics");\n'''
    + '''    assert!(debug.contains("AROON_UP") && debug.contains("AROON_DN"));\n'''
    + '''    assert!(debug.contains("MOM"), "ta.change must lower to one-bar momentum by default");\n''',
    1,
)
golden_path.write_text(golden)

print("extended Pine semantic contracts staged")
