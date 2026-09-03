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
