#!/usr/bin/env python3
"""Generate the Alpha158 numeric reference that finkit's factor library is checked against.

# Why this file exists

`docs/competitive-analysis/finkit-全量覆盖与工业级收敛方案-2026-09-23.md` (M0-3) requires
"Alpha158：158 因子全部可构建，与 Qlib 参考输出数值对照（容差 1e-8）全绿".  A parity
claim is only worth what its reference is worth, so the reference is not written
from memory and not re-derived from a description of Qlib.  It is a *transcription*
of Qlib's own source, executed on Qlib's own substrate:

| What                         | Transcribed from                                                       |
| ---------------------------- | ---------------------------------------------------------------------- |
| the 158 factor expressions   | `qlib/contrib/data/loader.py` → `Alpha158DL.get_feature_config`         |
| rolling-operator semantics   | `qlib/data/ops.py` → `Rolling`, `Ref`, `Quantile`, `Rank`, `Corr`       |
| the Cython rolling kernels   | `qlib/data/_libs/rolling.pyx` → `Mean`, `Slope`, `Resi`, `Rsquare`      |
| the execution substrate      | pandas — which is what `Rolling._load_internal` itself calls            |

Every operator below carries a `qlib:` citation naming the class it mirrors.  The
one deliberate substitution is the substrate: Qlib's `Rolling._load_internal` is
`getattr(series.rolling(N, min_periods=1), func)()`, so evaluating the same
expressions with pandas' `rolling` is not a re-implementation of Qlib's
semantics — it is the same call Qlib makes.  What is *not* reproduced is Qlib's
data plumbing (`Expression.load`, `MemCache`, `_load_internal`'s caching), which
has no bearing on the numbers.

# The warm-up convention, and why the comparison is stated as it is

`qlib/data/ops.py` builds every rolling window with `min_periods=1`:

    series = getattr(series.rolling(self.N, min_periods=1), self.func)()
    # series.iloc[:self.N-1] = np.nan      <-- commented out in Qlib's source

so Qlib reports a *partial-window* value for the first `N-1` bars.  finkit's
rolling kernels deliberately do the opposite: a window is valid only when it is
full, otherwise the bar is NaN.  That convention is load-bearing elsewhere in
finkit (it is what `math::leading_warmup` and the degenerate-period policy are
built on), so it is not changed for Alpha158.

The parity gate therefore asserts two separate, independently falsifiable
things, instead of pretending one of them away:

1. on the common support — every bar where finkit's window is full — the two
   implementations agree to 1e-8; and
2. on the warm-up bars, finkit is NaN exactly where its window is incomplete.

Claim (2) is what makes the divergence a *documented convention* rather than an
excuse: it pins the shape of the difference, so a genuine algorithmic error
during warm-up would still fail.

# Usage

    python scripts/gen_alpha158_reference.py            # regenerate the contract
    python scripts/gen_alpha158_reference.py --check    # verify it is up to date
    python scripts/gen_alpha158_reference.py --self-test  # exercise the evaluator
"""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
from pathlib import Path

import numpy as np
import pandas as pd

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / "tests" / "golden" / "alpha158" / "reference_v1.json"
#: The Rust side of the same transliteration; `--check-table` keeps it in step.
TABLE = ROOT / "core" / "src" / "factors" / "builtin" / "alpha158.rs"

#: The Qlib revision the transcriptions below were read from.
QLIB_REVISION = "microsoft/qlib@main (fetched 2026-09-23)"

#: Rolling windows used by `Alpha158DL.get_feature_config`'s default `rolling` block.
WINDOWS = (5, 10, 20, 30, 60)

#: Qlib's `Rsquare` and `Corr` NaN out windows whose rolling std is "close to 0".
#: Transcribed verbatim from `qlib/data/ops.py`:
#:     series.loc[np.isclose(_series.rolling(self.N, min_periods=1).std(), 0, atol=2e-05)] = np.nan
DEGENERATE_STD_ATOL = 2e-05


# ---------------------------------------------------------------------------
# Operator layer — one function per Qlib operator, each citing its source.
# ---------------------------------------------------------------------------


def _rolling(series: pd.Series, n: int) -> pd.core.window.rolling.Rolling:
    """The window every Qlib rolling operator is built on.

    `qlib/data/ops.py::Rolling._load_internal` — `min_periods=1`, with the
    `series.iloc[:self.N-1] = np.nan` line commented out in Qlib's own source.
    """
    return series.rolling(n, min_periods=1)


def op_ref(series: pd.Series, n: int) -> pd.Series:
    """`qlib/data/ops.py::Ref` — `series.shift(N)`; N == 0 broadcasts the first bar."""
    if n == 0:
        return pd.Series(series.iloc[0], index=series.index)
    return series.shift(n)


def op_mean(series: pd.Series, n: int) -> pd.Series:
    """`qlib/data/ops.py::Mean` — `rolling(N, min_periods=1).mean()`."""
    return _rolling(series, n).mean()


def op_sum(series: pd.Series, n: int) -> pd.Series:
    """`qlib/data/ops.py::Sum` — `rolling(N, min_periods=1).sum()`."""
    return _rolling(series, n).sum()


def op_std(series: pd.Series, n: int) -> pd.Series:
    """`qlib/data/ops.py::Std` — `rolling(N, min_periods=1).std()`, i.e. ddof=1."""
    return _rolling(series, n).std()


def op_max(series: pd.Series, n: int) -> pd.Series:
    """`qlib/data/ops.py::Max` — `rolling(N, min_periods=1).max()`."""
    return _rolling(series, n).max()


def op_min(series: pd.Series, n: int) -> pd.Series:
    """`qlib/data/ops.py::Min` — `rolling(N, min_periods=1).min()`."""
    return _rolling(series, n).min()


def op_quantile(series: pd.Series, n: int, q: float) -> pd.Series:
    """`qlib/data/ops.py::Quantile` — `rolling(N, min_periods=1).quantile(q)`.

    pandas' default interpolation is `linear`, matching numpy's `method="linear"`
    and finkit's `QuantileInterpolation::Linear`.
    """
    return _rolling(series, n).quantile(q)


def op_rank(series: pd.Series, n: int) -> pd.Series:
    """`qlib/data/ops.py::Rank` — `rolling(N, min_periods=1).rank(pct=True)`.

    `pct=True` divides the average (tie-aware) rank by the number of observations,
    so the result lives in `(0, 1]`.
    """
    rolling = _rolling(series, n)
    if hasattr(rolling, "rank"):
        return rolling.rank(pct=True)
    raise RuntimeError("pandas is too old for Rolling.rank; Qlib falls back to a python loop")


def op_idxmax(series: pd.Series, n: int) -> pd.Series:
    """`qlib/data/ops.py::IdxMax` — `rolling(N, min_periods=1).apply(argmax + 1)`.

    The `+ 1` is Qlib's, not an off-by-one: the published value is 1-based.
    """
    return _rolling(series, n).apply(lambda x: x.argmax() + 1, raw=True)


def op_idxmin(series: pd.Series, n: int) -> pd.Series:
    """`qlib/data/ops.py::IdxMin` — `rolling(N, min_periods=1).apply(argmin + 1)`."""
    return _rolling(series, n).apply(lambda x: x.argmin() + 1, raw=True)


def _ols_residual_and_r2(window: np.ndarray) -> tuple[float, float]:
    """OLS of `window` against `idx = 1..len(window)`, returning (residual, r²).

    `qlib/data/_libs/rolling.pyx::Slope` accumulates `x_sum += window` for each new
    observation, so the newest bar sits at x = `window` and the oldest at
    x = `window - k + 1`.  Both the slope and r² are invariant to that shift, and
    the residual is the last point's deviation from the fitted line, which is too.

    `Rsquare` returns `rvalue * rvalue` where `rvalue` is the correlation between
    the values and their indices; `Resi` returns `val - (slope * window + interp)`.
    """
    k = window.shape[0]
    if k < 2:
        return math.nan, math.nan
    x = np.arange(1, k + 1, dtype=np.float64)
    x_mean = x.mean()
    y_mean = window.mean()
    sxx = ((x - x_mean) ** 2).sum()
    sxy = ((x - x_mean) * (window - y_mean)).sum()
    syy = ((window - y_mean) ** 2).sum()
    slope = sxy / sxx
    intercept = y_mean - slope * x_mean
    residual = window[-1] - (slope * k + intercept)
    r2 = (sxy * sxy) / (sxx * syy) if syy > 0.0 else math.nan
    return residual, r2


def op_slope(series: pd.Series, n: int) -> pd.Series:
    """`qlib/data/_libs/rolling.pyx::Slope` — OLS slope against `idx = 1..N`.

    The kernel's own formula is
    `(N*xy_sum - x_sum*y_sum) / (N*x2_sum - x_sum*x_sum)` with x = 1..N, which is
    the textbook OLS slope; `_slope_of` states it in that form rather than
    reusing the residual helper, so the two transcriptions stay independent.
    """
    return _rolling(series, n).apply(_slope_of, raw=True)


def _slope_of(window: np.ndarray) -> float:
    k = window.shape[0]
    if k < 2:
        return math.nan
    x = np.arange(1, k + 1, dtype=np.float64)
    n = float(k)
    x_sum = x.sum()
    x2_sum = (x * x).sum()
    y_sum = window.sum()
    xy_sum = (x * window).sum()
    return (n * xy_sum - x_sum * y_sum) / (n * x2_sum - x_sum * x_sum)


def op_rsquare(series: pd.Series, n: int) -> pd.Series:
    """`qlib/data/ops.py::Rsquare` + `qlib/data/_libs/rolling.pyx::Rsquare`.

    Two steps, both from Qlib: the kernel's `rvalue * rvalue`, then the
    near-degenerate-window guard
    `series.loc[np.isclose(_rolling(series, n).std(), 0, atol=2e-05)] = np.nan`.
    """
    raw = _rolling(series, n).apply(lambda w: _ols_residual_and_r2(w)[1], raw=True)
    degenerate = np.isclose(_rolling(series, n).std(), 0, atol=DEGENERATE_STD_ATOL)
    raw = raw.copy()
    raw[degenerate] = np.nan
    return raw


def op_resi(series: pd.Series, n: int) -> pd.Series:
    """`qlib/data/ops.py::Resi` + `qlib/data/_libs/rolling.pyx::Resi`.

    The residual of the *newest* bar in the window from the fitted line.
    """
    return _rolling(series, n).apply(lambda w: _ols_residual_and_r2(w)[0], raw=True)


def op_corr(left: pd.Series, right: pd.Series, n: int) -> pd.Series:
    """`qlib/data/ops.py::Corr` over `PairRolling`.

    `PairRolling._load_internal` is `left.rolling(N, min_periods=1).corr(right)`;
    `Corr` then NaNs out windows where *either* side has a near-zero std:

        res.loc[
            np.isclose(left.rolling(N, min_periods=1).std(), 0, atol=2e-05)
            | np.isclose(right.rolling(N, min_periods=1).std(), 0, atol=2e-05)
        ] = np.nan
    """
    res = _rolling(left, n).corr(right).copy()
    degenerate = np.isclose(_rolling(left, n).std(), 0, atol=DEGENERATE_STD_ATOL) | np.isclose(
        _rolling(right, n).std(), 0, atol=DEGENERATE_STD_ATOL
    )
    res[degenerate] = np.nan
    return res


def op_abs(series: pd.Series) -> pd.Series:
    """`qlib/data/ops.py::Abs` via `NpElemOperator` — `np.abs`."""
    return np.abs(series)


def op_log(series: pd.Series) -> pd.Series:
    """`qlib/data/ops.py::Log` via `NpElemOperator` — `np.log`, natural log."""
    with np.errstate(divide="ignore", invalid="ignore"):
        return pd.Series(np.log(series.to_numpy(dtype=np.float64)), index=series.index)


def op_greater(left: pd.Series, right: pd.Series) -> pd.Series:
    """`qlib/data/ops.py::Greater` — `np.maximum`, i.e. element-wise max, not a boolean."""
    return np.maximum(left, right)


def op_less(left: pd.Series, right: pd.Series) -> pd.Series:
    """`qlib/data/ops.py::Less` — `np.minimum`, i.e. element-wise min, not a boolean."""
    return np.minimum(left, right)


# ---------------------------------------------------------------------------
# Expression evaluator for Qlib's field strings.
# ---------------------------------------------------------------------------


class _Parser:
    """Recursive-descent parser for the subset of Qlib's expression grammar that
    `Alpha158DL.get_feature_config` emits.

    Grammar (precedence low → high):

        comparison := additive (('>' | '<') additive)?
        additive   := multiplicative (('+' | '-') multiplicative)*
        multiplicative := unary (('*' | '/') unary)*
        unary      := ('-' | '+')? primary
        primary    := NUMBER | '$' IDENT | IDENT '(' args ')' | '(' comparison ')'
    """

    def __init__(self, text: str, fields: dict[str, pd.Series]) -> None:
        self.text = text
        self.pos = 0
        self.fields = fields

    def parse(self) -> pd.Series:
        value = self.comparison()
        self._skip_ws()
        if self.pos != len(self.text):
            raise ValueError(f"trailing input at {self.pos}: {self.text[self.pos:]!r}")
        return value

    # -- helpers ------------------------------------------------------------

    def _skip_ws(self) -> None:
        while self.pos < len(self.text) and self.text[self.pos].isspace():
            self.pos += 1

    def _peek(self) -> str:
        self._skip_ws()
        return self.text[self.pos] if self.pos < len(self.text) else ""

    def _eat(self, literal: str) -> bool:
        self._skip_ws()
        if self.text.startswith(literal, self.pos):
            self.pos += len(literal)
            return True
        return False

    def _expect(self, literal: str) -> None:
        if not self._eat(literal):
            raise ValueError(f"expected {literal!r} at {self.pos} in {self.text!r}")

    def _ident(self) -> str:
        self._skip_ws()
        start = self.pos
        while self.pos < len(self.text) and (self.text[self.pos].isalnum() or self.text[self.pos] == "_"):
            self.pos += 1
        if start == self.pos:
            raise ValueError(f"expected identifier at {self.pos} in {self.text!r}")
        return self.text[start : self.pos]

    # -- grammar ------------------------------------------------------------

    def comparison(self) -> pd.Series:
        left = self.additive()
        for token, combine in ((">", np.greater), ("<", np.less)):
            if self._eat(token):
                right = self.additive()
                # Qlib compares float Series, so NaN compares False.  finkit's
                # `BINARY:Gt`/`BINARY:Lt` kernels emit 1.0/0.0 with the same
                # NaN-as-False rule, so the reference emits 0.0/1.0 as floats.
                return pd.Series(
                    combine(_as_array(left), _as_array(right)).astype(np.float64),
                    index=self._index(),
                )
        return left

    def _index(self) -> pd.Index:
        return next(iter(self.fields.values())).index

    def additive(self) -> pd.Series:
        value = self.multiplicative()
        while True:
            if self._eat("+"):
                value = value + self.multiplicative()
            elif self._eat("-"):
                value = value - self.multiplicative()
            else:
                return value

    def multiplicative(self) -> pd.Series:
        value = self.unary()
        while True:
            if self._eat("*"):
                value = value * self.unary()
            elif self._eat("/"):
                value = value / self.unary()
            else:
                return value

    def unary(self) -> pd.Series:
        if self._eat("-"):
            return -self.unary()
        if self._eat("+"):
            return self.unary()
        return self.primary()

    def primary(self):
        char = self._peek()
        if char == "(":
            self._expect("(")
            value = self.comparison()
            self._expect(")")
            return value
        if char == "$":
            self._expect("$")
            name = self._ident().lower()
            if name not in self.fields:
                raise ValueError(f"unknown field ${name} in {self.text!r}")
            return self.fields[name]
        if char.isdigit() or char == ".":
            # A literal stays a plain float so it can be read back as an operator
            # parameter (`Ref($close, 5)`) without round-tripping through a Series.
            # Arithmetic with a Series broadcasts it.
            return self._number()

        name = self._ident()
        self._expect("(")
        args: list = []
        if not self._eat(")"):
            args.append(self.comparison())
            while self._eat(","):
                args.append(self.comparison())
            self._expect(")")
        return _apply(name, args)

    def _number(self) -> float:
        self._skip_ws()
        start = self.pos
        while self.pos < len(self.text) and (self.text[self.pos].isdigit() or self.text[self.pos] in ".eE+-"):
            # `1e-12` must stay one token, but the `-` in `x-1` must not be eaten.
            if self.text[self.pos] in "+-" and self.pos > start and self.text[self.pos - 1] not in "eE":
                break
            self.pos += 1
        return float(self.text[start : self.pos])


def _as_array(value) -> np.ndarray:
    """Coerce a parsed operand — Series or float literal — to a float array."""
    if isinstance(value, pd.Series):
        return value.to_numpy(dtype=np.float64)
    return np.asarray(value, dtype=np.float64)


def _apply(name: str, args: list) -> pd.Series:
    """Dispatch a Qlib operator name onto the transcription above."""
    if name == "Ref":
        return op_ref(args[0], int(args[1]))
    if name == "Mean":
        return op_mean(args[0], int(args[1]))
    if name == "Sum":
        return op_sum(args[0], int(args[1]))
    if name == "Std":
        return op_std(args[0], int(args[1]))
    if name == "Max":
        return op_max(args[0], int(args[1]))
    if name == "Min":
        return op_min(args[0], int(args[1]))
    if name == "Quantile":
        return op_quantile(args[0], int(args[1]), float(args[2]))
    if name == "Rank":
        return op_rank(args[0], int(args[1]))
    if name == "IdxMax":
        return op_idxmax(args[0], int(args[1]))
    if name == "IdxMin":
        return op_idxmin(args[0], int(args[1]))
    if name == "Slope":
        return op_slope(args[0], int(args[1]))
    if name == "Rsquare":
        return op_rsquare(args[0], int(args[1]))
    if name == "Resi":
        return op_resi(args[0], int(args[1]))
    if name == "Corr":
        return op_corr(args[0], args[1], int(args[2]))
    if name == "Abs":
        return op_abs(args[0])
    if name == "Log":
        return op_log(args[0])
    if name == "Greater":
        return op_greater(args[0], args[1])
    if name == "Less":
        return op_less(args[0], args[1])
    raise ValueError(f"unmapped Qlib operator {name!r}")


def evaluate(expression: str, fields: dict[str, pd.Series]) -> pd.Series:
    """Evaluate one Qlib feature expression into a float Series.

    A bare literal (`1e-12`) parses to a float, so it is broadcast across the
    field index here; everything else is already a Series.
    """
    value = _Parser(expression, fields).parse()
    if isinstance(value, pd.Series):
        return value
    return pd.Series(value, index=next(iter(fields.values())).index, dtype=np.float64)


# ---------------------------------------------------------------------------
# The 158 factors, transcribed from `Alpha158DL.get_feature_config`.
# ---------------------------------------------------------------------------


def alpha158_expressions() -> list[tuple[str, str]]:
    """Return `(name, qlib_expression)` for all 158 Alpha158 factors.

    Every string is a verbatim copy of the one Qlib builds, including the
    `+1e-12` guards and the `Ref($close, 1)` spelling.  Order follows Qlib:
    kbar, then price, then rolling.
    """
    fields: list[tuple[str, str]] = []

    # qlib/contrib/data/loader.py — the `if "kbar" in config:` block.
    fields += [
        ("KMID", "($close-$open)/$open"),
        ("KLEN", "($high-$low)/$open"),
        ("KMID2", "($close-$open)/($high-$low+1e-12)"),
        ("KUP", "($high-Greater($open, $close))/$open"),
        ("KUP2", "($high-Greater($open, $close))/($high-$low+1e-12)"),
        ("KLOW", "(Less($open, $close)-$low)/$open"),
        ("KLOW2", "(Less($open, $close)-$low)/($high-$low+1e-12)"),
        ("KSFT", "(2*$close-$high-$low)/$open"),
        ("KSFT2", "(2*$close-$high-$low)/($high-$low+1e-12)"),
    ]

    # qlib/contrib/data/loader.py — the `if "price" in config:` block, with the
    # default `windows=[0]` and `feature=["OPEN", "HIGH", "LOW", "VWAP"]`.
    for field in ("open", "high", "low", "vwap"):
        fields.append((f"{field.upper()}0", f"${field}/$close"))

    # qlib/contrib/data/loader.py — the `if "rolling" in config:` block, default
    # windows [5, 10, 20, 30, 60] and no `include`/`exclude` filter, so all 29
    # operators are emitted.  29 x 5 = 145, and 9 + 4 + 145 = 158.
    for d in WINDOWS:
        fields += [
            (f"ROC{d}", f"Ref($close, {d})/$close"),
            (f"MA{d}", f"Mean($close, {d})/$close"),
            (f"STD{d}", f"Std($close, {d})/$close"),
            (f"BETA{d}", f"Slope($close, {d})/$close"),
            (f"RSQR{d}", f"Rsquare($close, {d})"),
            (f"RESI{d}", f"Resi($close, {d})/$close"),
            (f"MAX{d}", f"Max($high, {d})/$close"),
            (f"MIN{d}", f"Min($low, {d})/$close"),
            (f"QTLU{d}", f"Quantile($close, {d}, 0.8)/$close"),
            (f"QTLD{d}", f"Quantile($close, {d}, 0.2)/$close"),
            (f"RANK{d}", f"Rank($close, {d})"),
            (f"RSV{d}", f"($close-Min($low, {d}))/(Max($high, {d})-Min($low, {d})+1e-12)"),
            (f"IMAX{d}", f"IdxMax($high, {d})/{d}"),
            (f"IMIN{d}", f"IdxMin($low, {d})/{d}"),
            (f"IMXD{d}", f"(IdxMax($high, {d})-IdxMin($low, {d}))/{d}"),
            (f"CORR{d}", f"Corr($close, Log($volume+1), {d})"),
            (f"CORD{d}", f"Corr($close/Ref($close,1), Log($volume/Ref($volume, 1)+1), {d})"),
            (f"CNTP{d}", f"Mean($close>Ref($close, 1), {d})"),
            (f"CNTN{d}", f"Mean($close<Ref($close, 1), {d})"),
            (f"CNTD{d}", f"Mean($close>Ref($close, 1), {d})-Mean($close<Ref($close, 1), {d})"),
            (
                f"SUMP{d}",
                f"Sum(Greater($close-Ref($close, 1), 0), {d})"
                f"/(Sum(Abs($close-Ref($close, 1)), {d})+1e-12)",
            ),
            (
                f"SUMN{d}",
                f"Sum(Greater(Ref($close, 1)-$close, 0), {d})"
                f"/(Sum(Abs($close-Ref($close, 1)), {d})+1e-12)",
            ),
            (
                f"SUMD{d}",
                f"(Sum(Greater($close-Ref($close, 1), 0), {d})"
                f"-Sum(Greater(Ref($close, 1)-$close, 0), {d}))"
                f"/(Sum(Abs($close-Ref($close, 1)), {d})+1e-12)",
            ),
            (f"VMA{d}", f"Mean($volume, {d})/($volume+1e-12)"),
            (f"VSTD{d}", f"Std($volume, {d})/($volume+1e-12)"),
            (
                f"WVMA{d}",
                f"Std(Abs($close/Ref($close,1)-1)*$volume, {d})"
                f"/(Mean(Abs($close/Ref($close,1)-1)*$volume, {d})+1e-12)",
            ),
            (
                f"VSUMP{d}",
                f"Sum(Greater($volume-Ref($volume, 1), 0), {d})"
                f"/(Sum(Abs($volume-Ref($volume, 1)), {d})+1e-12)",
            ),
            (
                f"VSUMN{d}",
                f"Sum(Greater(Ref($volume, 1)-$volume, 0), {d})"
                f"/(Sum(Abs($volume-Ref($volume, 1)), {d})+1e-12)",
            ),
            (
                f"VSUMD{d}",
                f"(Sum(Greater($volume-Ref($volume, 1), 0), {d})"
                f"-Sum(Greater(Ref($volume, 1)-$volume, 0), {d}))"
                f"/(Sum(Abs($volume-Ref($volume, 1)), {d})+1e-12)",
            ),
        ]
    return fields


# ---------------------------------------------------------------------------
# The Qlib -> finkit transliteration, so the Rust table cannot drift from the
# expressions this file evaluates.
# ---------------------------------------------------------------------------

#: Qlib operator -> finkit function.
#:
#: Every entry is a *renaming*, not a redefinition: the reference below evaluates
#: the Qlib spelling, the Rust table calls the finkit spelling, and the parity
#: gate compares the two outputs.  An entry is therefore only correct if the two
#: functions compute the same quantity, which is what the gate checks.
RENAME: dict[str, str] = {
    "Ref": "REF",
    "Mean": "MA",
    # pandas' `rolling(N).std()` is `ddof = 1`; finkit's `STD`/`STDDEV` are
    # TA-Lib's population convention.  The ratio is exactly sqrt((n-1)/n) --
    # 0.894 at n = 5 -- so the sample convention needs its own name.
    "Std": "STDDEV_SAMPLE",
    "Sum": "SUM",
    # Qlib's `Max`/`Min` are *rolling* extrema, not element-wise.
    "Max": "HHV",
    "Min": "LLV",
    "Abs": "ABS",
    "Log": "LN",
    # Qlib's `Greater`/`Less` are `np.maximum`/`np.minimum`, i.e. element-wise
    # selection, *not* boolean comparisons.  finkit spells that `MAX`/`MIN`.
    "Greater": "MAX",
    "Less": "MIN",
    "Slope": "LINEARREG_SLOPE",
    "Rsquare": "RSQUARE",
    "Resi": "RESI",
    "Quantile": "QUANTILE",
    "Rank": "RANK_PCT",
    "Corr": "CORREL",
    # `IdxMax`/`IdxMin` count from 1, finkit's `MAXINDEX`/`MININDEX` from 0, so
    # these two get an explicit `+1` (see `translate`).
    "IdxMax": "MAXINDEX",
    "IdxMin": "MININDEX",
}

#: Qlib data field -> finkit input name.
FIELDS: dict[str, str] = {
    "$open": "OPEN",
    "$high": "HIGH",
    "$low": "LOW",
    "$close": "CLOSE",
    "$volume": "VOLUME",
    "$vwap": "VWAP",
}


def _match_paren(text: str, open_index: int) -> int:
    """Index of the `)` matching the `(` at `open_index`."""
    depth = 0
    for index in range(open_index, len(text)):
        if text[index] == "(":
            depth += 1
        elif text[index] == ")":
            depth -= 1
            if depth == 0:
                return index
    raise ValueError(f"unbalanced parentheses in {text!r}")


def translate(expression: str) -> str:
    """Rewrite one Qlib expression into finkit's formula language.

    This is a *textual* transliteration: operator names and `$field` references
    are substituted and every other character -- including Qlib's own spacing,
    which is inconsistent (`Ref($close, 5)` here, `Ref($close,1)` there) -- is
    copied through verbatim.  Preserving the original text means a reader can
    diff the table against `qlib/contrib/data/loader.py` as a pure token
    substitution, and it is why the two `+1`s in `IMXD` are written out rather
    than algebraically cancelled.
    """
    out: list[str] = []
    index = 0
    while index < len(expression):
        char = expression[index]
        if char == "$":
            end = index + 1
            while end < len(expression) and (expression[end].isalnum() or expression[end] == "_"):
                end += 1
            field = expression[index:end]
            if field not in FIELDS:
                raise ValueError(f"unmapped Qlib field {field!r} in {expression!r}")
            out.append(FIELDS[field])
            index = end
        elif char.isdigit():
            # A numeric literal, copied verbatim.  It has to be consumed as a
            # unit so the `e` of `1e-12` is not mistaken for an identifier.
            end = index
            while end < len(expression) and expression[end].isdigit():
                end += 1
            if end < len(expression) and expression[end] == ".":
                end += 1
                while end < len(expression) and expression[end].isdigit():
                    end += 1
            if end < len(expression) and expression[end] in "eE":
                probe = end + 1
                if probe < len(expression) and expression[probe] in "+-":
                    probe += 1
                if probe < len(expression) and expression[probe].isdigit():
                    end = probe
                    while end < len(expression) and expression[end].isdigit():
                        end += 1
            out.append(expression[index:end])
            index = end
        elif char.isalpha() or char == "_":
            end = index
            while end < len(expression) and (expression[end].isalnum() or expression[end] == "_"):
                end += 1
            word = expression[index:end]
            cursor = end
            while cursor < len(expression) and expression[cursor] == " ":
                cursor += 1
            if cursor >= len(expression) or expression[cursor] != "(":
                raise ValueError(f"bare identifier {word!r} in {expression!r}")
            close = _match_paren(expression, cursor)
            inner = translate(expression[cursor + 1 : close])
            if word in ("IdxMax", "IdxMin"):
                out.append(f"({RENAME[word]}({inner})+1)")
            elif word in RENAME:
                out.append(f"{RENAME[word]}({inner})")
            else:
                raise ValueError(f"unmapped Qlib operator {word!r} in {expression!r}")
            index = close + 1
        else:
            out.append(char)
            index += 1
    return "".join(out)


def finkit_expressions() -> list[tuple[str, str]]:
    """The 158 `(name, finkit expression)` pairs, in Qlib's order."""
    return [(name, translate(expression)) for name, expression in alpha158_expressions()]


def render_rust_table() -> str:
    """The `EXPRESSIONS` const block for `core/src/factors/builtin/alpha158.rs`."""
    lines = [
        "/// The 158 `(name, finkit expression)` pairs, in Qlib's order.",
        "pub const EXPRESSIONS: &[(&str, &str)] = &[",
    ]
    for name, expression in finkit_expressions():
        lines.append(f'    ("{name}", "{expression}"),')
    lines.append("];")
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# The evaluation data.
# ---------------------------------------------------------------------------

#: Number of bars.  Long enough that window 60 has a large post-warm-up region.
BARS = 260

#: Half-open span of the deliberately flat run, `[start, end)`.
#:
#: The run is longer than the longest window (60) so that a *full* window can sit
#: entirely inside it and the rolling standard deviation can reach exactly 0 at
#: every window size.  That is what makes Qlib's near-degenerate guard in
#: `Rsquare` and `Corr` fire rather than remain dead code.
#:
#: It is exported into the contract because it is also the region where finkit's
#: and Qlib's degenerate-window policies can disagree (see `PROBE_NOTE`), and the
#: parity gate needs the span to make that a *structural* claim -- "every bar
#: where the two disagree about finiteness lies inside the probe region" -- rather
#: than a pinned count that nobody can interpret.
PROBE_START = 110
PROBE_END = 175

#: Why the probe region is where it is, and what a divergence there means.
PROBE_NOTE = (
    "The flat run is a probe, not decoration. Qlib NaNs any `Corr`/`Rsquare` window whose "
    "rolling std is within atol=2e-05 of zero -- an absolute, scale-dependent data-hygiene "
    "threshold. finkit NaNs only a window that is *exactly* degenerate (population variance "
    "< 1e-15), because a 2e-05 threshold is meaningless for a small-scale series such as "
    "returns. Both guard the same intent at different widths, so on the probe's shoulders -- "
    "windows straddling the flat run -- finkit can be finite where Qlib is NaN. The parity "
    "gate asserts the divergence is confined to `[PROBE_START - 59, PROBE_END)`, i.e. to "
    "windows that touch the flat run, and that agreement is exact everywhere else."
)


def synthetic_market() -> dict[str, pd.Series]:
    """Deterministic OHLCV, so the contract is reproducible without an RNG.

    Three properties are deliberate rather than decorative:

    * a **flat run** in the middle, so Qlib's near-zero-std guards in `Rsquare`
      and `Corr` actually fire — otherwise that branch of the transcription
      would be dead code that no gate exercises;
    * a **gap** between one bar's close and the next bar's open, so the kbar
      factors are not identically zero; and
    * a strictly positive volume with a real day-over-day change, so `VSUMP`,
      `VSUMD` and `VWMA` are not degenerate.

    The pseudo-random component is a hand-rolled LCG instead of `numpy.random`,
    because a generator algorithm is not part of numpy's compatibility promise
    and the committed contract must not move when numpy is upgraded.
    """
    state = 0x2545F491

    def next_unit() -> float:
        nonlocal state
        state = (state * 1103515245 + 12345) & 0x7FFFFFFF
        return state / 0x7FFFFFFF

    closes: list[float] = []
    opens: list[float] = []
    highs: list[float] = []
    lows: list[float] = []
    volumes: list[float] = []

    close = 100.0
    for bar in range(BARS):
        # A flat stretch covering more than the longest window (60), so that a
        # full window can land entirely inside it and the rolling std can reach
        # exactly 0 at *every* window size -- which is what makes Qlib's
        # near-degenerate guard fire for `Rsquare` and `Corr` at 60 as well.
        if PROBE_START <= bar < PROBE_END:
            step = 0.0
        else:
            step = (next_unit() - 0.48) * 2.4
        open_ = close * (1.0 + (next_unit() - 0.5) * 0.004)
        close = close + step
        high = max(open_, close) * (1.0 + next_unit() * 0.006)
        low = min(open_, close) * (1.0 - next_unit() * 0.006)
        volume = 1.0e6 * (1.0 + 0.35 * math.sin(bar * 0.21)) * (0.8 + 0.4 * next_unit())
        opens.append(open_)
        closes.append(close)
        highs.append(high)
        lows.append(low)
        volumes.append(volume)

    index = pd.RangeIndex(BARS)
    series = {
        "open": pd.Series(opens, index=index, dtype=np.float64),
        "high": pd.Series(highs, index=index, dtype=np.float64),
        "low": pd.Series(lows, index=index, dtype=np.float64),
        "close": pd.Series(closes, index=index, dtype=np.float64),
        "volume": pd.Series(volumes, index=index, dtype=np.float64),
    }
    # Qlib's `$vwap` is a data-layer field, not an operator, so it is supplied
    # here the same way a vendor would: a volume-weighted typical price.
    typical = (series["high"] + series["low"] + series["close"]) / 3.0
    series["vwap"] = (typical * series["volume"]).cumsum() / series["volume"].cumsum()
    return series


# ---------------------------------------------------------------------------
# Contract emission.
# ---------------------------------------------------------------------------


def build_reference() -> dict:
    fields = synthetic_market()
    factors: dict[str, list[float | None]] = {}
    expressions: dict[str, str] = {}
    for name, expression in alpha158_expressions():
        with np.errstate(divide="ignore", invalid="ignore"):
            values = evaluate(expression, fields).to_numpy(dtype=np.float64)
        factors[name] = [None if math.isnan(v) else v for v in values]
        expressions[name] = expression

    return {
        "schema": "finkit.alpha158.reference/v1",
        "reference": {
            "producer": "microsoft/qlib",
            "revision": QLIB_REVISION,
            "transcribed_from": [
                "qlib/contrib/data/loader.py::Alpha158DL.get_feature_config",
                "qlib/data/ops.py::Rolling",
                "qlib/data/ops.py::Ref",
                "qlib/data/ops.py::Quantile",
                "qlib/data/ops.py::Rank",
                "qlib/data/ops.py::Corr",
                "qlib/data/_libs/rolling.pyx::Slope",
                "qlib/data/_libs/rolling.pyx::Resi",
                "qlib/data/_libs/rolling.pyx::Rsquare",
            ],
            "substrate": f"pandas {pd.__version__} / numpy {np.__version__}",
            "note": (
                "Qlib's rolling operators are `series.rolling(N, min_periods=1)`, so the "
                "first N-1 bars of every rolling factor carry a partial-window value. "
                "finkit reports NaN there by design; the parity gate compares on the "
                "common support and separately asserts the warm-up shape."
            ),
            "degenerate_std_atol": DEGENERATE_STD_ATOL,
        },
        "windows": list(WINDOWS),
        "bars": BARS,
        "probe": {
            "start": PROBE_START,
            "end": PROBE_END,
            "widest_window": max(WINDOWS),
            "note": PROBE_NOTE,
        },
        "market": {name: [float(v) for v in s.to_numpy()] for name, s in fields.items()},
        "expressions": expressions,
        "factors": factors,
    }


def _serialise(payload: dict) -> str:
    return json.dumps(payload, indent=2, sort_keys=False, ensure_ascii=False) + "\n"


def _self_test() -> int:
    """Exercise the evaluator and the operator transcriptions on hand-checked cases."""
    failures: list[str] = []

    def check(label: str, actual, expected, tolerance: float = 1e-12) -> None:
        # NaN and None are the same thing to a caller: "no value at this bar".
        def normalise(value):
            if isinstance(value, float) and math.isnan(value):
                return None
            return value

        if isinstance(expected, (list, tuple)):
            actual_list = [normalise(v) for v in actual]
            ok = len(actual_list) == len(expected) and all(
                (a is None and e is None)
                or (a is not None and e is not None and abs(a - e) <= tolerance)
                for a, e in zip(actual_list, expected)
            )
        else:
            ok = abs(actual - expected) <= tolerance
        if not ok:
            failures.append(f"{label}: got {actual!r}, want {expected!r}")

    index = pd.RangeIndex(5)
    s = pd.Series([1.0, 2.0, 3.0, 4.0, 5.0], index=index)
    fields = {"x": s}

    check("arithmetic", list(evaluate("($x*2+1)/$x", fields)), [3.0, 2.5, 7 / 3, 2.25, 2.2])
    check("Ref shift", list(evaluate("Ref($x, 1)", fields)), [None, 1.0, 2.0, 3.0, 4.0])
    check("Mean min_periods=1", list(evaluate("Mean($x, 3)", fields)), [1.0, 1.5, 2.0, 3.0, 4.0])
    check("Std ddof=1", list(evaluate("Std($x, 3)", fields))[:3], [None, 0.7071067811865476, 1.0])
    check("Sum", list(evaluate("Sum($x, 3)", fields)), [1.0, 3.0, 6.0, 9.0, 12.0])
    check("Max/Min", list(evaluate("Max($x, 2)", fields)), [1.0, 2.0, 3.0, 4.0, 5.0])
    check("Quantile linear", list(evaluate("Quantile($x, 4, 0.8)", fields))[3:], [3.4, 4.4])
    check("Rank pct", list(evaluate("Rank($x, 3)", fields))[2:], [1.0, 1.0, 1.0])
    check("IdxMax is 1-based", list(evaluate("IdxMax($x, 3)", fields)), [1.0, 2.0, 3.0, 3.0, 3.0])
    check("IdxMin is 1-based", list(evaluate("IdxMin($x, 3)", fields)), [1.0, 1.0, 1.0, 1.0, 1.0])
    check("Slope of a line", list(evaluate("Slope($x, 3)", fields))[2:], [1.0, 1.0, 1.0])
    check("Rsquare of a line", list(evaluate("Rsquare($x, 3)", fields))[2:], [1.0, 1.0, 1.0])
    check("Resi of a line", list(evaluate("Resi($x, 3)", fields))[2:], [0.0, 0.0, 0.0])
    check("Greater is maximum", list(evaluate("Greater($x, 3)", fields)), [3.0, 3.0, 3.0, 4.0, 5.0])
    check("Less is minimum", list(evaluate("Less($x, 3)", fields)), [1.0, 2.0, 3.0, 3.0, 3.0])
    check("comparison is 0/1", list(evaluate("$x>3", fields)), [0.0, 0.0, 0.0, 1.0, 1.0])
    check("Log is natural", list(evaluate("Log($x)", fields))[1], math.log(2.0))
    check("Corr of a line", list(evaluate("Corr($x, $x, 3)", fields))[2:], [1.0, 1.0, 1.0])
    # 1e-12 must lex as one token, not as `1e - 12`.
    check("exponent literal", list(evaluate("1e-12", fields)), [1e-12] * 5)

    # The 158 expression list must be exactly the size Qlib documents.
    pairs = alpha158_expressions()
    names = [name for name, _ in pairs]
    if len(pairs) != 158:
        failures.append(f"factor count: got {len(pairs)}, want 158")
    if len(set(names)) != len(names):
        duplicates = sorted({n for n in names if names.count(n) > 1})
        failures.append(f"duplicate factor names: {duplicates}")
    for window in WINDOWS:
        if f"RSQR{window}" not in names or f"VSUMD{window}" not in names:
            failures.append(f"window {window} is missing operators")

    # Every expression must actually evaluate, so a typo in a template cannot
    # hide behind the count check above.
    fields = synthetic_market()
    for name, expression in pairs:
        try:
            with np.errstate(divide="ignore", invalid="ignore"):
                values = evaluate(expression, fields)
        except Exception as exc:  # noqa: BLE001 - the point is to report, not to handle
            failures.append(f"{name} failed to evaluate ({expression}): {exc}")
            continue
        if len(values) != BARS:
            failures.append(f"{name} produced {len(values)} bars, want {BARS}")
        if not any(pd.notna(v) for v in values):
            failures.append(f"{name} is all-NaN ({expression})")

    # The transliteration is a separate code path from the evaluator, and it is
    # the one that ships: `core/src/factors/builtin/alpha158.rs` is generated from
    # it.  Two properties are checked, because a translator can fail either by
    # refusing a valid operator or by silently emitting something that parses but
    # means something else.
    translated = finkit_expressions()
    if len(translated) != len(pairs):
        failures.append(f"translation produced {len(translated)} rows, want {len(pairs)}")
    for (name, qlib_expression), (translated_name, finkit_expression) in zip(pairs, translated):
        if name != translated_name:
            failures.append(f"translation reordered {name} -> {translated_name}")
        # Nothing may survive translation as a Qlib-only token: a `$field` or a
        # Qlib operator name left in the output would not parse in finkit.
        for leftover in list(FIELDS) + [f"{op}(" for op in RENAME if op not in ("IdxMax", "IdxMin")]:
            if leftover in finkit_expression:
                failures.append(f"{name} kept the Qlib token {leftover!r}: {finkit_expression}")
        # ... and nothing may be dropped: every finkit call in the output must be
        # a name the rename map can produce, so a typo in a template cannot reach
        # the Rust table as an unknown function.
        allowed = set(RENAME.values())
        for call in re.findall(r"([A-Z_][A-Z0-9_]*)\(", finkit_expression):
            if call not in allowed:
                failures.append(f"{name} emits the unknown function {call!r}: {finkit_expression}")
    # A couple of hand-checked transliterations, so the translator is not only
    # checked against itself.
    hand_checked = {
        "KMID2": "(CLOSE-OPEN)/(HIGH-LOW+1e-12)",
        "IMAX5": "(MAXINDEX(HIGH, 5)+1)/5",
        "IMXD5": "((MAXINDEX(HIGH, 5)+1)-(MININDEX(LOW, 5)+1))/5",
        "STD5": "STDDEV_SAMPLE(CLOSE, 5)/CLOSE",
        "CORR5": "CORREL(CLOSE, LN(VOLUME+1), 5)",
        "WVMA5": "STDDEV_SAMPLE(ABS(CLOSE/REF(CLOSE,1)-1)*VOLUME, 5)/"
        "(MA(ABS(CLOSE/REF(CLOSE,1)-1)*VOLUME, 5)+1e-12)",
    }
    by_name = dict(translated)
    for name, expected in hand_checked.items():
        if by_name.get(name) != expected:
            failures.append(f"translation of {name}: got {by_name.get(name)!r}, want {expected!r}")

    # The probe must actually be flat: the guard assertion below, and the parity
    # gate's structural claim, both depend on it.
    probe_close = fields["close"].to_numpy()[PROBE_START:PROBE_END]
    if float(np.ptp(probe_close)) != 0.0:
        failures.append(
            f"the probe region [{PROBE_START}, {PROBE_END}) is not exactly flat "
            f"(ptp={float(np.ptp(probe_close))}); Qlib's degenerate guard and the "
            "parity gate's divergence claim both depend on it"
        )

    # The near-degenerate-std guard is only worth transcribing if the reference
    # data actually trips it.  Without this assertion the flat run in
    # `synthetic_market` could silently stop being flat and the guard branch
    # would become untested code.
    for window in WINDOWS:
        std = _rolling(fields["close"], window).std()
        if not bool(np.isclose(std, 0, atol=DEGENERATE_STD_ATOL).any()):
            failures.append(
                f"the flat run does not trip the std<{DEGENERATE_STD_ATOL} guard at window {window}; "
                "Rsquare/Corr's degenerate branch would go untested"
            )

    if failures:
        for failure in failures:
            print(f"FAIL {failure}", file=sys.stderr)
        return 1
    print(f"self-test passed: {len(pairs)} factors evaluate, operator semantics verified")
    return 0


def _committed_table() -> str:
    """The `EXPRESSIONS` block as it currently sits in the Rust source."""
    text = TABLE.read_text(encoding="utf-8")
    start = text.index("pub const EXPRESSIONS: &[(&str, &str)] = &[")
    end = text.index("\n];", start) + len("\n];")
    # The doc comment directly above the const is part of what is checked.
    head = text.rindex("\n\n", 0, start) + 2
    return text[head:end]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the committed contract is stale")
    parser.add_argument("--self-test", action="store_true", help="verify the evaluator and operator set")
    parser.add_argument(
        "--emit-table",
        action="store_true",
        help="print the Rust EXPRESSIONS block for core/src/factors/builtin/alpha158.rs",
    )
    parser.add_argument(
        "--check-table",
        action="store_true",
        help="fail if the committed Rust table is not the transliteration of the reference",
    )
    args = parser.parse_args()

    if args.emit_table:
        print(render_rust_table())
        return 0

    if args.check_table:
        expected = render_rust_table()
        actual = _committed_table()
        if actual != expected:
            expected_lines = expected.splitlines()
            actual_lines = actual.splitlines()
            diff = [
                f"  line {number}:\n    committed: {left!r}\n    expected:  {right!r}"
                for number, (left, right) in enumerate(zip(actual_lines, expected_lines), 1)
                if left != right
            ]
            print(
                f"{TABLE} has drifted from the transliteration of "
                f"{OUTPUT.name}; regenerate with `--emit-table`.\n"
                + "\n".join(diff[:20]),
                file=sys.stderr,
            )
            return 1
        print(f"{TABLE.name} EXPRESSIONS matches the transliteration ({len(finkit_expressions())} factors)")
        return 0

    if args.self_test:
        return _self_test()

    payload = build_reference()
    text = _serialise(payload)

    if args.check:
        if not OUTPUT.exists():
            print(f"missing contract {OUTPUT}", file=sys.stderr)
            return 1
        if OUTPUT.read_text(encoding="utf-8") != text:
            print(
                f"{OUTPUT} is stale; run `python scripts/gen_alpha158_reference.py`",
                file=sys.stderr,
            )
            return 1
        print(f"{OUTPUT.name} is up to date ({len(payload['factors'])} factors)")
        return 0

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(text, encoding="utf-8")
    finite = sum(
        1 for series in payload["factors"].values() for value in series if value is not None
    )
    print(f"wrote {OUTPUT} — {len(payload['factors'])} factors, {finite} finite values")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
