"""Finkit — high-performance financial indicator, factor, and formula engine.

The public Python package is backed by the Rust native extension. The
``alpha_ta`` native module name is retained internally in v0.1.0 to preserve
compatibility while the project transitions from the historical AlphaTA
implementation to the Finkit public API.
"""

from __future__ import annotations

from .alpha_ta import *  # noqa: F401,F403

__version__ = "0.1.0"

try:
    from .alpha_ta import __all__ as _native_all
except ImportError:
    # PyO3 modules do not need to define __all__. In that case Python's normal
    # star-import behavior keeps every non-private native symbol visible.
    pass
else:
    __all__ = [*_native_all, "__version__"]
