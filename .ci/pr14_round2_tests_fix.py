from pathlib import Path

path = Path("core/src/formula/pine/builtin_table.rs")
text = path.read_text(encoding="utf-8")
old = '''    fn test_resolve_ta_sma() {
        let table = PineBuiltinTable::new();
        let m = table.resolve(Some("ta"), "sma").unwrap();
        assert_eq!(m.alpha_ta_name, "SMA");
    }
'''
new = '''    fn test_resolve_ta_sma() {
        let table = PineBuiltinTable::new();
        let m = table.resolve(Some("ta"), "sma").unwrap();
        assert_eq!(m.alpha_ta_name, "MA");
    }
'''
if text.count(old) != 1:
    raise SystemExit("builtin_table.rs: ta.sma resolution test baseline not found")
text = text.replace(old, new, 1)
old = '''        assert!(doc.contains("ta.sma"));
        assert!(doc.contains("SMA"));
'''
new = '''        assert!(doc.contains("ta.sma"));
        assert!(doc.contains("| ta | sma | MA |"));
'''
if text.count(old) != 1:
    raise SystemExit("builtin_table.rs: mapping doc test baseline not found")
text = text.replace(old, new, 1)
path.write_text(text, encoding="utf-8")

path = Path("core/src/schema.rs")
text = path.read_text(encoding="utf-8")
old = '''    fn schema_preserves_alias_parameters_and_compute_capabilities() {
        let schema = FunctionApiSchema::builtin();
        let sma = schema.get("SMA").unwrap();

        assert_eq!(sma.aliases, vec!["MA"]);
        assert_eq!(sma.category, "overlap");
        assert_eq!(sma.input, "series");
        assert_eq!(sma.outputs, 1);
        assert_eq!(sma.lookback, "period_minus_one");
        assert!(sma.streaming);
        assert!(sma.deterministic);
        assert!(!sma.stateful);
        assert_eq!(sma.effect, "pure");
        assert_eq!(sma.params[0].name, "period");
    }
'''
new = '''    fn schema_preserves_parameters_and_compute_capabilities() {
        let schema = FunctionApiSchema::builtin();
        let ma = schema.get("MA").unwrap();
        let sma = schema.get("SMA").unwrap();

        assert!(ma.aliases.is_empty());
        assert_eq!(ma.category, "overlap");
        assert_eq!(ma.input, "series");
        assert_eq!(ma.outputs, 1);
        assert_eq!(ma.lookback, "period_minus_one");
        assert!(ma.streaming);
        assert!(ma.deterministic);
        assert!(!ma.stateful);
        assert_eq!(ma.effect, "pure");
        assert_eq!(ma.params.len(), 1);
        assert_eq!(ma.params[0].name, "period");

        assert!(sma.aliases.is_empty());
        assert_eq!(sma.lookback, "dynamic");
        assert_eq!(sma.params.len(), 2);
        assert_eq!(sma.params[0].name, "period");
        assert_eq!(sma.params[1].name, "m");
        assert_eq!(sma.params[1].default.as_deref(), Some("1"));
    }
'''
if text.count(old) != 1:
    raise SystemExit("schema.rs: combined MA/SMA schema contract test not found")
text = text.replace(old, new, 1)
old = '''    fn schema_lookup_resolves_aliases_case_insensitively() {
        let schema = FunctionApiSchema::builtin();
        assert_eq!(schema.get("ma").unwrap().name, "SMA");
        assert_eq!(schema.get("boll").unwrap().name, "BBANDS");
        assert!(schema.get("not-a-function").is_none());
    }
'''
new = '''    fn schema_lookup_resolves_names_and_aliases_case_insensitively() {
        let schema = FunctionApiSchema::builtin();
        assert_eq!(schema.get("ma").unwrap().name, "MA");
        assert_eq!(schema.get("sma").unwrap().name, "SMA");
        assert_eq!(schema.get("boll").unwrap().name, "BBANDS");
        assert!(schema.get("not-a-function").is_none());
    }
'''
if text.count(old) != 1:
    raise SystemExit("schema.rs: alias lookup test not found")
text = text.replace(old, new, 1)
path.write_text(text, encoding="utf-8")
