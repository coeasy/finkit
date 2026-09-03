#!/usr/bin/env python3
"""Single Source of Truth (SSOT) documentation generator.

Scans Rust sources and workspace configuration to produce authoritative
generated docs under docs/generated/.

Sources:
  - core/src/indicators/mod.rs  — pub module exports and indicator catalog
  - ffi/c-binding/src/lib.rs    — FfiStatus error codes
  - Cargo.toml workspace members — version matrix
  - target/criterion/            — benchmark stats (optional, via shared logic)

Usage:
    python scripts/gen_ssot_docs.py --generate
    python scripts/gen_ssot_docs.py --check
"""

from __future__ import annotations

import argparse
import glob
import json
import math
import re
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INDICATORS_MOD = ROOT / "core" / "src" / "indicators" / "mod.rs"
INDICATORS_DIR = ROOT / "core" / "src" / "indicators"
STREAMING_MOD = ROOT / "core" / "src" / "streaming" / "mod.rs"
STREAMING_DIR = ROOT / "core" / "src" / "streaming"
FORMULA_MOD = ROOT / "core" / "src" / "formula" / "mod.rs"
FORMULA_FUNCTIONS = ROOT / "core" / "src" / "formula" / "functions.rs"
FEATURES_MOD = ROOT / "core" / "src" / "features" / "mod.rs"
PINE_BUILTIN = ROOT / "core" / "src" / "formula" / "pine" / "builtin_table.rs"
FFI_LIB = ROOT / "ffi" / "c-binding" / "src" / "lib.rs"
WORKSPACE_CARGO = ROOT / "Cargo.toml"
DOTNET_PROJECT = ROOT / "ffi" / "dotnet-binding" / "src" / "Finkit" / "Finkit.csproj"
JAVA_POM = ROOT / "ffi" / "java-binding" / "pom.xml"
GENERATED_DIR = ROOT / "docs" / "generated"
DEFAULT_CRITERION_DIR = ROOT / "target" / "criterion"
INDICATOR_REGISTRY = ROOT / "docs" / "indicator_registry.json"

OUT_INDICATORS = GENERATED_DIR / "indicators.md"
OUT_STREAMING = GENERATED_DIR / "streaming-indicators.md"
OUT_FORMULA = GENERATED_DIR / "formula-functions.md"
OUT_FEATURES = GENERATED_DIR / "features.md"
OUT_PINE = GENERATED_DIR / "pine-compatibility.md"
OUT_ERROR_CODES = GENERATED_DIR / "error-codes.md"
OUT_VERSION_MATRIX = GENERATED_DIR / "version-matrix.md"

PUB_MOD_RE = re.compile(r"^pub\s+mod\s+(\w+)\s*;")
PUB_FN_RE = re.compile(r"^pub\s+fn\s+(\w+)\s*\(")
FFI_STATUS_ENUM_RE = re.compile(
    r"pub\s+enum\s+FfiStatus\s*\{([^}]+)\}",
    re.DOTALL,
)
FFI_STATUS_VARIANT_RE = re.compile(r"(\w+)\s*=\s*(-?\d+)\s*,?")
WORKSPACE_MEMBERS_RE = re.compile(
    r"^\[workspace\]\s*$.*?^members\s*=\s*\[(.*?)\]",
    re.MULTILINE | re.DOTALL,
)
CARGO_VERSION_RE = re.compile(r"^version\s*=\s*\"([^\"]+)\"")
CARGO_VERSION_WORKSPACE_RE = re.compile(r"^version\.workspace\s*=\s*true")


@dataclass
class BenchStats:
    group: str
    name: str
    input_size: str | None
    mean_ns: float | None
    median_ns: float | None
    stddev_ns: float | None


def read_workspace_version() -> str:
    text = WORKSPACE_CARGO.read_text(encoding="utf-8")
    in_block = False
    for line in text.splitlines():
        if line.strip() == "[workspace.package]":
            in_block = True
            continue
        if in_block and line.startswith("[") and not line.startswith("[workspace"):
            break
        if in_block:
            m = CARGO_VERSION_RE.match(line)
            if m:
                return m.group(1)
    raise ValueError(f"workspace.package.version not found in {WORKSPACE_CARGO}")


def workspace_member_paths() -> list[Path]:
    text = WORKSPACE_CARGO.read_text(encoding="utf-8")
    m = WORKSPACE_MEMBERS_RE.search(text)
    if not m:
        return []
    members_block = m.group(1)
    paths: list[Path] = []
    for quoted in re.findall(r"\"([^\"]+)\"", members_block):
        paths.append(ROOT / quoted / "Cargo.toml")
    return paths


def read_cargo_package_version(cargo_path: Path, workspace_version: str) -> tuple[str, str]:
    """Return (resolved_version, source_label)."""
    if not cargo_path.is_file():
        return "missing", "file not found"
    text = cargo_path.read_text(encoding="utf-8")
    for line in text.splitlines():
        if CARGO_VERSION_WORKSPACE_RE.match(line):
            return workspace_version, "workspace"
        m = CARGO_VERSION_RE.match(line)
        if m:
            return m.group(1), "explicit"
    return "—", "no version field"


def read_pyproject_version() -> str | None:
    path = ROOT / "ffi" / "python-binding" / "pyproject.toml"
    if not path.is_file():
        return None
    in_project = False
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip() == "[project]":
            in_project = True
            continue
        if in_project and line.startswith("[") and line.strip() != "[project]":
            break
        if in_project:
            m = CARGO_VERSION_RE.match(line)
            if m:
                return m.group(1)
    return None


def read_node_version() -> str | None:
    path = ROOT / "ffi" / "node-binding" / "package.json"
    if not path.is_file():
        return None
    data = json.loads(path.read_text(encoding="utf-8"))
    version = data.get("version")
    return str(version) if version else None


def read_xml_project_version(path: Path, tag: str) -> str | None:
    """Read the first release version tag from an XML binding manifest."""
    if not path.is_file():
        return None
    text = path.read_text(encoding="utf-8")
    pattern = rf"(?m)^[ \t]*<{re.escape(tag)}>([^<]+)</{re.escape(tag)}>[ \t]*$"
    match = re.search(pattern, text)
    return match.group(1).strip() if match else None


def parse_indicator_modules() -> list[str]:
    modules: list[str] = []
    for line in INDICATORS_MOD.read_text(encoding="utf-8").splitlines():
        m = PUB_MOD_RE.match(line.strip())
        if m:
            modules.append(m.group(1))
    return modules


def scan_module_functions(module_name: str) -> list[str]:
    path = INDICATORS_DIR / f"{module_name}.rs"
    if not path.is_file():
        return []
    functions: list[str] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        m = PUB_FN_RE.match(line.strip())
        if m:
            functions.append(m.group(1))
    return sorted(set(functions))


def build_indicator_catalog() -> dict[str, list[str]]:
    catalog: dict[str, list[str]] = {}
    for module in parse_indicator_modules():
        catalog[module] = scan_module_functions(module)
    return catalog


def parse_streaming_modules() -> list[str]:
    """Parse streaming/mod.rs for pub mod declarations."""
    modules: list[str] = []
    if not STREAMING_MOD.is_file():
        return modules
    for line in STREAMING_MOD.read_text(encoding="utf-8").splitlines():
        m = PUB_MOD_RE.match(line.strip())
        if m:
            modules.append(m.group(1))
    return modules


def scan_streaming_structs(module_name: str) -> list[str]:
    """Scan streaming module for public struct definitions (indicator classes)."""
    path = STREAMING_DIR / f"{module_name}.rs"
    if not path.is_file():
        return []
    structs: list[str] = []
    struct_re = re.compile(r"^pub\s+struct\s+(\w+)")
    for line in path.read_text(encoding="utf-8").splitlines():
        m = struct_re.match(line.strip())
        if m:
            structs.append(m.group(1))
    return sorted(set(structs))


def build_streaming_catalog() -> dict[str, list[str]]:
    """Build catalog of streaming indicators: module -> [struct names]."""
    catalog: dict[str, list[str]] = {}
    for module in parse_streaming_modules():
        structs = scan_streaming_structs(module)
        if structs:
            catalog[module] = structs
    return catalog


def parse_formula_functions() -> list[str]:
    """Parse formula/functions.rs for builtin function names."""
    if not FORMULA_FUNCTIONS.is_file():
        return []
    fns: list[str] = []
    # Look for function name strings in get_builtin_functions()
    fn_name_re = re.compile(r'"([A-Z_]+)"\s*,')
    for line in FORMULA_FUNCTIONS.read_text(encoding="utf-8").splitlines():
        m = fn_name_re.search(line)
        if m:
            fns.append(m.group(1))
    return sorted(set(fns))


def parse_features_modules() -> list[str]:
    """Parse features/mod.rs for submodule declarations."""
    if not FEATURES_MOD.is_file():
        return []
    modules: list[str] = []
    for line in FEATURES_MOD.read_text(encoding="utf-8").splitlines():
        m = PUB_MOD_RE.match(line.strip())
        if m:
            modules.append(m.group(1))
    return modules


def parse_pine_builtins() -> list[str]:
    """Parse Pine Script builtin function table.

    Extracts pine_name values from BuiltinMapping structs in builtin_table.rs.
    Each mapping has a line like: pine_name: "sma".to_string(),
    """
    if not PINE_BUILTIN.is_file():
        return []
    fns: list[str] = []
    # Match pine_name: "xxx".to_string()
    pine_name_re = re.compile(r'pine_name:\s*"(\w+)"')
    for line in PINE_BUILTIN.read_text(encoding="utf-8").splitlines():
        m = pine_name_re.search(line)
        if m:
            fns.append(m.group(1))
    return sorted(set(fns))


def parse_ffi_status_codes() -> list[tuple[str, int]]:
    text = FFI_LIB.read_text(encoding="utf-8")
    m = FFI_STATUS_ENUM_RE.search(text)
    if not m:
        raise ValueError(f"FfiStatus enum not found in {FFI_LIB}")
    variants: list[tuple[str, int]] = []
    for vm in FFI_STATUS_VARIANT_RE.finditer(m.group(1)):
        variants.append((vm.group(1), int(vm.group(2))))
    return variants


def load_estimate_point(estimates: dict, key: str) -> float | None:
    section = estimates.get(key, {})
    if isinstance(section, dict):
        if "point_estimate" in section:
            return float(section["point_estimate"])
        if "estimate" in section:
            return float(section["estimate"])
    return None


def stddev_from_sample(sample_path: Path) -> float | None:
    try:
        with open(sample_path, encoding="utf-8") as fh:
            data = json.load(fh)
    except (OSError, json.JSONDecodeError):
        return None

    times = data.get("times", [])
    flat: list[float] = []
    for entry in times:
        if isinstance(entry, list):
            flat.extend(float(x) for x in entry)
        else:
            flat.append(float(entry))
    if len(flat) < 2:
        return None
    mean = sum(flat) / len(flat)
    variance = sum((x - mean) ** 2 for x in flat) / (len(flat) - 1)
    return math.sqrt(variance)


def parse_bench_path(criterion_dir: Path, estimates_path: Path) -> BenchStats | None:
    try:
        rel = estimates_path.relative_to(criterion_dir)
    except ValueError:
        return None

    parts = rel.parts
    if len(parts) < 4 or parts[-2] != "new" or parts[-1] != "estimates.json":
        return None

    try:
        with open(estimates_path, encoding="utf-8") as fh:
            estimates = json.load(fh)
    except (OSError, json.JSONDecodeError):
        return None

    sample_path = estimates_path.parent / "sample.json"
    stddev = stddev_from_sample(sample_path)

    if len(parts) == 4:
        group, name = parts[0], parts[1]
        input_size = None
    elif len(parts) == 5:
        group, name, input_size = parts[0], parts[1], parts[2]
    else:
        return None

    return BenchStats(
        group=group,
        name=name,
        input_size=input_size,
        mean_ns=load_estimate_point(estimates, "mean"),
        median_ns=load_estimate_point(estimates, "median"),
        stddev_ns=stddev,
    )


def collect_all_bench_stats(criterion_dir: Path) -> dict[tuple[str, str, str | None], BenchStats]:
    results: dict[tuple[str, str, str | None], BenchStats] = {}
    if not criterion_dir.is_dir():
        return results

    pattern = str(criterion_dir / "**" / "new" / "estimates.json")
    for path_str in glob.glob(pattern, recursive=True):
        stats = parse_bench_path(criterion_dir, Path(path_str))
        if stats is None:
            continue
        key = (stats.group, stats.name, stats.input_size)
        results[key] = stats
    return results


def format_indicators_md(catalog: dict[str, list[str]]) -> str:
    total_fns = sum(len(fns) for fns in catalog.values())
    lines = [
        "# Indicator Catalog",
        "",
        "> **SSOT** — auto-generated from `core/src/indicators/mod.rs` and submodule `pub fn` exports.",
        "> Do not edit manually. Regenerate: `python scripts/gen_ssot_docs.py --generate`",
        "",
        f"Modules exported from `indicators/mod.rs`: **{len(catalog)}** | "
        f"Public indicator functions: **{total_fns}**",
        "",
    ]

    for module in sorted(catalog.keys()):
        functions = catalog[module]
        lines.append(f"## {module}")
        lines.append("")
        if not functions:
            lines.append("_No `pub fn` exports in this module file._")
        else:
            lines.append("| Function |")
            lines.append("|----------|")
            for fn in functions:
                lines.append(f"| `{fn}` |")
        lines.append("")

    lines.append("## Regenerate")
    lines.append("")
    lines.append("```bash")
    lines.append("python scripts/gen_ssot_docs.py --generate")
    lines.append("python scripts/gen_ssot_docs.py --check   # CI gate")
    lines.append("```")
    lines.append("")
    return "\n".join(lines)


def read_registered_streaming_count() -> int | None:
    """Read the authoritative streaming count from the indicator registry."""
    if not INDICATOR_REGISTRY.is_file():
        return None
    data = json.loads(INDICATOR_REGISTRY.read_text(encoding="utf-8"))
    indicators = data.get("indicators", [])
    return sum(1 for item in indicators if item.get("streaming") is True)


def format_streaming_md(
    catalog: dict[str, list[str]], registered_count: int | None = None
) -> str:
    """Generate streaming indicators documentation."""
    total_structs = sum(len(structs) for structs in catalog.values())
    registered_label = (
        f"Registered indicator entries marked streaming in `docs/indicator_registry.json`: **{registered_count}**"
        if registered_count is not None
        else "Registered streaming indicator entries: **not available**"
    )
    lines = [
        "# Streaming Indicators Catalog",
        "",
        "> **SSOT** — auto-generated from `core/src/streaming/mod.rs` and submodule `pub struct` exports.",
        "> Do not edit manually. Regenerate: `python scripts/gen_ssot_docs.py --generate`",
        "",
        f"Streaming source modules: **{len(catalog)}** | "
        f"Direct public structs: **{total_structs}**",
        registered_label,
        "",
        "Streaming indicators provide O(1) per-bar updates via the `StreamingIndicator` trait.",
        "The source scan lists directly detected public structs; the registered count is the user-facing indicator count.",
        "",
    ]

    for module in sorted(catalog.keys()):
        structs = catalog[module]
        lines.append(f"## {module}")
        lines.append("")
        if not structs:
            lines.append("_No public streaming indicator structs._")
        else:
            lines.append("| Struct |")
            lines.append("|--------|")
            for struct in structs:
                lines.append(f"| `{struct}` |")
        lines.append("")

    lines.append("## Usage Example")
    lines.append("")
    lines.append("```rust")
    lines.append("use finkit::streaming::{StreamingIndicator, OhlcvBar};")
    lines.append("use finkit::streaming::indicators::StreamingSma;")
    lines.append("")
    lines.append("let mut sma = StreamingSma::new(20);")
    lines.append("let bar = OhlcvBar::new(open, high, low, close, volume);")
    lines.append("if let Some(value) = sma.next(&bar) {")
    lines.append("    println!(\"SMA: {}\", value);")
    lines.append("}")
    lines.append("```")
    lines.append("")

    lines.append("## Regenerate")
    lines.append("")
    lines.append("```bash")
    lines.append("python scripts/gen_ssot_docs.py --generate")
    lines.append("python scripts/gen_ssot_docs.py --check   # CI gate")
    lines.append("```")
    lines.append("")
    return "\n".join(lines)


def format_formula_md(functions: list[str]) -> str:
    """Generate formula engine functions documentation."""
    lines = [
        "# Formula Engine Functions",
        "",
        "> **SSOT** — auto-generated from `core/src/formula/functions.rs`.",
        "> Do not edit manually. Regenerate: `python scripts/gen_ssot_docs.py --generate`",
        "",
        f"Built-in formula functions: **{len(functions)}**",
        "",
        "These functions are available in the formula DSL for expressions like `SMA(CLOSE, 20)`.",
        "",
        "## Function Reference",
        "",
        "| Function |",
        "|----------|",
    ]

    for fn in functions:
        lines.append(f"| `{fn}` |")

    lines.append("")
    lines.append("## Usage Example")
    lines.append("")
    lines.append("```rust")
    lines.append("use finkit::formula::FormulaEngine;")
    lines.append("")
    lines.append("let engine = FormulaEngine::new();")
    lines.append("let result = engine.eval(\"SMA(CLOSE, 20) + EMA(CLOSE, 10)\", &ohlcv)?;")
    lines.append("```")
    lines.append("")

    lines.append("## Regenerate")
    lines.append("")
    lines.append("```bash")
    lines.append("python scripts/gen_ssot_docs.py --generate")
    lines.append("python scripts/gen_ssot_docs.py --check   # CI gate")
    lines.append("```")
    lines.append("")
    return "\n".join(lines)


def format_features_md(modules: list[str]) -> str:
    """Generate feature engineering modules documentation."""
    lines = [
        "# Feature Engineering Modules",
        "",
        "> **SSOT** — auto-generated from `core/src/features/mod.rs`.",
        "> Do not edit manually. Regenerate: `python scripts/gen_ssot_docs.py --generate`",
        "",
        f"Direct public submodules declared with `pub mod`: **{len(modules)}**",
        "",
        "Feature engineering transforms raw OHLCV data into ML-ready feature matrices.",
        "Feature symbols from internal modules are re-exported by `finkit::features`; the count above is not the total API symbol count.",
        "",
        "## Module Reference",
        "",
        "| Module |",
        "|--------|",
    ]

    for module in sorted(modules):
        lines.append(f"| `{module}` |")

    lines.append("")
    lines.append("## Usage Example")
    lines.append("")
    lines.append("```rust")
    lines.append("use finkit::features::{FeatureMatrix, FeatureSet};")
    lines.append("")
    lines.append("let mut features = FeatureSet::new();")
    lines.append("features.add_indicator(\"sma\", &[5, 10, 20]);")
    lines.append("features.add_indicator(\"rsi\", &[14]);")
    lines.append("let matrix = features.generate(&ohlcv)?;")
    lines.append("```")
    lines.append("")

    lines.append("## Regenerate")
    lines.append("")
    lines.append("```bash")
    lines.append("python scripts/gen_ssot_docs.py --generate")
    lines.append("python scripts/gen_ssot_docs.py --check   # CI gate")
    lines.append("```")
    lines.append("")
    return "\n".join(lines)


def format_pine_md(builtins: list[str]) -> str:
    """Generate Pine Script compatibility documentation."""
    lines = [
        "# Pine Script Compatibility",
        "",
        "> **SSOT** — auto-generated from `core/src/formula/pine/builtin_table.rs`.",
        "> Do not edit manually. Regenerate: `python scripts/gen_ssot_docs.py --generate`",
        "",
        f"Pine Script v5 built-in functions supported: **{len(builtins)}**",
        "",
        "Finkit supports a subset of Pine Script v5 for indicator migration from TradingView.",
        "",
        "## Supported Built-in Functions",
        "",
        "| Function |",
        "|----------|",
    ]

    for fn in builtins:
        lines.append(f"| `ta.{fn}` |")

    lines.append("")
    lines.append("## Usage Example")
    lines.append("")
    lines.append("```pine")
    lines.append("//@version=5")
    lines.append("indicator(\"RSI Example\", overlay=false)")
    lines.append("rsi = ta.rsi(close, 14)")
    lines.append("plot(rsi)")
    lines.append("```")
    lines.append("")

    lines.append("## Regenerate")
    lines.append("")
    lines.append("```bash")
    lines.append("python scripts/gen_ssot_docs.py --generate")
    lines.append("python scripts/gen_ssot_docs.py --check   # CI gate")
    lines.append("```")
    lines.append("")
    return "\n".join(lines)


def ffi_status_description(name: str) -> str:
    descriptions = {
        "Ok": "Success / no error",
        "NullPointer": "A required pointer argument was null",
        "InvalidParameter": "Parameter validation failed",
        "InsufficientData": "Input series too short for the requested calculation",
        "InternalError": "Internal error or panic caught at FFI boundary",
        "InvalidUtf8": "Invalid UTF-8 in a string argument",
        "Unknown": "Unclassified error",
    }
    return descriptions.get(name, "")


def format_error_codes_md(codes: list[tuple[str, int]]) -> str:
    lines = [
        "# FFI Error Codes (`FfiStatus`)",
        "",
        "> **SSOT** — auto-generated from `ffi/c-binding/src/lib.rs`.",
        "> Do not edit manually. Regenerate: `python scripts/gen_ssot_docs.py --generate`",
        "",
        "Stable ABI error codes returned at the C FFI boundary (`FfiStatus`).",
        "",
        "| Code | Name | Description |",
        "|------|------|-------------|",
    ]
    for name, code in codes:
        lines.append(f"| {code} | `{name}` | {ffi_status_description(name)} |")

    lines.extend(
        [
            "",
            "## Regenerate",
            "",
            "```bash",
            "python scripts/gen_ssot_docs.py --generate",
            "python scripts/gen_ssot_docs.py --check",
            "```",
            "",
        ]
    )
    return "\n".join(lines)


def format_version_matrix_md(
    canonical: str,
    rows: list[tuple[str, str, str, str, str]],
    bench_count: int,
) -> str:
    lines = [
        "# Version Matrix",
        "",
        "> **SSOT** — auto-generated from workspace `Cargo.toml` and binding manifests (including .NET and Java metadata).",
        "> Do not edit manually. Regenerate: `python scripts/gen_ssot_docs.py --generate`",
        "",
        f"Canonical workspace version: **`{canonical}`**",
        "",
        "| Package | Path | Version | Source | Match |",
        "|---------|------|---------|--------|-------|",
    ]

    for package, path, version, source, match in rows:
        lines.append(f"| {package} | `{path}` | {version} | {source} | {match} |")

    lines.extend(
        [
            "",
            "## Benchmark data",
            "",
            f"Criterion JSON benchmarks indexed: **{bench_count}** "
            f"(from `target/criterion/` when present).",
            "",
            "Full benchmark report: `python scripts/gen_benchmark_report.py` → `docs/BENCHMARK_REPORT.md`.",
            "",
            "## Regenerate",
            "",
            "```bash",
            "python scripts/gen_ssot_docs.py --generate",
            "python scripts/gen_ssot_docs.py --check",
            "```",
            "",
        ]
    )
    return "\n".join(lines)


def build_version_rows(canonical: str) -> list[tuple[str, str, str, str, str]]:
    rows: list[tuple[str, str, str, str, str]] = []

    rows.append(
        (
            "workspace",
            "Cargo.toml",
            canonical,
            "workspace.package",
            "✅ canonical",
        )
    )

    for cargo_path in workspace_member_paths():
        rel = cargo_path.relative_to(ROOT).as_posix()
        try:
            with open(cargo_path, encoding="utf-8") as fh:
                name_line = next(
                    (ln for ln in fh if ln.startswith("name = ")),
                    "name = ?",
                )
            pkg_name = name_line.split("=", 1)[1].strip().strip('"')
        except OSError:
            pkg_name = cargo_path.parent.name

        version, source = read_cargo_package_version(cargo_path, canonical)
        match = "✅" if version == canonical else "❌"
        rows.append((pkg_name, rel, version, source, match))

    py_ver = read_pyproject_version()
    if py_ver is not None:
        match = "✅" if py_ver == canonical else "❌"
        rows.append(
            (
                "finkit-python (pyproject)",
                "ffi/python-binding/pyproject.toml",
                py_ver,
                "project.version",
                match,
            )
        )

    node_ver = read_node_version()
    if node_ver is not None:
        match = "✅" if node_ver == canonical else "❌"
        rows.append(
            (
                "finkit-node (package.json)",
                "ffi/node-binding/package.json",
                node_ver,
                "version",
                match,
            )
        )

    for package, path, tag, source in (
        (
            "finkit-dotnet (.csproj)",
            DOTNET_PROJECT,
            "Version",
            "Version",
        ),
        (
            "finkit-java (pom.xml)",
            JAVA_POM,
            "version",
            "project.version",
        ),
    ):
        xml_ver = read_xml_project_version(path, tag)
        if xml_ver is not None:
            match = "✅" if xml_ver == canonical else "❌"
            rows.append(
                (
                    package,
                    path.relative_to(ROOT).as_posix(),
                    xml_ver,
                    source,
                    match,
                )
            )
    return rows


def generate_all(criterion_dir: Path) -> dict[Path, str]:
    catalog = build_indicator_catalog()
    codes = parse_ffi_status_codes()
    canonical = read_workspace_version()
    bench_stats = collect_all_bench_stats(criterion_dir)
    version_rows = build_version_rows(canonical)

    # Parse additional modules for streaming, formula, features, pine
    streaming_catalog = build_streaming_catalog()
    formula_functions = parse_formula_functions()
    features_modules = parse_features_modules()
    pine_builtins = parse_pine_builtins()

    outputs = {
        OUT_INDICATORS: format_indicators_md(catalog),
        OUT_ERROR_CODES: format_error_codes_md(codes),
        OUT_VERSION_MATRIX: format_version_matrix_md(
            canonical, version_rows, len(bench_stats)
        ),
    }

    # Add streaming indicators doc if data available
    if streaming_catalog:
        outputs[OUT_STREAMING] = format_streaming_md(
            streaming_catalog, read_registered_streaming_count()
        )

    # Add formula functions doc if data available
    if formula_functions:
        outputs[OUT_FORMULA] = format_formula_md(formula_functions)

    # Add features modules doc if data available
    if features_modules:
        outputs[OUT_FEATURES] = format_features_md(features_modules)

    # Add Pine compatibility doc if data available
    if pine_builtins:
        outputs[OUT_PINE] = format_pine_md(pine_builtins)

    return outputs


def write_outputs(outputs: dict[Path, str]) -> None:
    GENERATED_DIR.mkdir(parents=True, exist_ok=True)
    for path, content in outputs.items():
        path.write_text(content, encoding="utf-8")
        print(f"Wrote {path.relative_to(ROOT)}")


def check_outputs(outputs: dict[Path, str]) -> list[str]:
    errors: list[str] = []
    for path, expected in outputs.items():
        if not path.is_file():
            errors.append(f"{path.relative_to(ROOT)}: missing (run --generate)")
            continue
        actual = path.read_text(encoding="utf-8")
        if actual != expected:
            errors.append(f"{path.relative_to(ROOT)}: out of date")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description="SSOT documentation generator")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument(
        "--generate",
        action="store_true",
        help="Write docs/generated/*.md from current sources",
    )
    group.add_argument(
        "--check",
        action="store_true",
        help="Verify generated docs match sources (exit 1 on mismatch)",
    )
    parser.add_argument(
        "--criterion-dir",
        default=str(DEFAULT_CRITERION_DIR),
        help="Criterion output directory for benchmark index count",
    )
    args = parser.parse_args()

    criterion_dir = Path(args.criterion_dir)
    if not criterion_dir.is_absolute():
        criterion_dir = ROOT / criterion_dir

    try:
        outputs = generate_all(criterion_dir)
    except (OSError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2

    if args.generate:
        write_outputs(outputs)
        return 0

    errors = check_outputs(outputs)
    if errors:
        print("SSOT check FAILED:", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        print("Run: python scripts/gen_ssot_docs.py --generate", file=sys.stderr)
        return 1

    print("OK: SSOT generated docs match sources")
    return 0


if __name__ == "__main__":
    sys.exit(main())
