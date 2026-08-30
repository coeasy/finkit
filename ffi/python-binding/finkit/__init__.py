"""finkit — High-performance financial technical analysis library.

This package re-exports the native Rust extension module and provides
pure-Python convenience wrappers such as the pandas DataFrame accessor.
"""

from __future__ import annotations

try:
    from finkit.finkit import *  # noqa: F401,F403 — re-export native module
    from finkit.finkit import __all__ as _native_all  # noqa: F401
except ImportError:
    pass

from finkit.exceptions import (  # noqa: F401
    FinkitError,
    InsufficientDataError,
    InvalidParameterError,
    IndicatorNotFoundError,
)

from finkit.accessor import TaAccessor  # noqa: F401


def register_accessor():
    """Explicitly register the df.ta accessor (idempotent)."""
    TaAccessor._register()
