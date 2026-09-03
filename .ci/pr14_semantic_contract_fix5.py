from pathlib import Path
import re


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, got {count}")
    return text.replace(old, new, 1)


def replace_test_function(text: str, name: str, new_source: str) -> str:
    pattern = re.compile(
        rf"(?ms)^    #\[test\]\n    fn {re.escape(name)}\(\) \{{.*?^    \}}\n"
    )
    matches = list(pattern.finditer(text))
    if len(matches) != 1:
        raise SystemExit(f"{name}: expected exactly one test function, got {len(matches)}")
    match = matches[0]
    return text[: match.start()] + new_source.rstrip() + "\n" + text[match.end() :]


mapper_path = Path("core/src/formula/pine/ast_mapper.rs")
mapper = mapper_path.read_text()

# Reject malformed tuple destructuring before any positional indexing.  These
# are user-facing parser inputs, so they must return a mapper error rather than
# panic through names[0..N].
needle = '''    ) -> Result<AstNode, PineMapperError> {\n        if let PineAstNode::FunctionCall {\n            namespace,\n            name,\n            args,\n        } = expr\n'''
replacement = '''    ) -> Result<AstNode, PineMapperError> {\n        if names.is_empty() {\n            return Err(PineMapperError {\n                message: "tuple assignment requires at least one target".to_string(),\n            });\n        }\n\n        if let PineAstNode::FunctionCall {\n            namespace,\n            name,\n            ..\n        } = expr\n        {\n            let expected = match (namespace.as_deref(), name.as_str()) {\n                (Some("ta"), "macd" | "bb" | "dmi") => Some(3usize),\n                (Some("ta"), "supertrend" | "aroon") => Some(2usize),\n                _ => None,\n            };\n            if let Some(expected) = expected {\n                if names.len() != expected {\n                    return Err(PineMapperError {\n                        message: format!(\n                            "ta.{name} returns {expected} values but tuple has {} targets",\n                            names.len()\n                        ),\n                    });\n                }\n            }\n        }\n\n        if let PineAstNode::FunctionCall {\n            namespace,\n            name,\n            args,\n        } = expr\n'''
mapper = replace_once(mapper, needle, replacement, "tuple arity guard")

mapper = replace_test_function(
    mapper,
    "pine_change_defaults_to_one_bar_momentum",
    r'''    #[test]
    fn pine_change_defaults_to_one_bar_momentum() {
        let debug = mapped("//@version=5\nindicator(\"C\")\nc = ta.change(close)\n");
        assert!(debug.contains("FunctionCall { name: \"MOM\""));
        assert!(debug.contains("Number(1.0)"));
    }
''',
)
mapper = replace_test_function(
    mapper,
    "pine_stoch_uses_unsmoothed_fast_k_contract",
    r'''    #[test]
    fn pine_stoch_uses_unsmoothed_fast_k_contract() {
        let debug = mapped(
            "//@version=5\nindicator(\"S\")\ns = ta.stoch(close, high, low, 3)\n",
        );
        assert!(debug.contains("FunctionCall { name: \"STOCHF\""));
        assert!(debug.contains("Number(1.0)"));
    }
''',
)
mapper = replace_test_function(
    mapper,
    "pine_aroon_preserves_high_low_sources",
    r'''    #[test]
    fn pine_aroon_preserves_high_low_sources() {
        let debug = mapped("//@version=5\nindicator(\"A\")\n[u, d] = ta.aroon(3)\n");
        assert!(debug.contains("AROON_UP"));
        assert!(debug.contains("AROON_DN"));
        assert!(debug.contains("Variable(\"HIGH\")"));
        assert!(debug.contains("Variable(\"LOW\")"));
    }
''',
)

if "mod pr14_tuple_arity_tests" not in mapper:
    mapper += r'''

#[cfg(test)]
mod pr14_tuple_arity_tests {
    use super::*;
    use crate::formula::pine::parser::parse_pine;

    #[test]
    fn malformed_multi_return_tuple_is_an_error_not_a_panic() {
        let pine = parse_pine(
            "//@version=5\nindicator(\"M\")\n[a, b] = ta.macd(close, 12, 26, 9)\n",
        )
        .unwrap();
        let err = map_pine_to_alphata(&pine).unwrap_err();
        assert!(err.message.contains("ta.macd returns 3 values"));
        assert!(err.message.contains("tuple has 2 targets"));
    }
}
'''
mapper_path.write_text(mapper)


table_path = Path("core/src/formula/pine/builtin_table.rs")
table = table_path.read_text()

# Default-table construction must never silently overwrite a duplicate key.
old_lookup = '''fn build_lookup(entries: &[BuiltinMapping]) -> HashMap<String, BuiltinMapping> {\n    let mut map = HashMap::new();\n    for entry in entries {\n        let key = make_key(entry.namespace.as_deref(), &entry.pine_name);\n        map.insert(key, entry.clone());\n    }\n    map\n}\n'''
new_lookup = '''fn build_lookup(entries: &[BuiltinMapping]) -> HashMap<String, BuiltinMapping> {\n    let mut map = HashMap::new();\n    for entry in entries {\n        let key = make_key(entry.namespace.as_deref(), &entry.pine_name);\n        match map.entry(key.clone()) {\n            std::collections::hash_map::Entry::Vacant(slot) => {\n                slot.insert(entry.clone());\n            }\n            std::collections::hash_map::Entry::Occupied(_) => {\n                panic!("duplicate Pine builtin mapping key: {key}");\n            }\n        }\n    }\n    map\n}\n'''
table = replace_once(table, old_lookup, new_lookup, "Pine duplicate mapping defense")

table = replace_test_function(
    table,
    "stochastic_metadata_matches_single_series_lowering",
    r'''    #[test]
    fn stochastic_metadata_matches_single_series_lowering() {
        let table = PineBuiltinTable::new();
        let stoch = table.resolve(Some("ta"), "stoch").unwrap();
        assert!(!stoch.multi_return);
        assert_eq!(stoch.alpha_ta_name, "STOCHF");
        assert_eq!(stoch.return_names, ["STOCHF"]);
    }
''',
)

if "mod pr14_mapping_integrity_tests" not in table:
    table += r'''

#[cfg(test)]
mod pr14_mapping_integrity_tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn default_mapping_keys_are_unique() {
        let entries = default_mappings();
        let mut seen = HashSet::new();
        for entry in &entries {
            let key = make_key(entry.namespace.as_deref(), &entry.pine_name);
            assert!(seen.insert(key.clone()), "duplicate Pine mapping key: {key}");
        }
        let table = PineBuiltinTable::new();
        assert_eq!(table.entries().len(), seen.len());
    }
}
'''
table_path.write_text(table)

print("Pine contract tests, duplicate-key defense, and tuple arity guards staged")
