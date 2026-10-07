#!/usr/bin/env python3
"""BindingSpec: the single declarative source of truth for the Python binding build surface.

Why this file exists
--------------------
The Python wheel build used to reach its shipping shape by *rewriting tracked
hand-written Rust source in place*. Nine separate scripts text-patched
``native_fast_path.rs``, ``lib.rs``, ``momentum.rs``, ``volume.rs``,
``formula_plan.rs``, ``finkit/__init__.py`` and even other build scripts. Two
consequences followed, both observed in CI:

1. Plain ``cargo check --workspace`` was **not** evidence that the shipped
   source compiles. The four wheel platforms failed on ``u16`` vs ``"var"``
   while the workspace stayed green, because the defect only existed after the
   transformation ran.
2. Every transformation was a copy of a *fixed canonical form* embedded in a
   migration script. The canonical text lived in code that rewrote files, so
   nobody could review "what the binding is supposed to look like" without
   replaying a mutation.

V4 plan Batch 2 (``docs/FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md`` §24)
asks for the opposite arrangement: one declarative spec, a single generator,
and a build that no longer edits tracked source. That is what this module is:

* :data:`HOT_PATHS` is the numeric-dispatch SSOT. One operation table drives the
  Rust match arms, the Rust fast-path guard, and the public Python facade call
  sites, so the three cannot drift apart.
* :data:`CANONICAL_FUNCTIONS`, :data:`RULE_SETS` and :data:`TRANGE` pin the
  canonical bodies that used to be produced by rewriting.
* :func:`verify` reports drift; :func:`apply` performs the rewrite and exists
  only so a deliberate future change has a mechanical path. The build calls
  :func:`verify` and performs no writes.

Every rule is expressed as an ``old -> new`` fragment pair, which makes both
directions checkable: ``new`` present means the transformation is in place,
``old`` still present means it is pending, and neither present means the anchor
disappeared and the spec itself needs review.
"""

from __future__ import annotations

import hashlib
import re
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

NATIVE = ROOT / "ffi" / "python-binding" / "src" / "native_fast_path.rs"
INIT = ROOT / "ffi" / "python-binding" / "finkit" / "__init__.py"
LIB = ROOT / "ffi" / "python-binding" / "src" / "lib.rs"
GENERATED = ROOT / "ffi" / "python-binding" / "src" / "generated.rs"
MOMENTUM = ROOT / "core" / "src" / "indicators" / "momentum.rs"
VOLUME = ROOT / "core" / "src" / "indicators" / "volume.rs"
FORMULA_PLAN = ROOT / "ffi" / "python-binding" / "src" / "formula_plan.rs"
SYNC_BINDINGS = ROOT / "scripts" / "sync_bindings.py"
INDICATOR_REGISTRY = ROOT / "docs" / "indicator_registry.json"

# Sources the wheel build must not rewrite. `generated.rs` is deliberately
# absent because the registry generator owns it, and `docs/ffi_registry.json`
# is absent for the same reason (enrichment regenerates it and the result is
# committed by a deliberate `--discover` change, not by a wheel build).
HANDWRITTEN_SOURCES: tuple[Path, ...] = (
    NATIVE,
    INIT,
    LIB,
    MOMENTUM,
    VOLUME,
    FORMULA_PLAN,
    SYNC_BINDINGS,
    INDICATOR_REGISTRY,
)


def fingerprints() -> dict[Path, str]:
    """Content fingerprints of the hand-written sources.

    CRLF and LF are normalised away: `Path.write_text` translates a newline to
    ``os.linesep`` on Windows, and a line-ending round trip is a different
    (also real) defect from a content rewrite. What the build-state gate must
    prove is that no *content* changed.
    """

    return {
        path: hashlib.sha256(
            path.read_bytes().replace(b"\r\n", b"\n")
        ).hexdigest()
        for path in HANDWRITTEN_SOURCES
    }


def mutated_sources(
    before: "dict[Path, str]", after: "dict[Path, str]"
) -> list[str]:
    """Return the hand-written sources whose content changed during a build."""

    return [
        str(path.relative_to(ROOT))
        for path in HANDWRITTEN_SOURCES
        if before.get(path) != after.get(path)
    ]


# --------------------------------------------------------------------------- #
# engine
# --------------------------------------------------------------------------- #
@dataclass(frozen=True)
class Rule:
    """One exact source fragment and its canonical replacement."""

    old: str
    new: str

    def state(self, text: str) -> str:
        """Return ``applied``, ``pending``, or ``anchor-lost`` for this rule."""

        if self.new in text:
            return "applied"
        if self.old in text:
            return "pending"
        return "anchor-lost"


def write_lf(path: Path, text: str) -> None:
    """Write ``text`` with LF endings, preserving any pre-existing CRLF."""

    raw = path.read_bytes()
    if b"\r\n" in raw and "\r\n" not in text:
        text = text.replace("\n", "\r\n")
    path.write_text(text, encoding="utf-8", newline="")


def function_segment(text: str, name: str) -> str:
    """Return the source span of ``fn <name>`` up to the next ``#[pyfunction]``."""

    marker = f"fn {name}"
    start = text.find(marker)
    if start < 0:
        raise KeyError(f"{name}: function not found")
    end = text.find("\n#[pyfunction", start + len(marker))
    return text[start:] if end < 0 else text[start:end]


def replace_function(text: str, name: str, rules: "tuple[Rule, ...]") -> tuple[str, int]:
    """Apply ``rules`` inside one function span. Idempotent by construction."""

    marker = f"fn {name}"
    start = text.find(marker)
    if start < 0:
        raise KeyError(f"{name}: function not found")
    next_attr = text.find("\n#[pyfunction", start + len(marker))
    end = len(text) if next_attr < 0 else next_attr
    segment = text[start:end]
    changed = 0
    for rule in rules:
        if rule.new in segment:
            continue
        count = segment.count(rule.old)
        if count != 1:
            raise ValueError(
                f"{name}: expected exactly one {rule.old!r} fragment, found {count}"
            )
        segment = segment.replace(rule.old, rule.new, 1)
        changed += 1
    return text[:start] + segment + text[end:], changed


# --------------------------------------------------------------------------- #
# hot-path numeric dispatch SSOT
# --------------------------------------------------------------------------- #
@dataclass(frozen=True)
class HotPath:
    """One private native hot path and its stable operation-id table.

    The ids are ABI: they are resolved by the public Python facade and matched
    directly in Rust, so an id may never be renumbered without updating both
    sides in the same change. ``ops`` is the only place they are written down.
    """

    rust_fn: str
    facade_calls: tuple[str, ...]
    ops: tuple[tuple[str, int], ...]
    guard: Rule | None = None

    def rust_rules(self) -> tuple[Rule, ...]:
        rules = [Rule("operation: &str,", "operation: u16,")]
        rules.extend(Rule(f'"{op}" =>', f"{idx} =>") for op, idx in self.ops)
        if self.guard is not None:
            rules.append(self.guard)
        rules.append(
            Rule(
                "unsupported fast operation {operation}",
                "unsupported fast operation id {operation}",
            )
        )
        return tuple(rules)

    def expected_ids(self) -> set[str]:
        return {str(idx) for _, idx in self.ops}


HOT_PATHS: tuple[HotPath, ...] = (
    HotPath(
        rust_fn="fast_unary_period",
        facade_calls=("_unary_period",),
        ops=(
            ("midpoint", 1),
            ("mom", 2),
            ("dema", 3),
            ("tema", 4),
            ("rsi", 5),
            ("roc", 6),
            ("cmo", 7),
        ),
    ),
    HotPath(
        rust_fn="fast_unary_period_scale",
        facade_calls=("_fast_unary_period_scale",),
        ops=(("stddev", 1), ("var", 2)),
        # The early fast-path guard compares the same `operation` value the
        # match arms dispatch on. Leaving it in string form after the signature
        # becomes `u16` is precisely the defect that failed all four wheel
        # platforms while the workspace stayed green.
        guard=Rule('if operation == "var" && timeperiod == 20 {', "if operation == 2 && timeperiod == 20 {"),
    ),
    HotPath(
        rust_fn="fast_binary_period",
        facade_calls=("_fast_binary_period",),
        ops=(("midprice", 1), ("correl", 2)),
    ),
    HotPath(
        rust_fn="fast_hlc_period",
        facade_calls=("_hlc_period",),
        ops=(
            ("willr", 1),
            ("adx", 2),
            ("cci", 3),
            ("plus_di", 4),
            ("minus_di", 5),
            ("atr", 6),
            ("natr", 7),
            ("adxr", 8),
        ),
    ),
)

# Operations reachable only through a string-first compatibility probe. They are
# intentionally string-keyed in the facade and therefore never carry numeric
# ids; recording them here keeps the coverage check below exact instead of
# silently tolerating an unwired numeric call site.
STRING_ONLY_FACADE_CALLS: tuple[str, ...] = (
    "_unary_period_scale_native",
    "_binary_period_native",
)


def verify_hot_paths(native_text: str, facade_text: str) -> list[str]:
    """Verify the private ABI is numeric and the facade ids agree with Rust."""

    violations: list[str] = []
    for hot in HOT_PATHS:
        try:
            segment = function_segment(native_text, hot.rust_fn)
        except KeyError as exc:
            violations.append(str(exc))
            continue
        for rule in hot.rust_rules():
            state = rule.state(segment)
            if state == "pending":
                violations.append(
                    f"{hot.rust_fn}: numeric-dispatch rule not applied "
                    f"(found {rule.old!r}, expected {rule.new!r})"
                )
            elif state == "anchor-lost":
                violations.append(
                    f"{hot.rust_fn}: anchor lost for rule {rule.old!r} -> {rule.new!r}"
                )
        if 'operation == "' in segment:
            violations.append(f"{hot.rust_fn}: string comparison against operation remains")
        if re.search(r'(?m)^\s*"[A-Za-z_0-9]+" =>', segment):
            violations.append(f"{hot.rust_fn}: string match arm remains")

        expected = hot.expected_ids()
        for call in hot.facade_calls:
            found = set(re.findall(rf"(?<!\w){re.escape(call)}\((\d+),", facade_text))
            if not found:
                violations.append(f"{hot.rust_fn}: no numeric call sites for facade {call}()")
                continue
            unknown = found - expected
            unused = expected - found
            if unknown:
                violations.append(
                    f"{hot.rust_fn}: facade {call}() passes ids {sorted(unknown)} "
                    f"with no Rust match arm (rust ids {sorted(expected)})"
                )
            if unused:
                violations.append(
                    f"{hot.rust_fn}: ids {sorted(unused)} are declared in the spec but "
                    f"unreachable through facade {call}()"
                )
    return violations


# --------------------------------------------------------------------------- #
# canonical Rust bodies (previously produced by in-place rewriting)
# --------------------------------------------------------------------------- #
@dataclass(frozen=True)
class CanonicalFunction:
    """A public Rust function whose body is pinned byte-for-byte."""

    label: str
    path: Path
    start_token: str
    next_doc: str
    canonical: str
    forbidden: tuple[str, ...] = ()

    def section(self, text: str) -> str | None:
        start = text.find(self.start_token)
        if start < 0:
            return None
        end = text.find(self.next_doc, start + len(self.start_token))
        if end < 0:
            return None
        return text[start:end].rstrip()


MFI_CANONICAL = '''pub fn mfi(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    period: usize,
) -> Result<Array1<f64>> {
    crate::math::mfi::mfi(high, low, close, volume, period)
}'''

AD_CANONICAL = '''pub fn ad(high: &[f64], low: &[f64], close: &[f64], volume: &[f64]) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;

    // `ad_into` writes every slot (cumulative AD, no warm-up), so an
    // uninitialized buffer avoids the full zero-fill pass `zeros` would add.
    let mut output = Array1::from(crate::utils::uninit_output(high.len()));
    crate::math::volume_kernels::ad_into(
        high,
        low,
        close,
        volume,
        output.as_slice_mut().expect("owned Array1 is contiguous"),
    )?;
    Ok(output)
}'''

AD_INTO_CANONICAL = '''pub fn ad_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    output: &mut [f64],
) -> Result<()> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;
    if output.len() != high.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as high".to_string(),
        });
    }
    crate::math::volume_kernels::ad_into(high, low, close, volume, output)
}'''

ADOSC_CANONICAL = '''pub fn adosc(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    fast_period: usize,
    slow_period: usize,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), slow_period)?;

    let mut output = Array1::<f64>::zeros(high.len());
    crate::math::volume_kernels::adosc_into(
        high,
        low,
        close,
        volume,
        fast_period,
        slow_period,
        output.as_slice_mut().expect("owned Array1 is contiguous"),
    )?;
    Ok(output)
}'''

OBV_CANONICAL = '''pub fn obv(close: &[f64], volume: &[f64]) -> Result<Array1<f64>> {
    if close.len() != volume.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "close and volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(close.len(), 1)?;

    let mut output = vec![0.0_f64; close.len()];
    crate::math::volume_kernels::obv_into(close, volume, &mut output)?;
    Ok(Array1::from_vec(output))
}'''

CANONICAL_FUNCTIONS: tuple[CanonicalFunction, ...] = (
    CanonicalFunction(
        label="momentum.rs public MFI",
        path=MOMENTUM,
        start_token="pub fn mfi(\n",
        next_doc="/// Minus Directional Indicator",
        canonical=MFI_CANONICAL,
        forbidden=("simd_typical_price", "pos_ring", "neg_ring"),
    ),
    CanonicalFunction(
        label="volume.rs public AD",
        path=VOLUME,
        start_token="pub fn ad(high:",
        next_doc="/// AD zero-copy variant",
        canonical=AD_CANONICAL,
        forbidden=("simd_ad_line",),
    ),
    CanonicalFunction(
        label="volume.rs public AD into",
        path=VOLUME,
        start_token="pub fn ad_into(\n",
        next_doc="/// Chaikin A/D Oscillator",
        canonical=AD_INTO_CANONICAL,
    ),
    CanonicalFunction(
        label="volume.rs public ADOSC",
        path=VOLUME,
        start_token="pub fn adosc(\n",
        # Bound the section at the zero-copy variant, exactly as the AD spec
        # above does. Without this the section swallowed `adosc_into` as well
        # and could never match a canonical body for `adosc` alone.
        next_doc="/// ADOSC zero-copy variant",
        canonical=ADOSC_CANONICAL,
        forbidden=("simd_ad_line", "cumulative = vec!"),
    ),
    CanonicalFunction(
        label="volume.rs public OBV",
        path=VOLUME,
        start_token="pub fn obv(close:",
        next_doc="/// Volume Profile 结果结构体",
        canonical=OBV_CANONICAL,
        forbidden=("simd_obv",),
    ),
)


def verify_canonical_functions() -> list[str]:
    violations: list[str] = []
    cache: dict[Path, str] = {}
    for spec in CANONICAL_FUNCTIONS:
        if spec.path not in cache:
            cache[spec.path] = spec.path.read_text(encoding="utf-8")
        text = cache[spec.path]
        section = spec.section(text)
        if section is None:
            violations.append(f"{spec.label}: function boundary not found")
            continue
        if section != spec.canonical:
            violations.append(
                f"{spec.label}: body drifted from the canonical kernel delegation"
            )
        for needle in spec.forbidden:
            if needle in section:
                violations.append(
                    f"{spec.label}: retired implementation fragment {needle!r} remains"
                )
    return violations


# --------------------------------------------------------------------------- #
# TRANGE: caller-owned output
# --------------------------------------------------------------------------- #
# The shipped shape allocates the NumPy destination uninitialised and fills it
# through the canonical `trange_into` kernel, so the previous `vec![0.0; n]`
# scratch buffer and its full zero-fill pass are gone.
TRANGE_CANONICAL = '''fn fast_trange<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let high = high.as_slice().map_err(value_error)?;
    let low = low.as_slice().map_err(value_error)?;
    let close = close.as_slice().map_err(value_error)?;
    validate_same_len(high.len(), low.len())?;
    validate_same_len(high.len(), close.len())?;
    let len = high.len();
    let output = unsafe { PyArray1::new(py, [len], false) };
    let output_addr = output.data() as usize;
    py.detach(|| unsafe {
        let output_ptr = output_addr as *mut f64;
        indicators::trange_into(
            high,
            low,
            close,
            std::slice::from_raw_parts_mut(output_ptr, len),
        )
    })
    .map_err(value_error)?;
    Ok(output)
}'''

TRANGE_REQUIRED: tuple[str, ...] = (
    "indicators::trange_into(",
    "validate_same_len(high.len(), low.len())?;",
    "validate_same_len(high.len(), close.len())?;",
)


# --------------------------------------------------------------------------- #
# rule sets over facade / lib.rs / formula plan / sync_bindings
# --------------------------------------------------------------------------- #
@dataclass(frozen=True)
class RuleSet:
    label: str
    path: Path
    rules: tuple[Rule, ...]
    required: tuple[str, ...] = ()
    forbidden: tuple[str, ...] = ()
    # When set, the forbidden needles are only searched inside this section.
    forbidden_scope: tuple[str, str] | None = None


SINGLE_WRITE_RULESET = RuleSet(
    label="Python hot facade single-write outputs",
    path=INIT,
    rules=(
        Rule(
            '''        if close.dtype == np.float32 and hasattr(_native, "_fast_sma_f32"):
            return _native._fast_sma_f32(close, timeperiod)
        return _native._fast_sma(close, timeperiod)
''',
            '''        result = np.empty_like(close)
        if close.dtype == np.float32 and hasattr(_native, "_fast_sma_f32_into"):
            _native._fast_sma_f32_into(close, result, timeperiod)
        else:
            _native._fast_sma_into(close, result, timeperiod)
        return result
''',
        ),
        Rule(
            '''        if close.dtype == np.float32 and hasattr(_native, "_fast_ema_f32"):
            return _native._fast_ema_f32(close, timeperiod)
        return _native._fast_ema(close, timeperiod)
''',
            '''        result = np.empty_like(close)
        if close.dtype == np.float32 and hasattr(_native, "_fast_ema_f32_into"):
            _native._fast_ema_f32_into(close, result, timeperiod)
        else:
            _native._fast_ema_into(close, result, timeperiod)
        return result
''',
        ),
        Rule(
            """        return _native._fast_wma(close, timeperiod)
""",
            """        result = np.empty_like(close)
        _native._fast_wma_into(close, result, timeperiod)
        return result
""",
        ),
        Rule(
            "        return _native._fast_obv(close, volume)\n",
            "        result = np.empty_like(close)\n"
            "        _native._fast_obv_into(close, volume, result)\n"
            "        return result\n",
        ),
    ),
    required=("result = np.empty_like(close)",),
)

BATCH_CONTRACT_RULESET = RuleSet(
    label="Python batch borrowed-input / ndarray-output contract",
    path=LIB,
    rules=(
        Rule(
            '''    let open_vec: Option<Vec<f64>> = open.as_ref().map(|arr| arr.as_array().to_vec());
    let high_vec: Option<Vec<f64>> = high.as_ref().map(|arr| arr.as_array().to_vec());
    let low_vec: Option<Vec<f64>> = low.as_ref().map(|arr| arr.as_array().to_vec());
    let volume_vec: Option<Vec<f64>> = volume.as_ref().map(|arr| arr.as_array().to_vec());
    let secondary_vec: Option<Vec<f64>> = secondary.as_ref().map(|arr| arr.as_array().to_vec());
''',
            '''    let open_slice = open
        .as_ref()
        .map(|arr| arr.as_slice())
        .transpose()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high_slice = high
        .as_ref()
        .map(|arr| arr.as_slice())
        .transpose()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low_slice = low
        .as_ref()
        .map(|arr| arr.as_slice())
        .transpose()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let volume_slice = volume
        .as_ref()
        .map(|arr| arr.as_slice())
        .transpose()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let secondary_slice = secondary
        .as_ref()
        .map(|arr| arr.as_slice())
        .transpose()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
''',
        ),
        Rule(
            '''            open_vec.as_deref(),
            high_vec.as_deref(),
            low_vec.as_deref(),
            close_slice,
            volume_vec.as_deref(),
            secondary_vec.as_deref(),
''',
            '''            open_slice,
            high_slice,
            low_slice,
            close_slice,
            volume_slice,
            secondary_slice,
''',
        ),
        Rule(
            '''        match value {
            IndicatorResult::Single(arr) => {
                dict.set_item(key, arr)?;
            }
            IndicatorResult::Double(a, b) => {
                dict.set_item(format!("{}_0", key), a)?;
                dict.set_item(format!("{}_1", key), b)?;
            }
            IndicatorResult::Triple(a, b, c) => {
                dict.set_item(format!("{}_0", key), a)?;
                dict.set_item(format!("{}_1", key), b)?;
                dict.set_item(format!("{}_2", key), c)?;
            }
            IndicatorResult::Quad(a, b, c, d) => {
                dict.set_item(format!("{}_0", key), a)?;
                dict.set_item(format!("{}_1", key), b)?;
                dict.set_item(format!("{}_2", key), c)?;
                dict.set_item(format!("{}_3", key), d)?;
            }
''',
            '''        match value {
            IndicatorResult::Single(arr) => {
                dict.set_item(key, PyArray1::from_vec(py, arr))?;
            }
            IndicatorResult::Double(a, b) => {
                dict.set_item(format!("{}_0", key), PyArray1::from_vec(py, a))?;
                dict.set_item(format!("{}_1", key), PyArray1::from_vec(py, b))?;
            }
            IndicatorResult::Triple(a, b, c) => {
                dict.set_item(format!("{}_0", key), PyArray1::from_vec(py, a))?;
                dict.set_item(format!("{}_1", key), PyArray1::from_vec(py, b))?;
                dict.set_item(format!("{}_2", key), PyArray1::from_vec(py, c))?;
            }
            IndicatorResult::Quad(a, b, c, d) => {
                dict.set_item(format!("{}_0", key), PyArray1::from_vec(py, a))?;
                dict.set_item(format!("{}_1", key), PyArray1::from_vec(py, b))?;
                dict.set_item(format!("{}_2", key), PyArray1::from_vec(py, c))?;
                dict.set_item(format!("{}_3", key), PyArray1::from_vec(py, d))?;
            }
''',
        ),
    ),
    required=(
        "dict.set_item(key, PyArray1::from_vec(py, arr))?;",
    ),
    forbidden=(".as_array().to_vec()",),
    forbidden_scope=("fn compute_indicators", "/// Result type for indicator computation."),
)

# --------------------------------------------------------------------------- #
# FormulaPlan canonical classifier / executor convergence
# --------------------------------------------------------------------------- #
# `formula_plan.rs` compiles common formula calls into canonical numeric kernels
# once, at compiled-formula construction time. The three surfaces must agree:
# every enum variant must be *produced* by the classifier and *consumed* by the
# executor. A variant produced but not executed is an unreachable-panic path; a
# variant executed but not produced is dead code. Neither is visible to the
# compiler, which is why this is checked structurally.
FORMULA_PLAN_ENUM = "enum CanonicalFormula"
FORMULA_PLAN_CLASSIFIER = re.compile(r"then_some\(CanonicalFormula::([A-Z][A-Za-z0-9]*)")
FORMULA_PLAN_EXECUTOR = re.compile(
    r"(?m)^\s*CanonicalFormula::([A-Z][A-Za-z0-9]*)\s*\{[^}]*\}\s*=>"
)


def _match_brace(text: str, open_index: int) -> int:
    depth = 0
    for index in range(open_index, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return index
    return -1


def formula_plan_sets(text: str) -> tuple[set[str], set[str], set[str]]:
    """Return the (enum variants, classifier outputs, executor arms) sets."""

    start = text.find(FORMULA_PLAN_ENUM)
    enum_names: set[str] = set()
    if start >= 0:
        brace = text.find("{", start)
        end = _match_brace(text, brace)
        if brace >= 0 and end > brace:
            body = text[brace + 1 : end]
            enum_names = set(re.findall(r"(?m)^\s+([A-Z][A-Za-z0-9]*)\s*\{", body))
    classifiers = set(FORMULA_PLAN_CLASSIFIER.findall(text))
    executors = set(FORMULA_PLAN_EXECUTOR.findall(text))
    return enum_names, classifiers, executors


def verify_formula_plan() -> list[str]:
    text = FORMULA_PLAN.read_text(encoding="utf-8")
    violations: list[str] = []
    if FORMULA_PLAN_ENUM not in text:
        return [f"FormulaPlan: {FORMULA_PLAN_ENUM!r} not found in {FORMULA_PLAN.name}"]
    enum_names, classifiers, executors = formula_plan_sets(text)
    if not classifiers or not executors:
        return [
            "FormulaPlan: classifier or executor arms could not be parsed; the "
            "convergence gate would be vacuous"
        ]
    for name in sorted(classifiers - executors):
        violations.append(
            f"FormulaPlan: classifier produces {name} but no executor arm consumes it"
        )
    for name in sorted(executors - classifiers):
        violations.append(
            f"FormulaPlan: executor arm {name} is unreachable (no classifier produces it)"
        )
    for name in sorted(enum_names - classifiers - executors):
        violations.append(
            f"FormulaPlan: enum variant {name} is neither classified nor executed"
        )
    return violations


SYNC_BINDINGS_RULESET = RuleSet(
    label="sync_bindings generator hooks",
    path=SYNC_BINDINGS,
    # The generator is richer than the historical migration patches: it merges
    # the core registry with the FFI SSOT so `--discover` cannot overwrite rich
    # metadata with a name-only stub, and it prefers the transient Python
    # overlay when the preparation workflow created one -- but only while that
    # overlay is genuinely newer than the checked-in SSOT, because a stale copy
    # silently shadows hand-authored bodies. These are invariants, not fragments
    # to rewrite.
    rules=(),
    required=(
        "from optimize_python_bindings import optimize_source as optimize_python_source",
        "optimize_python_source(text)",
        'core = ff.get("core_call", pub).split("::")[-1]',
        'impl_name = f"vec_{nm}_impl"',
        'PYTHON_REGISTRY_OVERLAY = ROOT / "target" / "python_registry_ssot.json"',
        "def _overlay_is_fresh(overlay: Path) -> bool:",
        "if _overlay_is_fresh(PYTHON_REGISTRY_OVERLAY):",
        "ffi_registry = json.loads(FFI_REG.read_text(encoding=\"utf-8\"))",
        "core_registry = (",
    ),
)

RULE_SETS: tuple[RuleSet, ...] = (
    SINGLE_WRITE_RULESET,
    BATCH_CONTRACT_RULESET,
    SYNC_BINDINGS_RULESET,
)


def verify_rule_sets() -> list[str]:
    violations: list[str] = []
    for spec in RULE_SETS:
        text = spec.path.read_text(encoding="utf-8")
        for rule in spec.rules:
            state = rule.state(text)
            if state == "pending":
                violations.append(
                    f"{spec.label}: rule not applied "
                    f"(found {rule.old!r}, expected {rule.new!r})"
                )
            elif state == "anchor-lost":
                violations.append(
                    f"{spec.label}: anchor lost for rule "
                    f"{rule.old!r} -> {rule.new!r}"
                )
        for needle in spec.required:
            if needle not in text:
                violations.append(f"{spec.label}: required fragment {needle!r} missing")
        if spec.forbidden_scope is not None:
            start_token, end_token = spec.forbidden_scope
            start = text.find(start_token)
            end = text.find(end_token, start + 1) if start >= 0 else -1
            scope = text[start:end] if start >= 0 and end > start else ""
            for needle in spec.forbidden:
                if needle in scope:
                    violations.append(
                        f"{spec.label}: forbidden fragment {needle!r} remains in "
                        f"{start_token!r}"
                    )
        else:
            for needle in spec.forbidden:
                if needle in text:
                    violations.append(
                        f"{spec.label}: forbidden fragment {needle!r} remains"
                    )
    return violations


# --------------------------------------------------------------------------- #
# CFO / TMF ownership boundary
# --------------------------------------------------------------------------- #
# CFO/TMF are carried by `docs/ffi_registry.json`, so `sync_bindings.py`
# emits their wrappers into the registry-owned `generated.rs`. The historical
# migration patched lib.rs instead; ownership is now verified, never rewritten.
CFO_TMF_SYMBOLS = ("fn chande_forecast_oscillator(", "fn twiggs_money_flow(")


def verify_cfo_tmf_ownership() -> list[str]:
    """CFO/TMF live in the registry-generated file and must stay single-owner.

    The historical migration script assumed the opposite boundary (wrappers in
    ``lib.rs`` because they are absent from the C-ABI registry). Measured
    reality: both symbols are carried by ``docs/ffi_registry.json``, so the
    registry generator emits them into ``generated.rs``, and a full
    ``sync_bindings.py --generate --lang python`` reproduces that file
    byte-for-byte. Declaring them in both places would be a duplicate-symbol
    compile error, and declaring them in neither would silently delete a public
    API, so the gate pins exactly one owner.
    """

    lib = LIB.read_text(encoding="utf-8")
    generated = GENERATED.read_text(encoding="utf-8") if GENERATED.exists() else ""
    violations: list[str] = []
    for symbol in CFO_TMF_SYMBOLS:
        if symbol in lib and symbol in generated:
            violations.append(
                f"CFO/TMF ownership: {symbol!r} is declared in both lib.rs and "
                "generated.rs (duplicate symbols)"
            )
        elif symbol in lib:
            violations.append(
                f"CFO/TMF ownership: {symbol!r} is declared in lib.rs, but the "
                "registry generator owns it in generated.rs"
            )
        elif symbol not in generated:
            violations.append(
                f"CFO/TMF ownership: {symbol!r} is declared in neither lib.rs nor "
                "generated.rs; the public API would disappear"
            )
    return violations


# --------------------------------------------------------------------------- #
# NumPy-direct return contract (shared with optimize_python_bindings)
# --------------------------------------------------------------------------- #
def verify_numpy_direct() -> list[str]:
    from optimize_python_bindings import optimize_source

    violations: list[str] = []
    for path in (GENERATED, LIB):
        if not path.exists():
            violations.append(f"NumPy-direct contract: {path} does not exist")
            continue
        source = path.read_bytes().decode("utf-8")
        optimized, count = optimize_source(source)
        if optimized != source:
            violations.append(
                f"NumPy-direct contract: {path} still has {count} numeric "
                "pyfunctions returning Python lists"
            )
    return violations


# --------------------------------------------------------------------------- #
# Extrema kernel convergence (architecture v3 round 6)
# --------------------------------------------------------------------------- #
# MIDPOINT, MIDPRICE and WILLR must all consume the one fused rolling-extrema
# visitor in `core/src/math/statistics.rs`. The migration that established this
# also deleted three binding-local extrema helpers, and a reintroduced local
# copy would silently change warm-up and NaN behavior relative to the public
# kernels — the same code computing the same statistic three different ways.
EXTREMA_KERNEL = ROOT / "core" / "src" / "math" / "statistics.rs"
EXTREMA_KERNEL_SIGNATURE = "pub(crate) fn rolling_minmax_visit("
EXTREMA_KERNEL_MARKERS = (
    # Two-tier design, and both tiers must survive:
    #   * the cached-index fast path, which rescans only when an extrema leaves
    #     the window (a per-bar rescan here is what used to make TA_MAX/TA_MIN
    #     slow on long series), and
    #   * the monotonic ring fallback for oversized windows, which bounds the
    #     worst case the cached index cannot.
    # The markers are phrases from the live kernel doc; they replaced the older
    # pair ("Expire stale fronts before insertion" / "Large-window compatibility
    # fallback"), which described the pre-rewrite implementation and had gone
    # stale -- leaving this gate permanently red and therefore ignored.
    "rescanning only when an extrema leaves the window",
    "Monotonic ring",
    "fn rolling_minmax_ring(",
)
EXTREMA_CONSUMERS = (
    (
        ROOT / "core" / "src" / "indicators" / "overlap.rs",
        "pub fn midpoint_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()>",
    ),
    (
        ROOT / "core" / "src" / "indicators" / "overlap.rs",
        "pub fn midprice_into(high: &[f64], low: &[f64], period: usize, output: &mut [f64]) -> Result<()>",
    ),
    (
        ROOT / "core" / "src" / "indicators" / "momentum.rs",
        "pub fn willr_into(",
    ),
)
# Helpers that existed before the convergence and must not come back.
BINDING_LOCAL_EXTREMA = ("midpoint_vec(", "willr_vec(", "Sliding extrema")


def verify_extrema_convergence() -> list[str]:
    violations: list[str] = []
    kernel = EXTREMA_KERNEL.read_text(encoding="utf-8")
    if EXTREMA_KERNEL_SIGNATURE not in kernel:
        violations.append(
            f"Extrema convergence: {EXTREMA_KERNEL_SIGNATURE!r} missing from "
            "core/src/math/statistics.rs"
        )
    for marker in EXTREMA_KERNEL_MARKERS:
        if marker not in kernel:
            violations.append(f"Extrema convergence: extrema kernel lost {marker!r}")

    seen: dict[Path, str] = {}
    for path, signature in EXTREMA_CONSUMERS:
        if path not in seen:
            seen[path] = path.read_text(encoding="utf-8")
        text = seen[path]
        if signature not in text:
            violations.append(
                f"Extrema convergence: {signature!r} missing from "
                f"{path.relative_to(ROOT)}"
            )
        elif "rolling_minmax_visit(" not in text:
            violations.append(
                f"Extrema convergence: {path.relative_to(ROOT)} declares a "
                "caller-owned extrema kernel but never calls rolling_minmax_visit"
            )

    binding = NATIVE.read_text(encoding="utf-8")
    for helper in BINDING_LOCAL_EXTREMA:
        if helper in binding:
            violations.append(
                f"Extrema convergence: binding-local extrema helper {helper!r} is "
                "back; MIDPOINT/MIDPRICE/WILLR must share the core visitor"
            )
    return violations


# --------------------------------------------------------------------------- #
# aggregate entry points
# --------------------------------------------------------------------------- #
def verify() -> list[str]:
    """Return every drift violation across the whole binding spec."""

    native = NATIVE.read_text(encoding="utf-8")
    facade = INIT.read_text(encoding="utf-8")
    violations: list[str] = []
    violations.extend(verify_hot_paths(native, facade))
    violations.extend(verify_canonical_functions())
    try:
        trange_segment = function_segment(native, "fast_trange")
    except KeyError as exc:
        violations.append(str(exc))
    else:
        if trange_segment.rstrip() != TRANGE_CANONICAL:
            violations.append(
                "TRANGE direct output: fast_trange drifted from the caller-owned "
                "trange_into contract"
            )
        for needle in TRANGE_REQUIRED:
            if needle not in trange_segment:
                violations.append(
                    f"TRANGE direct output: required fragment {needle!r} missing"
                )
    violations.extend(verify_formula_plan())
    violations.extend(verify_extrema_convergence())
    violations.extend(verify_rule_sets())
    violations.extend(verify_cfo_tmf_ownership())
    violations.extend(verify_numpy_direct())
    return violations


def apply() -> int:
    """Rewrite tracked sources so they match the spec. Development path only.

    The build never calls this. It exists so a deliberate future change to the
    binding surface has one mechanical route from "spec edited" to "source
    updated", instead of a new ad-hoc patcher script.
    """

    changed = 0

    native = NATIVE.read_text(encoding="utf-8")
    native_changed = 0
    for hot in HOT_PATHS:
        native, count = replace_function(native, hot.rust_fn, hot.rust_rules())
        native_changed += count
    start = native.find("fn fast_trange")
    if start < 0:
        raise SystemExit("TRANGE direct output: function not found")
    next_attr = native.find("\n#[pyfunction", start + len("fn fast_trange"))
    end = len(native) if next_attr < 0 else next_attr
    trange = native[start:end]
    body_end = trange.rstrip()
    if body_end != TRANGE_CANONICAL:
        trailing = trange[len(body_end) :]
        native = native[:start] + TRANGE_CANONICAL + trailing + native[end:]
        native_changed += 1
    if native_changed:
        write_lf(NATIVE, native)
        changed += native_changed

    for spec in RULE_SETS:
        text = spec.path.read_text(encoding="utf-8")
        spec_changed = 0
        for rule in spec.rules:
            if rule.new in text:
                continue
            count = text.count(rule.old)
            if count != 1:
                raise SystemExit(
                    f"{spec.label}: expected exactly one {rule.old!r} fragment, "
                    f"found {count}"
                )
            text = text.replace(rule.old, rule.new, 1)
            spec_changed += 1
        if spec_changed:
            write_lf(spec.path, text)
            changed += spec_changed

    for spec in CANONICAL_FUNCTIONS:
        text = spec.path.read_text(encoding="utf-8")
        start = text.find(spec.start_token)
        if start < 0:
            raise SystemExit(f"{spec.label}: function not found")
        end = text.find(spec.next_doc, start + len(spec.start_token))
        if end < 0:
            raise SystemExit(f"{spec.label}: function boundary not found")
        current = text[start:end]
        # The section deliberately keeps the blank separator line that precedes
        # `next_doc`; only the body itself is owned by the spec.
        body_end = current.rstrip()
        trailing = current[len(body_end) :]
        if body_end != spec.canonical:
            text = text[:start] + spec.canonical + trailing + text[end:]
            write_lf(spec.path, text)
            changed += 1

    # CFO/TMF live in the registry-owned generated file; regenerate it with
    # `sync_bindings.py --generate --lang python` instead of patching lib.rs.
    remaining = verify_cfo_tmf_ownership()
    if remaining:
        raise SystemExit("\n".join(remaining))

    return changed


if __name__ == "__main__":
    import sys as _sys

    if "--apply" in _sys.argv:
        _changed = apply()
        print(f"binding spec applied: {_changed} source edit(s); re-run the verifier")
        raise SystemExit(0)
    _violations = verify()
    for _item in _violations:
        print(f"::error title=Python binding spec::{_item}")
    print(f"binding spec violations: {len(_violations)}")
    raise SystemExit(1 if _violations else 0)
