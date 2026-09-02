use finkit::compute::ComputeEffect;
use finkit::formula::{
    parse_formula_for_terminal, CompatibilityLevel, FormulaComputePlan, FormulaTerminal,
};

const TDX: &str = include_str!("fixtures/formula_compat/tdx/basic.txt");
const THS: &str = include_str!("fixtures/formula_compat/ths/basic.txt");
const EAST_MONEY: &str = include_str!("fixtures/formula_compat/eastmoney/basic.txt");
const PINE: &str = include_str!("fixtures/formula_compat/pine/basic.txt");

#[test]
fn external_terminal_contracts_are_explicit_subsets() {
    assert_eq!(
        FormulaTerminal::Finkit.compatibility_level(),
        CompatibilityLevel::Native
    );
    for terminal in [
        FormulaTerminal::TongDaXin,
        FormulaTerminal::TongHuaShun,
        FormulaTerminal::EastMoney,
        FormulaTerminal::TradingView,
    ] {
        assert_eq!(
            terminal.compatibility_level(),
            CompatibilityLevel::CommonSubset
        );
    }
}

#[test]
fn china_terminal_golden_fixtures_parse_through_one_canonical_runtime() {
    for (terminal, source) in [
        (FormulaTerminal::TongDaXin, TDX),
        (FormulaTerminal::TongHuaShun, THS),
        (FormulaTerminal::EastMoney, EAST_MONEY),
    ] {
        let ast = parse_formula_for_terminal(source, terminal).unwrap();
        let semantic = FormulaComputePlan::compile(&ast).unwrap();
        assert!(semantic.plan().has_observable_effects());
        assert!(!semantic.plan().is_empty());
    }
}

#[test]
fn tdx_golden_fixture_preserves_assignment_and_named_output_effects() {
    let ast = parse_formula_for_terminal(TDX, FormulaTerminal::TongDaXin).unwrap();
    let semantic = FormulaComputePlan::compile(&ast).unwrap();
    let plan = semantic.plan();

    let mut saw_ma_assignment = false;
    let mut saw_signal_output = false;
    for &id in plan.execution_order() {
        match &plan.node(id).unwrap().capabilities.effect {
            ComputeEffect::WriteVariable(name) if name == "MA5" => saw_ma_assignment = true,
            ComputeEffect::EmitOutput(name) if name == "SIGNAL" => saw_signal_output = true,
            _ => {}
        }
    }

    assert!(saw_ma_assignment);
    assert!(saw_signal_output);
}

#[test]
fn pine_golden_fixture_parses_as_documented_subset() {
    let ast = parse_formula_for_terminal(PINE, FormulaTerminal::TradingView).unwrap();
    assert!(!format!("{ast:?}").is_empty());
}
