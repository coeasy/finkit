"""Rewrite generated float64 Python indicators to return NumPy arrays directly."""

from pathlib import Path


PATH = Path(__file__).resolve().parents[1] / "ffi/python-binding/src/generated.rs"


def main() -> None:
    source = PATH.read_text(encoding="utf-8")
    source = source.replace(
        "-> PyResult<(Vec<f64>, Vec<f64>, Vec<f64>)>",
        "-> PyResult<(Py<PyArray1<f64>>, Py<PyArray1<f64>>, Py<PyArray1<f64>>)>"
    )
    source = source.replace(
        "-> PyResult<(Vec<f64>, Vec<f64>)>",
        "-> PyResult<(Py<PyArray1<f64>>, Py<PyArray1<f64>>)>"
    )
    source = source.replace(
        "-> PyResult<Vec<f64>>",
        "-> PyResult<Py<PyArray1<f64>>>"
    )
    source = source.replace(
        "-> PyResult<(Bound<'_, PyArray1<f64>>, Bound<'_, PyArray1<f64>>, Bound<'_, PyArray1<f64>>)> ",
        "-> PyResult<(Py<PyArray1<f64>>, Py<PyArray1<f64>>, Py<PyArray1<f64>>)> "
    )
    source = source.replace(
        "-> PyResult<(Bound<'_, PyArray1<f64>>, Bound<'_, PyArray1<f64>>)> ",
        "-> PyResult<(Py<PyArray1<f64>>, Py<PyArray1<f64>>)> "
    )
    source = source.replace(
        "-> PyResult<Bound<'_, PyArray1<f64>>> ",
        "-> PyResult<Py<PyArray1<f64>>> "
    )

    needle = "py.detach(||"
    positions = []
    cursor = 0
    while True:
        index = source.find(needle, cursor)
        if index < 0:
            break
        positions.append(index)
        cursor = index + len(needle)

    for index in reversed(positions):
        function_start = source.rfind("fn ", 0, index)
        signature = source[function_start:index]
        if "PyResult<(Py<PyArray1<f64>>, Py<PyArray1<f64>>, Py<PyArray1<f64>>)>" in signature:
            replacement = "py_arrays3_f64(py, ||"
        elif "PyResult<(Py<PyArray1<f64>>, Py<PyArray1<f64>>)>" in signature:
            replacement = "py_arrays2_f64(py, ||"
        elif "PyResult<Py<PyArray1<f64>>>" in signature:
            replacement = "py_array_f64(py, ||"
        else:
            replacement = needle
        source = source[:index] + source[index:].replace(needle, replacement, 1)

    # Candlestick bindings return integer arrays and must retain their normal
    # Vec conversion path.
    cursor = 0
    while True:
        start = source.find("fn cdl_", cursor)
        if start < 0:
            break
        end = source.find("\n#[pyfunction]", start + 1)
        if end < 0:
            end = len(source)
        block = source[start:end]
        block = block.replace("py_array_f64(py, ||", "py.detach(||")
        block = block.replace("py_arrays2_f64(py, ||", "py.detach(||")
        block = block.replace("py_arrays3_f64(py, ||", "py.detach(||")
        source = source[:start] + block + source[end:]
        cursor = start + len(block)

    PATH.write_text(source, encoding="utf-8")


if __name__ == "__main__":
    main()
