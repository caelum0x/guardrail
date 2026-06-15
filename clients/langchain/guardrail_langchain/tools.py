"""LangChain tool wrappers for the Guardrail Alpha read-only API.

Each tool wraps one method of the stdlib-only Python SDK
(:class:`guardrail_client.GuardrailClient`) and returns a JSON string -- the
shape LangChain agents expect from a tool. The SDK is dependency-free, so this
module only needs ``langchain_core`` when you want real LangChain tools.

The public entry point is :func:`build_tools`. It works in two modes:

* If ``langchain_core`` is importable, every tool is wrapped as a
  ``langchain_core.tools.StructuredTool`` ready to register with an agent.
* If ``langchain_core`` is *not* installed, each tool is returned as a plain
  :class:`ToolSpec` dataclass exposing ``.name``, ``.description`` and
  ``.func``. This keeps the module importable, usable and testable without any
  third-party dependency.

The ``langchain_core`` import is guarded in a ``try``/``except`` so importing
this module never fails because of a missing optional dependency.
"""

from __future__ import annotations

import json
import os
import sys
from dataclasses import dataclass
from typing import Any, Callable, List, Optional

# --- Locate the stdlib-only Python SDK (clients/python/guardrail_client) ------
# This file lives at clients/langchain/guardrail_langchain/tools.py, so the
# sibling Python SDK is two directories up, under ../../python.
_SDK_PATH = os.path.abspath(
    os.path.join(os.path.dirname(__file__), "..", "..", "python")
)
if _SDK_PATH not in sys.path:
    sys.path.insert(0, _SDK_PATH)

# Repo root anchors the file-backed surfaces (the ensemble config has no API
# route). This file lives at clients/langchain/guardrail_langchain/tools.py, so
# the repo root is three directories up. Override with GUARDRAIL_REPO_ROOT.
_REPO_ROOT = os.environ.get(
    "GUARDRAIL_REPO_ROOT",
    os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "..")),
)
_ENSEMBLE_FILE = os.path.join(_REPO_ROOT, "skills", "ensemble.json")
_SKILLS_INDEX_FILE = os.path.join(_REPO_ROOT, "skills", "INDEX.json")

from guardrail_client import (  # noqa: E402  (import after sys.path tweak)
    DEFAULT_BASE_URL,
    GuardrailApiError,
    GuardrailClient,
)

__all__ = [
    "ToolSpec",
    "build_tools",
    "langchain_available",
    "DEFAULT_BASE_URL",
]


# --- Optional langchain_core import (guarded) ---------------------------------
try:  # pragma: no cover - exercised only when langchain is installed
    from langchain_core.tools import StructuredTool

    _LANGCHAIN_AVAILABLE = True
except Exception:  # noqa: BLE001 - any import failure means "not available"
    StructuredTool = None  # type: ignore[assignment]
    _LANGCHAIN_AVAILABLE = False


def langchain_available() -> bool:
    """Return ``True`` when ``langchain_core`` could be imported."""
    return _LANGCHAIN_AVAILABLE


@dataclass(frozen=True)
class ToolSpec:
    """Framework-agnostic description of a single Guardrail tool.

    Mirrors the attributes a LangChain tool exposes (``name``,
    ``description``, ``func``) so callers can use the same interface whether or
    not LangChain is installed.
    """

    name: str
    description: str
    func: Callable[..., str]


def _to_json(value: Any) -> str:
    """Serialize an SDK result to a stable, human-readable JSON string."""
    if isinstance(value, str):
        # Text endpoints (metrics, markdown) already return plain strings.
        return value
    return json.dumps(value, indent=2, sort_keys=True, default=str)


def _wrap(method: Callable[..., Any]) -> Callable[..., str]:
    """Adapt an SDK method into a tool callable returning a JSON string.

    Errors are caught and returned as a JSON error envelope so an agent can
    reason about the failure instead of the whole tool call raising.
    """

    def _call(**kwargs: Any) -> str:
        try:
            result = method(**kwargs)
        except GuardrailApiError as exc:
            return json.dumps(
                {
                    "error": str(exc),
                    "status": getattr(exc, "status", None),
                    "path": getattr(exc, "path", None),
                },
                indent=2,
                sort_keys=True,
            )
        except Exception as exc:  # noqa: BLE001 - surface unexpected errors as data
            return json.dumps(
                {"error": f"{type(exc).__name__}: {exc}"},
                indent=2,
                sort_keys=True,
            )
        return _to_json(result)

    return _call


def _resolve_ensemble_routing(regime_label: str) -> Any:
    """Blend the embedded ensemble weights for a classified regime.

    Reads the committed ``skills/ensemble.json`` meta-allocator config and
    returns the per-skill weights for ``regime_label`` plus the skill labels,
    reserve symbol and rationale. The lookup is case-insensitive and tolerates
    the ``risk-on`` / ``risk_on`` spelling variants. Pure and offline: it does
    not propose an execution book (the Rust risk engine is the sole gate).
    """
    with open(_ENSEMBLE_FILE, "r", encoding="utf-8") as handle:
        config = json.load(handle)

    regimes = config.get("regimes", {})
    skills = config.get("skills", {})
    normalized = regime_label.strip().lower().replace("-", "_")
    matched_key: Optional[str] = None
    for key in regimes:
        if key.lower().replace("-", "_") == normalized:
            matched_key = key
            break

    if matched_key is None:
        return {
            "error": f"Unknown regime: {regime_label!r}",
            "available_regimes": sorted(regimes.keys()),
        }

    entry = regimes[matched_key]
    weights = entry.get("weights", {})
    resolved = [
        {
            "skill": skill_id,
            "label": skills.get(skill_id, {}).get("label", skill_id),
            "weight": weight,
        }
        for skill_id, weight in sorted(
            weights.items(), key=lambda kv: kv[1], reverse=True
        )
    ]
    return {
        "regime": matched_key,
        "ensemble": config.get("name"),
        "reserve_symbol": config.get("reserve_symbol"),
        "max_risk_allocation_pct": config.get("max_risk_allocation_pct"),
        "rationale": entry.get("rationale"),
        "weights": resolved,
    }


def _ensemble_routing_tool(client: GuardrailClient) -> Callable[..., str]:
    """Build the ensemble-routing tool callable for a client.

    With a ``regime`` argument it resolves that regime offline from the embedded
    config. Without one it fetches the live ``/regime`` classification and
    blends against the embedded weights. Errors are returned as a JSON error
    envelope so an agent can reason about the failure.
    """

    def _call(regime: Optional[str] = None) -> str:
        try:
            if regime:
                result = _resolve_ensemble_routing(regime)
                if isinstance(result, dict):
                    result["regime_source"] = "argument"
                return _to_json(result)

            classification = client.regime()
            label = (
                classification.get("regime")
                or classification.get("label")
                or classification.get("classification")
            )
            if not isinstance(label, str) or not label:
                return _to_json(
                    {
                        "error": "Could not determine a regime label from "
                        "/regime.",
                        "regime_response": classification,
                    }
                )
            result = _resolve_ensemble_routing(label)
            if isinstance(result, dict):
                result["regime_source"] = "live"
                result["regime_classification"] = classification
            return _to_json(result)
        except GuardrailApiError as exc:
            return json.dumps(
                {
                    "error": str(exc),
                    "status": getattr(exc, "status", None),
                    "path": getattr(exc, "path", None),
                },
                indent=2,
                sort_keys=True,
            )
        except Exception as exc:  # noqa: BLE001 - surface as data, never raise
            return json.dumps(
                {"error": f"{type(exc).__name__}: {exc}"},
                indent=2,
                sort_keys=True,
            )

    return _call


def _skill_catalog() -> List[dict]:
    """Return the 5-skill catalog as ``[{id, name, thesis}, ...]``.

    Reads the committed ``skills/INDEX.json`` (the full skill catalog) and
    projects each entry to its ``id``, human ``name`` and one-line thesis (the
    ``summary`` field). Pure and offline; file/parse errors propagate to the
    caller, which surfaces them as a JSON error envelope.
    """
    with open(_SKILLS_INDEX_FILE, "r", encoding="utf-8") as handle:
        catalog = json.load(handle)
    if not isinstance(catalog, list):
        raise ValueError("skills/INDEX.json must be a JSON array of skills")
    return [
        {
            "id": entry.get("id"),
            "name": entry.get("name"),
            "thesis": entry.get("summary"),
        }
        for entry in catalog
    ]


def _skill_catalog_tool() -> Callable[..., str]:
    """Build the skill-catalog tool callable.

    Lists the strategy skills (id, name, thesis) from ``skills/INDEX.json``.
    Errors are returned as a JSON error envelope so an agent can reason about
    the failure instead of the call raising.
    """

    def _call() -> str:
        try:
            return _to_json({"skills": _skill_catalog()})
        except Exception as exc:  # noqa: BLE001 - surface as data, never raise
            return json.dumps(
                {"error": f"{type(exc).__name__}: {exc}"},
                indent=2,
                sort_keys=True,
            )

    return _call


def _specs_for_client(client: GuardrailClient) -> List[ToolSpec]:
    """Build the canonical list of :class:`ToolSpec` for a client.

    The set of tools intentionally covers the endpoints highlighted in the
    integration brief plus a couple of closely related read-only views.
    """

    def make(
        name: str,
        method: Callable[..., Any],
        description: str,
    ) -> ToolSpec:
        return ToolSpec(name=name, description=description, func=_wrap(method))

    return [
        make(
            "guardrail_health",
            client.health,
            "Check the Guardrail API and database health/status. No arguments.",
        ),
        make(
            "guardrail_backtest",
            client.backtest,
            "Run a strategy-vs-benchmark backtest. Optional args: "
            "steps (int), fear_greed (int 0-100), preset (str). "
            "Returns performance metrics as JSON.",
        ),
        make(
            "guardrail_walkforward",
            client.walkforward,
            "Run a rolling walk-forward analysis. Optional args: "
            "windows (int), steps (int), preset (str). Returns per-window "
            "results as JSON.",
        ),
        make(
            "guardrail_sweep",
            client.sweep,
            "Run a sentiment comparison sweep across fear/greed values. "
            "Optional args: steps (int), fear_greed (list[int]), preset (str).",
        ),
        make(
            "guardrail_regime",
            client.regime,
            "Get the current market regime classification. No arguments.",
        ),
        make(
            "guardrail_funding",
            client.funding,
            "Get current funding rates for tracked assets. No arguments.",
        ),
        make(
            "guardrail_compete",
            client.compete,
            "Get the competition status and standings. No arguments.",
        ),
        make(
            "guardrail_alerts",
            client.alerts,
            "Get evaluated risk/operational alerts. No arguments.",
        ),
        make(
            "guardrail_proof",
            client.proof,
            "Get the agent identity and cryptographic report proof. "
            "No arguments.",
        ),
        make(
            "guardrail_compile_policy",
            client.compile_policy,
            "Compile a natural-language mandate into a validated policy plus "
            "hash. Required arg: mandate (str).",
        ),
        make(
            "guardrail_indicators",
            client.indicators,
            "Get deterministic synthetic indicators for a symbol. Optional "
            "args: symbol (str), steps (int).",
        ),
        make(
            "guardrail_journal",
            client.events,
            "Get the decision journal: the agent's recent event log with "
            "timestamps and context. No arguments.",
        ),
        make(
            "guardrail_scenarios",
            client.scenarios,
            "Get the stress-scenario catalog and each scenario's expected "
            "risk response. No arguments.",
        ),
        ToolSpec(
            name="guardrail_skill_catalog",
            description=(
                "List the Guardrail strategy skills (id, name, thesis) from "
                "skills/INDEX.json. The catalog has 5 skills: the four "
                "regime-complementary skills that form the ensemble core plus "
                "volatility-targeted-risk-parity, an additional standalone "
                "sizing strategy. No arguments."
            ),
            func=_skill_catalog_tool(),
        ),
        ToolSpec(
            name="guardrail_ensemble_routing",
            description=(
                "Resolve the regime-routed ensemble: blend the embedded "
                "per-skill weights for a market regime. The ensemble core is "
                "the 4 regime-complementary skills; volatility-targeted-risk-"
                "parity is an additional standalone sizing strategy and is not "
                "part of the blended weights. Optional arg: regime (str, e.g. "
                "'risk_on'/'risk_off'/'chop'/'breakout'). When omitted, the "
                "live /regime classification is used."
            ),
            func=_ensemble_routing_tool(client),
        ),
    ]


def _to_structured_tool(spec: ToolSpec) -> Any:
    """Convert a :class:`ToolSpec` into a ``langchain_core`` StructuredTool."""
    # StructuredTool.from_function infers the args schema from the wrapped
    # callable's signature. Our wrapped callables accept **kwargs, so we set a
    # permissive schema by relying on LangChain's default inference.
    return StructuredTool.from_function(
        func=spec.func,
        name=spec.name,
        description=spec.description,
    )


def build_tools(base_url: Optional[str] = None) -> List[Any]:
    """Build the Guardrail tool set for a LangChain agent.

    Args:
        base_url: Base URL of the Guardrail API. Defaults to the SDK's
            ``DEFAULT_BASE_URL`` (``http://localhost:8080``) when ``None``.

    Returns:
        A list of tools. When ``langchain_core`` is importable each item is a
        ``StructuredTool``; otherwise each item is a :class:`ToolSpec` exposing
        ``.name``, ``.description`` and ``.func``. Both forms expose ``.name``
        and ``.description``, so simple inspection works either way.
    """
    client = GuardrailClient(base_url=base_url or DEFAULT_BASE_URL)
    specs = _specs_for_client(client)

    if _LANGCHAIN_AVAILABLE:
        return [_to_structured_tool(spec) for spec in specs]
    return list(specs)
