#!/usr/bin/env python3
"""Guard the shipped Python type stub and the public class naming convention.

`ffi/python-binding/finkit/__init__.pyi` is shipped inside the wheel, so it is
part of the public contract. It had rotted badly before this gate existed:

* it declared a ``StreamingIndicator`` base class -- and a ``next()`` method on
  it -- that do not exist in the compiled extension;
* it declared ``cdlhomingsoldier``, a name that has never existed (the real
  function is ``cdlhomingpigeon``);
* its frozen ``__all__`` listed 170 names while the runtime exported 449.

The same audit found a second, related defect: nine streaming classes were
registered under their internal Rust struct name (``PyStreamingPlusDi``) instead
of the user-facing ``Streaming*`` form used by the other seventy.

Two families of checks run here:

**Static (no build required).**
  * the stub parses;
  * ``__all__`` is declared as an annotation rather than a frozen literal, so it
    cannot drift away from the runtime again;
  * every class registered from the Rust side follows the project naming
    convention -- no ``Py``-prefixed name may leak into Python.

**Dynamic (opt-in: only when the caller names the build under test).**
  * every top-level class/function declared in the stub actually exists in the
    extension, and every name in the runtime ``__all__`` resolves to an attribute.

The dynamic half is the one that catches a fabricated declaration, so CI runs it
after building a wheel via ``--require-extension`` (plus ``--expect-prefix`` to
pin the freshly built wheel). It is deliberately *not* run by default: importing
whatever ``finkit`` happens to be on ``sys.path`` would either fail against a
stale site-packages copy or -- worse -- silently validate the wrong build.
"""

from __future__ import annotations

import argparse
import ast
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
STUB = ROOT / "ffi" / "python-binding" / "finkit" / "__init__.pyi"
BINDING_SRC = ROOT / "ffi" / "python-binding" / "src"

# A Python-visible class name must not leak the Rust wrapper prefix. The FFI
# layer legitimately names its structs `PyFoo` internally, but that name is only
# acceptable if `#[pyclass(name = "...")]` renames it for Python.
LEAKED_PREFIX_RE = re.compile(r"^Py[A-Z]")

# How many class declarations we expect to find. This is a tautology guard: if
# the extractor silently stops matching (a refactor of the macro shape, say), the
# checks below would pass on an empty set and report success.
MIN_EXPECTED_CLASSES = 60


def strip_rust_comments(text: str) -> str:
    """Remove ``//`` and ``/* */`` comments so attributes inside them are ignored."""
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.DOTALL)
    return re.sub(r"//[^\n]*", "", text)


def pyclass_python_name(struct_name: str, attrs: str) -> str:
    """Resolve the Python-visible name of a ``#[pyclass]`` struct."""
    match = re.search(r'name\s*=\s*"([^"]+)"', attrs)
    return match.group(1) if match else struct_name


def registered_class_names(src_dir: Path) -> dict[str, str]:
    """Map every Python-visible class name to the file that declares it."""
    found: dict[str, str] = {}

    struct_re = re.compile(
        r"#\[pyclass(?P<attrs>[^\]]*)\]\s*"
        r"(?:#\[[^\]]*\]\s*)*"
        r"pub\s+struct\s+(?P<name>\w+)",
        re.DOTALL,
    )
    # Macro invocations where the *first* argument is the pyclass name, e.g.
    # `py_streaming_hlc_f64!(StreamingPlusDi, finkit::...::StreamingPlusDi, "...")`.
    macro_re = re.compile(
        r"(?m)^\s*(?:py_streaming|py)\w*!\s*\(\s*(?P<name>[A-Za-z_]\w*)\s*,",
    )

    for path in sorted(src_dir.glob("*.rs")):
        text = strip_rust_comments(path.read_text(encoding="utf-8"))
        for match in struct_re.finditer(text):
            name = pyclass_python_name(match.group("name"), match.group("attrs"))
            found[name] = path.name
        for match in macro_re.finditer(text):
            found.setdefault(match.group("name"), path.name)

    return found


def stub_declarations(stub: Path) -> tuple[set[str], list[str], bool]:
    """Return (declared names, errors, whether __all__ is a frozen literal)."""
    errors: list[str] = []
    try:
        tree = ast.parse(stub.read_text(encoding="utf-8"))
    except SyntaxError as exc:  # pragma: no cover - reported, not raised
        return set(), [f"{stub}: does not parse: {exc}"], False

    declared: set[str] = set()
    has_frozen_all = False

    for node in tree.body:
        if isinstance(node, (ast.ClassDef, ast.FunctionDef)):
            declared.add(node.name)
        elif isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name) and target.id == "__all__":
                    has_frozen_all = True
        elif isinstance(node, ast.AnnAssign):
            target = node.target
            if isinstance(target, ast.Name) and target.id == "__all__":
                if node.value is not None:
                    errors.append(
                        f"{stub}: `__all__` is annotated *and* assigned a literal; "
                        "the runtime computes it, so a literal here will rot"
                    )
                    has_frozen_all = True

    return declared, errors, has_frozen_all


def check_static(stub: Path, src_dir: Path) -> tuple[list[str], dict[str, str]]:
    errors: list[str] = []
    declared, stub_errors, has_frozen_all = stub_declarations(stub)
    errors.extend(stub_errors)

    if has_frozen_all:
        errors.append(
            f"{stub}: `__all__` must be declared as an annotation "
            "(`__all__: List[str]`), not a frozen list of names"
        )

    classes = registered_class_names(src_dir)

    if len(classes) < MIN_EXPECTED_CLASSES:
        errors.append(
            f"extractor sanity: only found {len(classes)} registered classes in "
            f"{src_dir} (expected >= {MIN_EXPECTED_CLASSES}); the extractor is "
            "probably broken, which would make every check below vacuous"
        )

    for name, where in sorted(classes.items()):
        if LEAKED_PREFIX_RE.match(name):
            errors.append(
                f"{where}: class `{name}` leaks the internal Rust wrapper prefix; "
                'add `#[pyclass(name = "...")]` or rename it to the user-facing form'
            )

    return errors, classes


def check_dynamic(stub: Path, expect_prefix: Path | None) -> tuple[list[str], str]:
    errors: list[str] = []
    declared, stub_errors, _ = stub_declarations(stub)
    errors.extend(stub_errors)

    try:
        import finkit  # noqa: PLC0415 - optional, only when a build exists
    except Exception as exc:  # pragma: no cover - environment dependent
        raise ImportError(str(exc)) from exc

    location = getattr(finkit, "__file__", None) or "<unknown>"

    # A stale copy in site-packages would silently make this check meaningless,
    # so make the tested build loud -- and reject it outright when the caller
    # knows which one it built.
    if expect_prefix is not None:
        try:
            Path(location).resolve().relative_to(expect_prefix.resolve())
        except ValueError:
            errors.append(
                f"imported finkit from {location}, which is outside the expected "
                f"build at {expect_prefix}; a stale install would make this check vacuous"
            )

    runtime = set(dir(finkit))
    for name in sorted(declared - runtime):
        errors.append(
            f"{stub}: declares `{name}`, which does not exist in the built extension"
        )

    all_attr = getattr(finkit, "__all__", None)
    if not isinstance(all_attr, list):
        errors.append("finkit.__all__ is not a list at runtime")
    else:
        for name in sorted(set(all_attr) - runtime):
            errors.append(f"finkit.__all__ lists `{name}`, which is not an attribute")

    return errors, location


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stub", type=Path, default=STUB, help="path to __init__.pyi")
    parser.add_argument(
        "--src",
        type=Path,
        default=BINDING_SRC,
        help="directory holding the Rust binding sources",
    )
    parser.add_argument(
        "--require-extension",
        action="store_true",
        help="fail instead of skipping when finkit cannot be imported",
    )
    parser.add_argument(
        "--expect-prefix",
        type=Path,
        default=None,
        help="fail if the imported finkit does not live under this path",
    )
    args = parser.parse_args(argv)

    if not args.stub.exists():
        print(f"FAIL: stub not found at {args.stub}", file=sys.stderr)
        return 1

    errors, classes = check_static(args.stub, args.src)
    print(f"static: {len(classes)} registered classes scanned")

    # The dynamic half is only meaningful when the caller names the build under
    # test. Importing whatever `finkit` happens to be on `sys.path` would fail
    # against a stale site-packages copy, or -- worse -- silently validate the
    # wrong build and report success. So it is opt-in: ask for it with
    # `--require-extension`, or pin the build with `--expect-prefix`.
    want_dynamic = args.require_extension or args.expect_prefix is not None

    if not want_dynamic:
        print(
            "dynamic: SKIPPED (static checks only; pass --require-extension or "
            "--expect-prefix to check a built extension)"
        )
    else:
        try:
            dynamic_errors, location = check_dynamic(args.stub, args.expect_prefix)
        except ImportError as exc:
            errors.append(f"could not import finkit: {exc}")
        else:
            errors.extend(dynamic_errors)
            print(f"dynamic: checked stub against the extension at {location}")

    if errors:
        print(f"\nFAIL: {len(errors)} problem(s)", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print("OK: python stub contract holds")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
