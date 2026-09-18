import json
from pathlib import Path

import numpy as np

import finkit as ta


FIXTURE = json.loads(
    (Path(__file__).resolve().parents[3] / "tests/contracts/engine_contract_v1.json").read_text(
        encoding="utf-8"
    )
)


def test_shared_engine_contract_v1_is_executed_by_python_binding():
    operation = FIXTURE["operation"]
    operation_result = json.loads(ta.operation_execute_json(json.dumps(operation["request"])))
    assert operation_result["values"]["SMA"] == operation["expected_primary"]

    formula = FIXTURE["formula"]
    formula_result = json.loads(
        ta.formula_eval_contract_json(
            formula["source"],
            formula["dialect"],
            np.asarray(formula["open"], dtype=np.float64),
            np.asarray(formula["high"], dtype=np.float64),
            np.asarray(formula["low"], dtype=np.float64),
            np.asarray(formula["close"], dtype=np.float64),
            np.asarray(formula["volume"], dtype=np.float64),
        )
    )
    assert formula_result["values"]["__PRIMARY__"] == formula["expected_primary"]

    factor = FIXTURE["factor"]
    factor_result = json.loads(ta.factor_execute_json(json.dumps(factor["request"])))
    assert factor_result["values"]["momentum_5"] == factor["expected_primary"]

    composite = FIXTURE["composite"]
    composite_result = json.loads(
        ta.composite_execute_json(json.dumps(composite["request"]))
    )
    assert composite_result["values"]["sma3"] == composite["expected_primary"]
