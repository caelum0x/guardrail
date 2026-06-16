"""Shared value-formatting helpers for the HTML and text renderers."""

from __future__ import annotations

from decimal import Decimal, ROUND_HALF_UP
from typing import Optional

_DASH = "—"


def fmt_money(value: Optional[Decimal], symbol: str = "$") -> str:
    """Format a Decimal as a money string with thousands separators."""
    if value is None:
        return _DASH
    q = value.quantize(Decimal("0.01"), rounding=ROUND_HALF_UP)
    return f"{symbol}{q:,.2f}"


def fmt_pct(value: Optional[Decimal], places: int = 4) -> str:
    """Format a Decimal percentage value (already multiplied by 100)."""
    if value is None:
        return _DASH
    exp = Decimal(1).scaleb(-places)
    q = value.quantize(exp, rounding=ROUND_HALF_UP)
    return f"{q}%"


def fmt_ratio(value: Optional[Decimal], places: int = 4) -> str:
    """Format a plain Decimal ratio (e.g. a Sharpe ratio)."""
    if value is None:
        return _DASH
    exp = Decimal(1).scaleb(-places)
    q = value.quantize(exp, rounding=ROUND_HALF_UP)
    return f"{q}"


def fmt_int(value: Optional[int]) -> str:
    if value is None:
        return _DASH
    return f"{value:,}"


def fmt_str(value: Optional[str]) -> str:
    if value is None or value == "":
        return _DASH
    return str(value)


def sign_class(value: Optional[Decimal]) -> str:
    """Return 'pos', 'neg' or 'flat' for CSS colouring of a signed value."""
    if value is None:
        return "flat"
    if value > 0:
        return "pos"
    if value < 0:
        return "neg"
    return "flat"
