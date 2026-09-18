import json
import math
from pathlib import Path

import finkit as ta


FIXTURE = json.loads(
    (
        Path(__file__).resolve().parents[3]
        / "tests/contracts/talib_numeric_contract_v1.json"
    ).read_text(encoding="utf-8")
)


def test_executes_every_talib_numeric_contract_vector():
    assert FIXTURE["semantic_profile"] == "talib_0_8_0"
    assert len(FIXTURE["vectors"]) == 201

    for vector in FIXTURE["vectors"]:
        payload = json.loads(
            ta.operation_execute_json(
                json.dumps(
                    {
                        "operation": vector["operation"],
                        "semantic_profile": FIXTURE["semantic_profile"],
                        "input_order": vector["input_order"],
                        "inputs": FIXTURE["inputs"],
                        "params": vector["params"],
                    }
                )
            )
        )
        assert "error" not in payload, f"{vector['operation']}: {payload}"

        for output, expected in vector["expected"].items():
            actual = payload["values"].get(output)
            assert actual is not None, f"{vector['operation']}/{output}: missing output"
            assert len(actual) == len(expected)
            atol = vector["tolerance"]["atol"]
            rtol = vector["tolerance"]["rtol"]
            for index, expected_value in enumerate(expected):
                actual_value = actual[index]
                if expected_value is None:
                    assert actual_value is None, (
                        f"{vector['operation']}/{output}[{index}]: "
                        "expected null"
                    )
                    continue
                assert actual_value is not None, (
                    f"{vector['operation']}/{output}[{index}]: unexpected null"
                )
                error = abs(actual_value - expected_value)
                limit = atol + rtol * abs(expected_value)
                assert math.isfinite(actual_value)
                assert error <= limit, (
                    f"{vector['operation']}/{output}[{index}]: "
                    f"error {error} > {limit}"
                )
