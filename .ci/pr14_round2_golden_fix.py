from pathlib import Path

fixture = Path("core/tests/fixtures/formula_compat/pine/basic.txt")
text = fixture.read_text(encoding="utf-8")
old = '''//@version=5
indicator("RSI")
length = input(14)
rsi = ta.rsi(close, length)
plot(rsi)
'''
new = '''//@version=5
indicator("RSI")
length = input(14)
rsi = ta.rsi(close, length)
sma20 = ta.sma(close, 20)
plot(rsi)
plot(sma20)
'''
if text != old:
    raise SystemExit("Pine golden fixture does not match expected baseline")
fixture.write_text(new, encoding="utf-8")

path = Path("core/tests/formula_terminal_golden.rs")
text = path.read_text(encoding="utf-8")
old = '''#[test]
fn pine_golden_fixture_parses_as_documented_subset() {
    let ast = parse_formula_for_terminal(PINE, FormulaTerminal::TradingView).unwrap();
    assert!(!format!("{ast:?}").is_empty());
}
'''
new = '''#[test]
fn pine_golden_fixture_parses_as_documented_subset() {
    let ast = parse_formula_for_terminal(PINE, FormulaTerminal::TradingView).unwrap();
    let debug = format!("{ast:?}");
    assert!(!debug.is_empty());
    assert!(debug.contains("FunctionCall { name: \\"MA\\""));
    assert!(!debug.contains("FunctionCall { name: \\"SMA\\""));
}
'''
if text.count(old) != 1:
    raise SystemExit("Pine golden test baseline not found")
path.write_text(text.replace(old, new, 1), encoding="utf-8")
