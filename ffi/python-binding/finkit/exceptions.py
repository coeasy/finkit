"""Semantic exception hierarchy for finkit.

All finkit-specific errors inherit from ``FinkitError`` so callers can
catch the entire family with a single ``except FinkitError`` clause.
"""

from __future__ import annotations


class FinkitError(Exception):
    """Base exception for all finkit errors."""


class InsufficientDataError(FinkitError):
    """Raised when the input data is too short for the requested operation."""


class InvalidParameterError(FinkitError):
    """Raised when an indicator parameter is out of its valid range."""


class IndicatorNotFoundError(FinkitError):
    """Raised when a named indicator or formula template does not exist."""
