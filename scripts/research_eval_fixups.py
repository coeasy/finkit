from pathlib import Path


def replace(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    if new in text:
        return
    if old not in text:
        raise RuntimeError(f'missing expected source fragment in {path}')
    p.write_text(text.replace(old, new, 1))


# FactorStudy tests follow the tradable overlapping-cohort report shape.
replace(
    'factor-analysis/src/api.rs',
    'assert!(report.performance.by_horizon.contains_key(&1));',
    'assert!(report.performance.by_holding_period.contains_key(&1));',
)

# Jensen alpha is defined on excess returns rather than raw returns.
replace(
    'core/src/performance.rs',
    'let alpha_per_period = strategy_mean - beta * benchmark_mean;',
    'let alpha_per_period = (strategy_mean - config.risk_free_rate)\n        - beta * (benchmark_mean - config.risk_free_rate);',
)
