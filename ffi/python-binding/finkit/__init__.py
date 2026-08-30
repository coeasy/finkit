"""Finkit — high-performance financial indicator, factor, and formula engine.

The public Python package is backed by the Rust native extension.  The
``alpha_ta`` native module name is retained internally in v0.1.0 to preserve
compatibility while the project transitions from the historical AlphaTA
implementation to the Finkit public API.
"""

from __future__ import annotations

from .alpha_ta import *  # noqa: F401,F403

try:
    from .alpha_ta import __all__ as _native_all
except ImportError:  # pragma: no cover - only relevant for incomplete local builds
    _native_all = []

__version__ = "0.1.0"
__all__ = [*_native_all, "__version__"]
