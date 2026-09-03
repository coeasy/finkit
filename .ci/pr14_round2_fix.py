from pathlib import Path


def replace_exact(path: str, old: str, new: str, expected: int = 1) -> None:
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    count = text.count(old)
    if count != expected:
        raise SystemExit(f"{path}: expected {expected} occurrences, found {count}")
    file.write_text(text.replace(old, new), encoding="utf-8")


registry = Path("core/src/registry.rs")
text = registry.read_text(encoding="utf-8")
period = 'const PERIOD_REQUIRED: &[ParamSpec] = &[ParamSpec::new("period", "usize", None, Some("> 0"))];\n'
if text.count(period) != 1:
    raise SystemExit("registry.rs: PERIOD_REQUIRED marker not found")
text = text.replace(
    period,
    period
    + '''const SMA_PARAMS: &[ParamSpec] = &[\n    ParamSpec::new("period", "usize", None, Some("> 0")),\n    ParamSpec::new("m", "f64", Some("1"), Some("> 0")),\n];\n''',
    1,
)
old_spec = '''        FunctionSpec {
            name: "SMA",
            aliases: &["MA"],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
'''
new_spec = '''        FunctionSpec {
            name: "MA",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "SMA",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: SMA_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
'''
if text.count(old_spec) != 1:
    raise SystemExit("registry.rs: combined SMA/MA spec not found")
text = text.replace(old_spec, new_spec, 1)
marker = '''    #[test]
    fn aliases_resolve_case_insensitively() {
'''
regression = '''    #[test]
    fn ma_and_sma_have_distinct_canonical_contracts() {
        let registry = builtin_function_registry();
        let ma = registry.get("MA").unwrap();
        let sma = registry.get("SMA").unwrap();
        assert_eq!(ma.name, "MA");
        assert_eq!(sma.name, "SMA");
        assert_eq!(ma.lookback, LookbackSpec::PeriodMinusOne);
        assert_eq!(sma.lookback, LookbackSpec::Dynamic);
        assert_eq!(ma.params.len(), 1);
        assert_eq!(sma.params.len(), 2);
    }

'''
if text.count(marker) != 1:
    raise SystemExit("registry.rs: test insertion marker not found")
text = text.replace(marker, regression + marker, 1)
registry.write_text(text, encoding="utf-8")

replace_exact(
    "core/src/formula/functions.rs",
    "// Pine `ta.sma(src, length)` passes only 2 args (m defaults to 1 = plain SMA).",
    "// The two-argument compatibility form defaults M to 1.",
)

mapper = Path("core/src/formula/pine/ast_mapper.rs")
text = mapper.read_text(encoding="utf-8")
old = '''                "sma" => {
                    let source = mapped_args.get(0).cloned().unwrap_or(v("CLOSE"));
                    let n = mapped_args.get(1).cloned().unwrap_or(AstNode::Number(1.0));
                    return Ok(AstNode::FunctionCall {
                        name: "SMA".to_string(),
                        args: vec![source, n, AstNode::Number(1.0)],
                    });
                }
'''
new = '''                "sma" => {
                    let source = mapped_args.get(0).cloned().unwrap_or(v("CLOSE"));
                    let n = mapped_args.get(1).cloned().unwrap_or(AstNode::Number(1.0));
                    return Ok(AstNode::FunctionCall {
                        name: "MA".to_string(),
                        args: vec![source, n],
                    });
                }
'''
if text.count(old) != 1:
    raise SystemExit("ast_mapper.rs: ta.sma special mapping not found")
text = text.replace(old, new, 1)
old_test = '''        assert!(json.contains("SMA"));
        assert!(json.contains("CLOSE"));
'''
new_test = '''        assert!(json.contains("FunctionCall { name: \\"MA\\""));
        assert!(!json.contains("FunctionCall { name: \\"SMA\\""));
        assert!(json.contains("CLOSE"));
'''
if text.count(old_test) != 1:
    raise SystemExit("ast_mapper.rs: SMA mapping test baseline not found")
text = text.replace(old_test, new_test, 1)
mapper.write_text(text, encoding="utf-8")

builtin = Path("core/src/formula/pine/builtin_table.rs")
text = builtin.read_text(encoding="utf-8")
old = '''        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "sma".to_string(),
            alpha_ta_name: "SMA".to_string(),
            multi_return: false,
            return_names: vec!["SMA".to_string()],
            description: "Simple Moving Average — ta.sma(source, length) → SMA".to_string(),
        },
'''
new = '''        BuiltinMapping {
            namespace: Some("ta".to_string()),
            pine_name: "sma".to_string(),
            alpha_ta_name: "MA".to_string(),
            multi_return: false,
            return_names: vec!["MA".to_string()],
            description: "Simple Moving Average — ta.sma(source, length) → MA".to_string(),
        },
'''
if text.count(old) != 1:
    raise SystemExit("builtin_table.rs: ta.sma mapping baseline not found")
text = text.replace(old, new, 1)
builtin.write_text(text, encoding="utf-8")

compute_ir = Path("core/src/formula/compute_ir.rs")
text = compute_ir.read_text(encoding="utf-8")
old = '''                let capabilities = self.function_capabilities(name);
                if capabilities.effect.is_pure() {
                    self.add_node(
                        format!("CALL:{}", canonical_name(name)),
                        dependencies,
                        capabilities,
                    )
                } else {
                    self.add_effect(
                        format!("CALL:{}", canonical_name(name)),
                        dependencies,
                        capabilities,
                    )
                }
'''
new = '''                let (operation_name, capabilities) = self.function_metadata(name);
                if capabilities.effect.is_pure() {
                    self.add_node(format!("CALL:{operation_name}"), dependencies, capabilities)
                } else {
                    self.add_effect(format!("CALL:{operation_name}"), dependencies, capabilities)
                }
'''
if text.count(old) != 1:
    raise SystemExit("compute_ir.rs: FunctionCall lowering baseline not found")
text = text.replace(old, new, 1)
old = '''    fn function_capabilities(&self, name: &str) -> ComputeCapabilities {
        self.registry.get(name).map_or_else(
            || ComputeCapabilities {
                // Unknown/custom formula functions are deliberately conservative.
                // Once registered in the SSOT they regain precise capabilities.
                deterministic: false,
                streaming: false,
                stateful: true,
                lookback: LookbackRequirement::Dynamic,
                effect: ComputeEffect::Stateful,
            },
            ComputeCapabilities::from_function_spec,
        )
    }
'''
new = '''    fn function_metadata(&self, name: &str) -> (String, ComputeCapabilities) {
        self.registry.get(name).map_or_else(
            || {
                (
                    canonical_name(name),
                    ComputeCapabilities {
                        // Unknown/custom formula functions are deliberately conservative.
                        // Once registered in the SSOT they regain precise capabilities.
                        deterministic: false,
                        streaming: false,
                        stateful: true,
                        lookback: LookbackRequirement::Dynamic,
                        effect: ComputeEffect::Stateful,
                    },
                )
            },
            |spec| {
                (
                    canonical_name(spec.name),
                    ComputeCapabilities::from_function_spec(spec),
                )
            },
        )
    }
'''
if text.count(old) != 1:
    raise SystemExit("compute_ir.rs: function_capabilities baseline not found")
text = text.replace(old, new, 1)
marker = '''    #[test]
    fn unknown_function_is_conservative_until_registered() {
'''
regression = '''    #[test]
    fn ma_and_sma_keep_distinct_canonical_operations() {
        let ma = FormulaComputePlan::compile(&parse_formula("MA(CLOSE,5)").unwrap()).unwrap();
        let sma = FormulaComputePlan::compile(&parse_formula("SMA(CLOSE,5,1)").unwrap()).unwrap();

        let ma_node = ma.plan().node(ma.root()).unwrap();
        let sma_node = sma.plan().node(sma.root()).unwrap();
        assert_eq!(ma_node.operation, "CALL:MA");
        assert_eq!(sma_node.operation, "CALL:SMA");
        assert_ne!(ma_node.capabilities.lookback, sma_node.capabilities.lookback);
    }

'''
if text.count(marker) != 1:
    raise SystemExit("compute_ir.rs: test insertion marker not found")
text = text.replace(marker, regression + marker, 1)
compute_ir.write_text(text, encoding="utf-8")
