"""Alphalens-compatible factor-research adapter backed by the Rust research engine.

The Python layer owns only pandas/index/schema conversion. Numeric analysis is
performed by ``finkit-factor-analysis`` through ``features.factor_study_json``.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Iterable, Sequence

import numpy as np

from . import finkit as _native


def _require_pandas():
    try:
        import pandas as pd
    except ImportError as exc:  # pragma: no cover - optional dependency boundary
        raise ImportError(
            "finkit.alphalens requires pandas for MultiIndex compatibility; "
            "the Rust research engine itself does not depend on pandas"
        ) from exc
    return pd


def _periods_as_bars(periods: Iterable[int | str]) -> list[int]:
    parsed: list[int] = []
    for period in periods:
        if isinstance(period, (int, np.integer)):
            value = int(period)
        else:
            text = str(period).strip().lower()
            if text.endswith("d"):
                text = text[:-1]
            value = int(text)
        if value <= 0:
            raise ValueError("forward-return periods must be greater than zero")
        parsed.append(value)
    return parsed


def _series_to_panel_arrays(factor, prices, groupby=None):
    pd = _require_pandas()
    if not isinstance(factor.index, pd.MultiIndex) or factor.index.nlevels != 2:
        raise ValueError("factor must use a two-level MultiIndex: (date, asset)")

    factor = factor.rename("factor").sort_index()
    dates = factor.index.get_level_values(0)
    assets = factor.index.get_level_values(1)

    if isinstance(prices, pd.DataFrame):
        price_values = []
        for date, asset in factor.index:
            try:
                price_values.append(float(prices.loc[date, asset]))
            except KeyError:
                price_values.append(float("nan"))
        price_values = np.asarray(price_values, dtype=np.float64)
    else:
        aligned = prices.reindex(factor.index)
        price_values = np.asarray(aligned, dtype=np.float64)

    asset_codes, _ = pd.factorize(assets, sort=True)
    timestamps = np.asarray(pd.DatetimeIndex(dates).view("int64") // 1_000_000_000, dtype=np.int64)
    factor_values = np.asarray(factor, dtype=np.float64)

    group_ids = None
    if groupby is not None:
        if isinstance(groupby, dict):
            groups = [groupby.get(asset, None) for asset in assets]
        else:
            groups = groupby.reindex(factor.index).tolist()
        group_ids, _ = pd.factorize(groups, sort=True)
        group_ids = np.asarray(group_ids, dtype=np.uint32)

    return (
        timestamps,
        np.asarray(asset_codes, dtype=np.uint32),
        factor_values,
        price_values,
        group_ids,
    )


def create_full_tear_sheet(
    factor,
    prices,
    groupby=None,
    quantiles: int = 5,
    periods: Sequence[int | str] = (1, 5, 10),
):
    """Compute a full factor-study report with Alphalens-like inputs.

    Returns a nested ``dict``. Plotting/rendering is intentionally separate from
    computation so the same report is usable by Rust, CLI, JSON and other FFI.
    """

    timestamps, assets, values, aligned_prices, groups = _series_to_panel_arrays(
        factor, prices, groupby
    )
    report_json = _native.features.factor_study_json(
        timestamps,
        assets,
        values,
        aligned_prices,
        _periods_as_bars(periods),
        int(quantiles),
        groups,
    )
    return json.loads(report_json)


@dataclass(frozen=True)
class FactorStudy:
    """Small Python convenience wrapper around the native factor-study engine."""

    factor: object
    prices: object
    groupby: object | None = None
    quantiles: int = 5
    periods: Sequence[int | str] = (1, 5, 10)

    def full_report(self):
        return create_full_tear_sheet(
            self.factor,
            self.prices,
            groupby=self.groupby,
            quantiles=self.quantiles,
            periods=self.periods,
        )


# Names intentionally mirror the most common Alphalens entry point while the
# lower-level performance functions remain canonical in Rust.
create_summary_tear_sheet = create_full_tear_sheet
create_returns_tear_sheet = create_full_tear_sheet
create_information_tear_sheet = create_full_tear_sheet
create_turnover_tear_sheet = create_full_tear_sheet
