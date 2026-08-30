"""Tests for FFI error-code → Python semantic exception mapping.

Target exceptions (see docs/api-reference.md):
  - InsufficientDataError  — input too short for the requested operation
  - InvalidParameterError — period or other argument out of valid range

Tests skip when alpha_ta is not installed or semantic exceptions are not yet
registered on the module.
"""

from __future__ import annotations

import numpy as np
import pytest


def _import_alpha_ta():
    try:
        import alpha_ta as ta
        return ta
    except ImportError:
        pytest.skip("alpha_ta not installed — build with: maturin develop")


def _get_exception_classes():
    """Resolve semantic exception types from alpha_ta (module or exceptions sub-module)."""
    ta = _import_alpha_ta()

    insufficient = getattr(ta, "InsufficientDataError", None)
    invalid_param = getattr(ta, "InvalidParameterError", None)

    if insufficient is None or invalid_param is None:
        try:
            from alpha_ta.exceptions import InsufficientDataError, InvalidParameterError
            insufficient = insufficient or InsufficientDataError
            invalid_param = invalid_param or InvalidParameterError
        except ImportError:
            pass

    if insufficient is None or invalid_param is None:
        pytest.skip("InsufficientDataError / InvalidParameterError not implemented yet")

    return ta, insufficient, invalid_param


# ---------------------------------------------------------------------------
# InsufficientDataError
# ---------------------------------------------------------------------------

def test_insufficient_data_empty_input():
    """Empty close array should raise InsufficientDataError."""
    ta, InsufficientDataError, _ = _get_exception_classes()

    empty = np.array([], dtype=np.float64)

    with pytest.raises(InsufficientDataError) as exc_info:
        ta.sma(empty, timeperiod=14)

    msg = str(exc_info.value).lower()
    assert "insufficient" in msg or "empty" in msg or "data" in msg


def test_insufficient_data_too_short():
    """Input shorter than period should raise InsufficientDataError."""
    ta, InsufficientDataError, _ = _get_exception_classes()

    short = np.array([1.0, 2.0, 3.0], dtype=np.float64)
    period = 14

    with pytest.raises(InsufficientDataError) as exc_info:
        ta.sma(short, timeperiod=period)

    msg = str(exc_info.value).lower()
    assert "insufficient" in msg or "data" in msg or "short" in msg


def test_insufficient_data_rsi():
    """RSI with too few bars should raise InsufficientDataError."""
    ta, InsufficientDataError, _ = _get_exception_classes()

    short = np.array([44.0, 44.5, 45.0], dtype=np.float64)

    with pytest.raises(InsufficientDataError):
        ta.rsi(short, timeperiod=14)


# ---------------------------------------------------------------------------
# InvalidParameterError
# ---------------------------------------------------------------------------

def test_invalid_parameter_zero_period():
    """Period == 0 should raise InvalidParameterError."""
    ta, _, InvalidParameterError = _get_exception_classes()

    close = np.arange(1, 21, dtype=np.float64)

    with pytest.raises(InvalidParameterError) as exc_info:
        ta.sma(close, timeperiod=0)

    msg = str(exc_info.value).lower()
    assert "parameter" in msg or "period" in msg or "invalid" in msg


def test_invalid_parameter_negative_period():
    """Negative period is invalid (usize in Rust — may raise TypeError or InvalidParameterError)."""
    ta, _, InvalidParameterError = _get_exception_classes()

    close = np.arange(1, 21, dtype=np.float64)

    with pytest.raises((InvalidParameterError, TypeError, ValueError)):
        ta.sma(close, timeperiod=-1)


def test_invalid_parameter_macd_periods():
    """MACD with zero fast/slow period should raise InvalidParameterError."""
    ta, _, InvalidParameterError = _get_exception_classes()

    close = np.arange(1, 51, dtype=np.float64)

    with pytest.raises(InvalidParameterError):
        ta.macd(close, fastperiod=0, slowperiod=26, signalperiod=9)

    with pytest.raises(InvalidParameterError):
        ta.macd(close, fastperiod=12, slowperiod=0, signalperiod=9)


# ---------------------------------------------------------------------------
# Exception hierarchy
# ---------------------------------------------------------------------------

def test_exception_inheritance():
    """Semantic errors should inherit from a common AlphaTAError base."""
    ta, InsufficientDataError, InvalidParameterError = _get_exception_classes()

    base = getattr(ta, "AlphaTAError", None) or getattr(ta, "TaLibError", None)

    if base is None:
        try:
            from alpha_ta.exceptions import AlphaTAError as base
        except ImportError:
            pytest.skip("AlphaTAError base class not defined")

    assert issubclass(InsufficientDataError, Exception)
    assert issubclass(InvalidParameterError, Exception)
    if base is not None:
        assert issubclass(InsufficientDataError, base)
        assert issubclass(InvalidParameterError, base)
