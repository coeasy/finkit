"""Semantic exception hierarchy for finkit.

All finkit-specific errors inherit from ``FinkitError`` so callers can
catch the entire family with a single ``except FinkitError`` clause.

They *also* inherit from the matching built-in, because the two contracts are
both real: `docs/api-reference.md` documents the semantic names, while callers
(and this package's own tests) legitimately write ``except ValueError`` for a
bad period or ``except KeyError`` for an unknown factor. A semantic name that is
not also a built-in forces every caller to know this package's vocabulary.
"""

from __future__ import annotations


class FinkitError(Exception):
    """Base exception for all finkit errors."""


class InsufficientDataError(FinkitError, ValueError):
    """Raised when the input data is too short for the requested operation."""


class InvalidParameterError(FinkitError, ValueError):
    """Raised when an indicator parameter is out of its valid range."""


class IndicatorNotFoundError(FinkitError, KeyError):
    """Raised when a named indicator or formula template does not exist."""
