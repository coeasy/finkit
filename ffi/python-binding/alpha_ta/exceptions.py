"""Semantic exception hierarchy for AlphaTA.

All AlphaTA-specific errors inherit from ``AlphaTAError`` so callers can
catch the entire family with a single ``except AlphaTAError`` clause.
"""

from __future__ import annotations


class AlphaTAError(Exception):
    """Base exception for all AlphaTA errors."""


class InsufficientDataError(AlphaTAError):
    """Raised when the input data is too short for the requested operation."""


class InvalidParameterError(AlphaTAError):
    """Raised when an indicator parameter is out of its valid range."""


class IndicatorNotFoundError(AlphaTAError):
    """Raised when a named indicator or formula template does not exist."""
