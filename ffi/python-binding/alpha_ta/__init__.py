"""AlphaTA — High-performance financial technical analysis library.

This package re-exports the native Rust extension module and provides
pure-Python convenience wrappers such as the pandas DataFrame accessor.
"""

from __future__ import annotations

try:
    from alpha_ta.alpha_ta import *  # noqa: F401,F403 — re-export native module
    from alpha_ta.alpha_ta import __all__ as _native_all  # noqa: F401
except ImportError:
    pass

from alpha_ta.exceptions import (  # noqa: F401
    AlphaTAError,
    InsufficientDataError,
    InvalidParameterError,
    IndicatorNotFoundError,
)

from alpha_ta.accessor import TaAccessor  # noqa: F401


def register_accessor():
    """Explicitly register the df.ta accessor (idempotent)."""
    TaAccessor._register()
