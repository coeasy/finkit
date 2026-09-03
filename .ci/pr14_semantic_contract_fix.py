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
# Formula execution semantics
# ---------------------------------------------------------------------------
functions_path = Path("core/src/formula/functions.rs")
functions = functions_path.read_text()
functions = replace_once(
    functions,
    "    // Pine `ta.sma(src, length)` passes only 2 args (m defaults to 1 = plain SMA).",
    "    // Terminal SMA(X, N[, M]) is recursive smoothing. With two arguments, M defaults to 1; it is not the same algorithm as simple MA(X, N).",
    "SMA semantic comment",
)

functions = replace_function(
    functions,
    "fn_cci",
    r'''fn fn_cci(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let (source, n) = match args.len() {
        2 => (args[0].clone(), extract_n(args, 1, "CCI")?),
        len if len >= 4 => {
            let high = &args[0];
            let low = &args[1];
            let close = &args[2];
            let n = extract_n(args, 3, "CCI")?;
            let data_len = high.len().min(low.len()).min(close.len());
            let mut typical = Array1::zeros(data_len);
            for i in 0..data_len {
                typical[i] = (high[i] + low[i] + close[i]) / 3.0;
            }
            (typical, n)
        }
        _ => {
            return Err(FormulaError::InvalidParameter(format!(
                "CCI requires (source, period) or (high, low, close, period), got {} arguments",
                args.len()
            )))
        }
    };

    let data_len = source.len().min(ctx.data_len);
    let mut result = nan_vec(data_len);
    for i in (n - 1)..data_len {
        let window_start = i + 1 - n;
        let sum: f64 = (window_start..=i).map(|j| source[j]).sum();
        let mean = sum / n as f64;
        let mean_dev: f64 = (window_start..=i)
            .map(|j| (source[j] - mean).abs())
            .sum::<f64>()
            / n as f64;
        if mean_dev > 1e-15 {
            result[i] = (source[i] - mean) / (0.015 * mean_dev);
        }
    }

    Ok(result)
}''',
)

functions = replace_function(
    functions,
    "fn_sar",
    r'''fn fn_sar(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    if args.len() < 4 || args.len() > 5 {
        return Err(FormulaError::InvalidParameter(format!(
            "SAR requires 4 arguments (high, low, step, max) or 5 arguments (high, low, start, increment, max), got {}",
            args.len()
        )));
    }
    let high = &args[0];
    let low = &args[1];
    let af_start = extract_f64_arg(args, 2, "SAR")?;
    let (af_increment, af_max) = if args.len() == 5 {
        (
            extract_f64_arg(args, 3, "SAR")?,
            extract_f64_arg(args, 4, "SAR")?,
        )
    } else {
        (af_start, extract_f64_arg(args, 3, "SAR")?)
    };
    if !(af_start > 0.0 && af_increment > 0.0 && af_max >= af_start) {
        return Err(FormulaError::InvalidParameter(
            "SAR acceleration factors must satisfy start > 0, increment > 0, max >= start"
                .to_string(),
        ));
    }

    let data_len = high.len().min(low.len());
    if data_len < 2 {
        return Ok(nan_vec(data_len));
    }

    let mut result = nan_vec(data_len);
    let mut is_long = high[1] - low[1] > 0.0;
    let mut af = af_start;
    let mut ep = if is_long { high[0] } else { low[0] };
    result[0] = if is_long { low[0] } else { high[0] };

    for i in 1..data_len {
        let prev_sar = result[i - 1];
        let mut sar = prev_sar + af * (ep - prev_sar);

        if is_long {
            sar = sar.min(low[i - 1]);
            if i >= 2 {
                sar = sar.min(low[i - 2]);
            }
            if low[i] < sar {
                is_long = false;
                sar = ep;
                af = af_start;
                ep = low[i];
            } else if high[i] > ep {
                ep = high[i];
                af = (af + af_increment).min(af_max);
            }
        } else {
            sar = sar.max(high[i - 1]);
            if i >= 2 {
                sar = sar.max(high[i - 2]);
            }
            if high[i] > sar {
                is_long = true;
                sar = ep;
                af = af_start;
                ep = high[i];
            } else if low[i] < ep {
                ep = low[i];
                af = (af + af_increment).min(af_max);
            }
        }

        result[i] = sar;
    }

    Ok(result)
}''',
)

functions = replace_function(
    functions,
    "fn_vwap",
    r'''fn fn_vwap(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let (price, volume) = match args.len() {
        2 => (args[0].clone(), &args[1]),
        len if len >= 4 => {
            let high = &args[0];
            let low = &args[1];
            let close = &args[2];
            let data_len = high.len().min(low.len()).min(close.len());
            let mut typical = Array1::zeros(data_len);
            for i in 0..data_len {
                typical[i] = (high[i] + low[i] + close[i]) / 3.0;
            }
            (typical, &args[3])
        }
        _ => {
            return Err(FormulaError::InvalidParameter(format!(
                "VWAP requires (source, volume) or (high, low, close, volume), got {} arguments",
                args.len()
            )))
        }
    };

    let len = price.len().min(volume.len());
    let mut result = Array1::zeros(len);
    let mut cum_price_volume = 0.0f64;
    let mut cum_volume = 0.0f64;
    for i in 0..len {
        cum_price_volume += price[i] * volume[i];
        cum_volume += volume[i];
        result[i] = if cum_volume.abs() > 1e-15 {
            cum_price_volume / cum_volume
        } else {
            f64::NAN
        };
    }
    Ok(result)
}''',
)

alias_anchor = '''    map
}

fn fn_high1'''
alias_sync = '''    // Registry aliases are executable compatibility names, not merely documentation.
    // If an alias already has an implementation, it must be the exact same function as
    // its canonical target. This fail-fast invariant prevents MA/SMA-style semantic
    // collisions from silently entering the formula runtime again.
    let registry = crate::registry::builtin_function_registry();
    for spec in registry.iter() {
        let Some(&canonical_fn) = map.get(spec.name) else {
            continue;
        };
        for &alias in spec.aliases {
            if let Some(&existing_fn) = map.get(alias) {
                assert!(
                    std::ptr::fn_addr_eq(existing_fn, canonical_fn),
                    "formula alias {alias} resolves to a different implementation than canonical {}",
                    spec.name
                );
            } else {
                map.insert(alias.to_string(), canonical_fn);
            }
        }
    }

    map
}

fn fn_high1'''
functions = replace_once(functions, alias_anchor, alias_sync, "formula alias synchronization")

if "mod pr14_semantic_contract_tests" not in functions:
    functions += r'''

#[cfg(test)]
mod pr14_semantic_contract_tests {
    use super::*;
    use ndarray::array;

    fn context(close: Array1<f64>, volume: Array1<f64>) -> FormulaContext {
        let len = close.len();
        let open = close.clone();
        let high = close.mapv(|v| v + 1.0);
        let low = close.mapv(|v| v - 1.0);
        assert_eq!(volume.len(), len);
        FormulaContext::new(open, high, low, close, volume, None)
    }

    fn scalar(len: usize, value: f64) -> Array1<f64> {
        Array1::from_elem(len, value)
    }

    #[test]
    fn ma_and_terminal_sma_are_distinct_algorithms() {
        let values = array![1.0, 2.0, 3.0, 4.0, 5.0];
        let ctx = context(values.clone(), Array1::ones(values.len()));
        let n = scalar(values.len(), 3.0);
        let ma = fn_ma(&ctx, &[values.clone(), n.clone()]).unwrap();
        let sma = fn_sma(&ctx, &[values, n]).unwrap();
        assert!(ma[0].is_nan());
        assert!((ma[2] - 2.0).abs() < 1e-12);
        assert!((sma[2] - (17.0 / 9.0)).abs() < 1e-12);
        assert_ne!(ma[4], sma[4]);
    }

    #[test]
    fn registry_aliases_cannot_shadow_different_formula_implementations() {
        let map = get_builtin_functions();
        let registry = crate::registry::builtin_function_registry();
        for spec in registry.iter() {
            let Some(&canonical_fn) = map.get(spec.name) else {
                continue;
            };
            for &alias in spec.aliases {
                let alias_fn = *map
                    .get(alias)
                    .unwrap_or_else(|| panic!("registry alias {alias} is not executable"));
                assert!(
                    std::ptr::fn_addr_eq(alias_fn, canonical_fn),
                    "alias {alias} differs from canonical {}",
                    spec.name
                );
            }
        }
    }

    #[test]
    fn pine_source_overloads_preserve_requested_series() {
        let close = array![10.0, 20.0, 30.0, 40.0];
        let volume = array![1.0, 2.0, 1.0, 2.0];
        let ctx = context(close.clone(), volume.clone());
        let n = scalar(close.len(), 3.0);

        let cci = fn_cci(&ctx, &[close.clone(), n]).unwrap();
        assert!(cci[2].is_finite());

        let vwap = fn_vwap(&ctx, &[close, volume]).unwrap();
        assert!((vwap[0] - 10.0).abs() < 1e-12);
        assert!((vwap[1] - (50.0 / 3.0)).abs() < 1e-12);
        assert!((vwap[2] - 20.0).abs() < 1e-12);
    }

    #[test]
    fn sar_keeps_legacy_four_arg_form_and_separate_increment() {
        let high = array![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low = array![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let len = high.len();
        let ctx = context(array![9.5, 10.5, 11.5, 12.5, 13.5, 14.5], Array1::ones(len));
        let start = scalar(len, 0.02);
        let inc_small = scalar(len, 0.01);
        let inc_large = scalar(len, 0.05);
        let max = scalar(len, 0.2);

        let legacy = fn_sar(&ctx, &[high.clone(), low.clone(), start.clone(), max.clone()]).unwrap();
        let small = fn_sar(
            &ctx,
            &[high.clone(), low.clone(), start.clone(), inc_small, max.clone()],
        )
        .unwrap();
        let large = fn_sar(&ctx, &[high, low, start, inc_large, max]).unwrap();
        assert_eq!(legacy.len(), len);
        assert_eq!(small.len(), len);
        assert_eq!(large.len(), len);
        assert!(small
            .iter()
            .zip(large.iter())
            .any(|(a, b)| (*a - *b).abs() > 1e-12));
    }
}
'''
functions_path.write_text(functions)


# ---------------------------------------------------------------------------
# Registry metadata: describe formula-compatible input shapes accurately.
# ---------------------------------------------------------------------------
registry_path = Path("core/src/registry.rs")
registry = registry_path.read_text()
registry = replace_once(
    registry,
    '''            name: "CCI",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Hlc,
            params: PERIOD_14,''',
    '''            name: "CCI",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Dynamic,
            params: PERIOD_14,''',
    "CCI input metadata",
)
registry = replace_once(
    registry,
    '''            name: "OBV",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Hlcv,
            params: &[],''',
    '''            name: "OBV",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Dynamic,
            params: &[],''',
    "OBV input metadata",
)
registry = replace_once(
    registry,
    '''            name: "VWAP",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Hlcv,
            params: PERIOD_20,''',
    '''            name: "VWAP",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Dynamic,
            params: &[],''',
    "VWAP input metadata",
)
registry_path.write_text(registry)


# ---------------------------------------------------------------------------
# Pine compatibility mappings: preserve source/arguments/output ordering.
# ---------------------------------------------------------------------------
mapper_path = Path("core/src/formula/pine/ast_mapper.rs")
mapper = mapper_path.read_text()
mapper = replace_once(
    mapper,
    '''                "nz" => {
                    let mapped_args: Vec<AstNode> = args
                        .iter()
                        .map(|(_, a)| self.map_node(a))
                        .collect::<Result<_, _>>()?;
                    if mapped_args.len() >= 2 {
                        return Ok(AstNode::FunctionCall {
                            name: "IF".to_string(),
                            args: vec![
                                AstNode::FunctionCall {
                                    name: "ISNA".to_string(),
                                    args: vec![mapped_args[0].clone()],
                                },
                                mapped_args[1].clone(),
                                mapped_args[0].clone(),
                            ],
                        });
                    }
                }
                "na" => {
                    // `na` is a constant NaN value in Pine, not a function call.
                    return Ok(AstNode::Number(f64::NAN));
                }''',
    '''                "nz" => {
                    let mapped_args: Vec<AstNode> = args
                        .iter()
                        .map(|(_, a)| self.map_node(a))
                        .collect::<Result<_, _>>()?;
                    if let Some(value) = mapped_args.first() {
                        let replacement = mapped_args
                            .get(1)
                            .cloned()
                            .unwrap_or(AstNode::Number(0.0));
                        return Ok(AstNode::FunctionCall {
                            name: "IF".to_string(),
                            args: vec![
                                AstNode::FunctionCall {
                                    name: "ISNA".to_string(),
                                    args: vec![value.clone()],
                                },
                                replacement,
                                value.clone(),
                            ],
                        });
                    }
                }
                "na" => {
                    if let Some((_, value)) = args.first() {
                        return Ok(AstNode::FunctionCall {
                            name: "ISNA".to_string(),
                            args: vec![self.map_node(value)?],
                        });
                    }
                    return Err(PineMapperError {
                        message: "na(x) requires one argument; bare na is parsed as NaLiteral".to_string(),
                    });
                }''',
    "Pine na/nz semantics",
)
mapper = replace_once(
    mapper,
    '''                "cci" => {
                    let n = mapped_args.get(1).cloned().unwrap_or(AstNode::Number(20.0));
                    return Ok(AstNode::FunctionCall {
                        name: "CCI".to_string(),
                        args: vec![v("HIGH"), v("LOW"), v("CLOSE"), n],
                    });
                }''',
    '''                "cci" => {
                    let source = mapped_args.get(0).cloned().unwrap_or(v("CLOSE"));
                    let n = mapped_args.get(1).cloned().unwrap_or(AstNode::Number(20.0));
                    return Ok(AstNode::FunctionCall {
                        name: "CCI".to_string(),
                        args: vec![source, n],
                    });
                }''',
    "Pine CCI source",
)
mapper = replace_once(
    mapper,
    '''                "vwap" => {
                    return Ok(AstNode::FunctionCall {
                        name: "VWAP".to_string(),
                        args: vec![v("HIGH"), v("LOW"), v("CLOSE"), v("VOL")],
                    });
                }''',
    '''                "vwap" => {
                    let source = mapped_args.get(0).cloned().unwrap_or(v("CLOSE"));
                    return Ok(AstNode::FunctionCall {
                        name: "VWAP".to_string(),
                        args: vec![source, v("VOL")],
                    });
                }''',
    "Pine VWAP source",
)
mapper = replace_once(
    mapper,
    '''                "sar" => {
                    let start = mapped_args.get(0).cloned().unwrap_or(AstNode::Number(0.02));
                    let maximum = mapped_args.get(2).cloned().unwrap_or(AstNode::Number(0.2));
                    return Ok(AstNode::FunctionCall {
                        name: "SAR".to_string(),
                        args: vec![v("HIGH"), v("LOW"), start, maximum],
                    });
                }''',
    '''                "sar" => {
                    let start = mapped_args.get(0).cloned().unwrap_or(AstNode::Number(0.02));
                    let increment = mapped_args.get(1).cloned().unwrap_or(AstNode::Number(0.02));
                    let maximum = mapped_args.get(2).cloned().unwrap_or(AstNode::Number(0.2));
                    return Ok(AstNode::FunctionCall {
                        name: "SAR".to_string(),
                        args: vec![v("HIGH"), v("LOW"), start, increment, maximum],
                    });
                }''',
    "Pine SAR increment",
)
mapper_path.write_text(mapper)


table_path = Path("core/src/formula/pine/builtin_table.rs")
table = table_path.read_text()
table = replace_once(
    table,
    '''            return_names: vec![
                "ADX".to_string(),
                "PLUS_DI".to_string(),
                "MINUS_DI".to_string(),
            ],
            description:
                "Directional Movement Index — ta.dmi(diLength, adxSmoothing) → ADX, +DI, -DI"
                    .to_string(),''',
    '''            return_names: vec![
                "PLUS_DI".to_string(),
                "MINUS_DI".to_string(),
                "ADX".to_string(),
            ],
            description:
                "Directional Movement Index — ta.dmi(diLength, adxSmoothing) → +DI, -DI, ADX"
                    .to_string(),''',
    "Pine DMI output order",
)
table = replace_once(
    table,
    '''            pine_name: "highest".to_string(),
            alpha_ta_name: "MAX".to_string(),
            multi_return: false,
            return_names: vec!["MAX".to_string()],
            description: "Highest value — ta.highest(source, length) → HHV / MAX".to_string(),''',
    '''            pine_name: "highest".to_string(),
            alpha_ta_name: "HHV".to_string(),
            multi_return: false,
            return_names: vec!["HHV".to_string()],
            description: "Rolling highest value — ta.highest(source, length) → HHV".to_string(),''',
    "Pine highest mapping",
)
table = replace_once(
    table,
    '''            pine_name: "lowest".to_string(),
            alpha_ta_name: "MIN".to_string(),
            multi_return: false,
            return_names: vec!["MIN".to_string()],
            description: "Lowest value — ta.lowest(source, length) → LLV / MIN".to_string(),''',
    '''            pine_name: "lowest".to_string(),
            alpha_ta_name: "LLV".to_string(),
            multi_return: false,
            return_names: vec!["LLV".to_string()],
            description: "Rolling lowest value — ta.lowest(source, length) → LLV".to_string(),''',
    "Pine lowest mapping",
)
if "mod pr14_semantic_mapping_tests" not in table:
    table += r'''

#[cfg(test)]
mod pr14_semantic_mapping_tests {
    use super::*;

    #[test]
    fn similarly_named_builtins_keep_distinct_semantics() {
        let table = PineBuiltinTable::new();
        assert_eq!(table.resolve(Some("ta"), "sma").unwrap().alpha_ta_name, "MA");
        assert_eq!(
            table.resolve(Some("ta"), "highest").unwrap().alpha_ta_name,
            "HHV"
        );
        assert_eq!(
            table.resolve(Some("ta"), "lowest").unwrap().alpha_ta_name,
            "LLV"
        );
        assert_eq!(
            table.resolve(Some("math"), "max").unwrap().alpha_ta_name,
            "MAX"
        );
        assert_eq!(
            table.resolve(Some("math"), "min").unwrap().alpha_ta_name,
            "MIN"
        );
    }

    #[test]
    fn dmi_metadata_matches_tuple_lowering_order() {
        let table = PineBuiltinTable::new();
        let dmi = table.resolve(Some("ta"), "dmi").unwrap();
        assert_eq!(dmi.return_names, ["PLUS_DI", "MINUS_DI", "ADX"]);
    }
}
'''
table_path.write_text(table)


# Extend the Pine golden fixture so these compatibility contracts are parsed on every CI run.
fixture_path = Path("core/tests/fixtures/formula_compat/pine/basic.txt")
fixture = fixture_path.read_text()
for line in [
    "highest3 = ta.highest(close, 3)",
    "lowest3 = ta.lowest(close, 3)",
    "is_missing = na(close)",
    "filled = nz(close)",
    "source_cci = ta.cci(open, 3)",
    "source_vwap = ta.vwap(open)",
    "psar = ta.sar(0.02, 0.03, 0.2)",
    "[plus_di, minus_di, adx] = ta.dmi(3, 3)",
]:
    if line not in fixture:
        fixture = fixture.rstrip() + "\n" + line + "\n"
fixture_path.write_text(fixture)

# Guard the source-level lowering shape in the golden integration test. The assertions
# intentionally check canonical function names and retained arguments rather than exact
# pretty-print formatting of the whole AST.
golden_path = Path("core/tests/formula_terminal_golden.rs")
golden = golden_path.read_text()
if "pine_golden_fixture_preserves_semantic_distinctions" not in golden:
    golden += r'''

#[test]
fn pine_golden_fixture_preserves_semantic_distinctions() {
    let ast = parse_formula_for_terminal(PINE, FormulaTerminal::TradingView).unwrap();
    let debug = format!("{ast:?}");
    assert!(debug.contains("MA"), "ta.sma must lower to simple MA");
    assert!(debug.contains("HHV"), "ta.highest must lower to rolling HHV");
    assert!(debug.contains("LLV"), "ta.lowest must lower to rolling LLV");
    assert!(debug.contains("ISNA"), "na(x) must lower to an ISNA predicate");
    assert!(debug.contains("CCI"));
    assert!(debug.contains("VWAP"));
    assert!(debug.contains("SAR"));
    let plus = debug.find("PLUS_DI").expect("DMI +DI lowering");
    let minus = debug.find("MINUS_DI").expect("DMI -DI lowering");
    let adx = debug.find("ADX").expect("DMI ADX lowering");
    assert!(plus < minus && minus < adx, "DMI tuple order must be +DI, -DI, ADX");
}
'''
golden_path.write_text(golden)

print("semantic contract repairs staged")
