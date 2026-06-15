"""JSON-RPC 2.0 stdio MCP server wrapping the read-only Guardrail API.

The server reads line-delimited JSON-RPC requests from stdin and writes
line-delimited JSON-RPC responses to stdout. It implements the subset of the
Model Context Protocol needed for tool, resource and prompt discovery and use:

* ``initialize``     -> returns serverInfo + capabilities (tools/resources/prompts)
* ``tools/list``     -> advertises the Guardrail tools and their input schemas
* ``tools/call``     -> dispatches to a :class:`GuardrailClient` method
* ``resources/list`` -> advertises read-only Guardrail artifacts (guardrail://...)
* ``resources/read`` -> fetches the underlying API route and returns its content
* ``prompts/list``   -> advertises reusable prompt templates and their arguments
* ``prompts/get``    -> renders a prompt template into MCP messages

Every advertised tool maps to a single read-only API route. API errors are
returned as MCP tool errors (``isError`` true) rather than crashing the server,
so a transient backend outage never takes the transport down. Resource reads
guard the backend the same way: when the API is down, ``resources/read``
returns a JSON error payload as the resource content instead of raising.

Stdlib only. The Guardrail SDK is imported by inserting ``../python`` onto
``sys.path`` at import time.
"""

from __future__ import annotations

import json
import os
import sys
from typing import Any, Callable, Dict, List, Optional, TextIO

# --- SDK import via sys.path insert --------------------------------------------
# The SDK lives in clients/python/guardrail_client. This file lives in
# clients/mcp/guardrail_mcp, so the SDK package root is two levels up + python.
_THIS_DIR = os.path.dirname(os.path.abspath(__file__))
_SDK_ROOT = os.path.normpath(os.path.join(_THIS_DIR, "..", "..", "python"))
if _SDK_ROOT not in sys.path:
    sys.path.insert(0, _SDK_ROOT)

from guardrail_client import (  # noqa: E402  (import after sys.path mutation)
    DEFAULT_BASE_URL,
    GuardrailApiError,
    GuardrailClient,
)

SERVER_NAME = "guardrail-mcp"
SERVER_VERSION = "0.4.0"
PROTOCOL_VERSION = "2024-11-05"

# --- Repo-root anchored data files --------------------------------------------
# A few resources read committed artifacts directly off disk (they have no API
# route). This file lives at clients/mcp/guardrail_mcp/server.py, so the repo
# root is three directories up. The location can be overridden for tests / hosts
# that relocate the package via ``GUARDRAIL_REPO_ROOT``.
_REPO_ROOT = os.environ.get(
    "GUARDRAIL_REPO_ROOT",
    os.path.normpath(os.path.join(_THIS_DIR, "..", "..", "..")),
)
ENSEMBLE_FILE = os.path.join(_REPO_ROOT, "skills", "ensemble.json")
SCENARIOS_FILE = os.path.join(_REPO_ROOT, "configs", "scenarios", "index.json")
SKILLS_INDEX_FILE = os.path.join(_REPO_ROOT, "skills", "INDEX.json")
CMC_CAPABILITIES_FILE = os.path.join(_REPO_ROOT, "configs", "cmc", "capabilities.json")

# JSON-RPC 2.0 standard error codes.
PARSE_ERROR = -32700
INVALID_REQUEST = -32600
METHOD_NOT_FOUND = -32601
INVALID_PARAMS = -32602
INTERNAL_ERROR = -32603


# --- Tool definitions ----------------------------------------------------------
# Each tool maps a JSON-schema inputSchema to a handler that calls the SDK.
# Handlers receive (client, arguments) and return a JSON-serializable result.

def _schema(
    properties: Optional[Dict[str, Any]] = None,
    required: Optional[List[str]] = None,
) -> Dict[str, Any]:
    """Build a JSON-schema object for a tool's inputSchema."""
    schema: Dict[str, Any] = {
        "type": "object",
        "properties": properties or {},
        "additionalProperties": False,
    }
    if required:
        schema["required"] = required
    return schema


def _opt_int(arguments: Dict[str, Any], key: str) -> Optional[int]:
    """Read an optional integer argument, validating its type."""
    value = arguments.get(key)
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, int):
        raise ValueError(f"'{key}' must be an integer")
    return value


def _opt_str(arguments: Dict[str, Any], key: str) -> Optional[str]:
    """Read an optional string argument, validating its type."""
    value = arguments.get(key)
    if value is None:
        return None
    if not isinstance(value, str):
        raise ValueError(f"'{key}' must be a string")
    return value


def _req_str(arguments: Dict[str, Any], key: str) -> str:
    """Read a required string argument, validating its presence and type."""
    value = arguments.get(key)
    if not isinstance(value, str) or value == "":
        raise ValueError(f"'{key}' is required and must be a non-empty string")
    return value


def _load_json_file(path: str) -> Any:
    """Read and parse a committed JSON artifact off disk.

    Raises ``FileNotFoundError`` if the file is missing and
    ``json.JSONDecodeError`` if it is malformed; both are surfaced to the caller
    as MCP errors rather than crashing the transport.
    """
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def _resolve_ensemble_routing(regime_label: str) -> Dict[str, Any]:
    """Blend the embedded ensemble weights for a classified regime.

    Reads the committed ``skills/ensemble.json`` meta-allocator config and
    returns the per-skill weights for ``regime_label`` together with the skill
    labels, reserve symbol and rationale. The lookup is case-insensitive and
    tolerates the ``risk-on`` / ``risk_on`` spelling variants used across the
    product surfaces. This is a pure, offline resolution: it does not propose an
    execution book (the Rust risk engine remains the sole execution gate).
    """
    config = _load_json_file(ENSEMBLE_FILE)
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


def _skill_catalog() -> List[Dict[str, Any]]:
    """Return the 5-skill catalog as ``[{id, name, thesis}, ...]``.

    Reads the committed ``skills/INDEX.json`` (the full skill catalog) and
    projects each entry down to its identity (``id``), human name (``name``)
    and one-line thesis (the ``summary`` field). Pure and offline; file or
    parse errors propagate to the caller, which surfaces them as JSON.
    """
    catalog = _load_json_file(SKILLS_INDEX_FILE)
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


def _h_skill_catalog(_client: GuardrailClient, _a: Dict[str, Any]) -> Any:
    """List the strategy skills (id, name, thesis) from skills/INDEX.json."""
    return {"skills": _skill_catalog()}


def _h_health(client: GuardrailClient, _a: Dict[str, Any]) -> Any:
    return client.health()


def _h_backtest(client: GuardrailClient, a: Dict[str, Any]) -> Any:
    return client.backtest(
        steps=_opt_int(a, "steps"),
        fear_greed=_opt_int(a, "fear_greed"),
        preset=_opt_str(a, "preset"),
    )


def _h_walkforward(client: GuardrailClient, a: Dict[str, Any]) -> Any:
    return client.walkforward(
        windows=_opt_int(a, "windows"),
        steps=_opt_int(a, "steps"),
        preset=_opt_str(a, "preset"),
    )


def _h_sweep(client: GuardrailClient, a: Dict[str, Any]) -> Any:
    fear_greed = a.get("fear_greed")
    parsed: Optional[List[int]] = None
    if fear_greed is not None:
        if not isinstance(fear_greed, list) or not all(
            isinstance(v, int) and not isinstance(v, bool) for v in fear_greed
        ):
            raise ValueError("'fear_greed' must be an array of integers")
        parsed = list(fear_greed)
    return client.sweep(
        steps=_opt_int(a, "steps"),
        fear_greed=parsed,
        preset=_opt_str(a, "preset"),
    )


def _h_regime(client: GuardrailClient, _a: Dict[str, Any]) -> Any:
    return client.regime()


def _h_funding(client: GuardrailClient, _a: Dict[str, Any]) -> Any:
    return client.funding()


def _h_compete(client: GuardrailClient, _a: Dict[str, Any]) -> Any:
    return client.compete()


def _h_alerts(client: GuardrailClient, _a: Dict[str, Any]) -> Any:
    return client.alerts()


def _h_proof(client: GuardrailClient, _a: Dict[str, Any]) -> Any:
    return client.proof()


def _h_compile_policy(client: GuardrailClient, a: Dict[str, Any]) -> Any:
    return client.compile_policy(mandate=_req_str(a, "mandate"))


def _h_indicators(client: GuardrailClient, a: Dict[str, Any]) -> Any:
    return client.indicators(
        symbol=_opt_str(a, "symbol"),
        steps=_opt_int(a, "steps"),
    )


def _h_skill(client: GuardrailClient, _a: Dict[str, Any]) -> Any:
    return client.skill()


def _h_prizes(client: GuardrailClient, _a: Dict[str, Any]) -> Any:
    return client.prizes()


def _h_readiness(client: GuardrailClient, _a: Dict[str, Any]) -> Any:
    return client.readiness()


def _h_journal(client: GuardrailClient, _a: Dict[str, Any]) -> Any:
    """Decision journal: the agent's recent event log (``/events``)."""
    return client.events()


def _h_scenarios(client: GuardrailClient, _a: Dict[str, Any]) -> Any:
    """Stress scenario catalog and expected responses (``/scenarios``)."""
    return client.scenarios()


def _h_ensemble_routing(client: GuardrailClient, a: Dict[str, Any]) -> Any:
    """Resolve the regime-routed ensemble weights.

    When a ``regime`` argument is supplied it is used directly (offline,
    file-only). Otherwise the live ``/regime`` classification is fetched from
    the API and blended against the embedded ensemble weights. The response
    always includes the regime source so callers know whether the blend used a
    supplied label or the live classification.
    """
    regime_arg = _opt_str(a, "regime")
    if regime_arg:
        result = _resolve_ensemble_routing(regime_arg)
        result["regime_source"] = "argument"
        return result

    classification = client.regime()
    label = (
        classification.get("regime")
        or classification.get("label")
        or classification.get("classification")
    )
    if not isinstance(label, str) or not label:
        return {
            "error": "Could not determine a regime label from /regime.",
            "regime_response": classification,
        }
    result = _resolve_ensemble_routing(label)
    result["regime_source"] = "live"
    result["regime_classification"] = classification
    return result


# Tool registry: name -> (description, inputSchema, handler).
ToolHandler = Callable[[GuardrailClient, Dict[str, Any]], Any]

_PRESET_DESC = "Strategy preset name (e.g. 'balanced', 'aggressive')."


def build_tools() -> Dict[str, Dict[str, Any]]:
    """Return the tool registry mapping tool name to its definition.

    Each value is a dict with ``description``, ``inputSchema`` and ``handler``.
    """
    return {
        "guardrail_health": {
            "description": "API and database health status (read-only).",
            "inputSchema": _schema(),
            "handler": _h_health,
        },
        "guardrail_backtest": {
            "description": "Run a strategy-vs-benchmark backtest (read-only).",
            "inputSchema": _schema(
                {
                    "steps": {
                        "type": "integer",
                        "description": "Number of simulation steps.",
                    },
                    "fear_greed": {
                        "type": "integer",
                        "description": "Fear & greed sentiment index (0-100).",
                    },
                    "preset": {"type": "string", "description": _PRESET_DESC},
                }
            ),
            "handler": _h_backtest,
        },
        "guardrail_walkforward": {
            "description": "Rolling walk-forward window analysis (read-only).",
            "inputSchema": _schema(
                {
                    "windows": {
                        "type": "integer",
                        "description": "Number of walk-forward windows.",
                    },
                    "steps": {
                        "type": "integer",
                        "description": "Steps per window.",
                    },
                    "preset": {"type": "string", "description": _PRESET_DESC},
                }
            ),
            "handler": _h_walkforward,
        },
        "guardrail_sweep": {
            "description": "Sentiment comparison sweep across fear/greed values "
            "(read-only).",
            "inputSchema": _schema(
                {
                    "steps": {
                        "type": "integer",
                        "description": "Number of simulation steps.",
                    },
                    "fear_greed": {
                        "type": "array",
                        "items": {"type": "integer"},
                        "description": "List of fear & greed values to compare.",
                    },
                    "preset": {"type": "string", "description": _PRESET_DESC},
                }
            ),
            "handler": _h_sweep,
        },
        "guardrail_regime": {
            "description": "Current market regime classification (read-only).",
            "inputSchema": _schema(),
            "handler": _h_regime,
        },
        "guardrail_funding": {
            "description": "Funding rates across tracked markets (read-only).",
            "inputSchema": _schema(),
            "handler": _h_funding,
        },
        "guardrail_compete": {
            "description": "Competition status and standings (read-only).",
            "inputSchema": _schema(),
            "handler": _h_compete,
        },
        "guardrail_alerts": {
            "description": "Currently evaluated alerts (read-only).",
            "inputSchema": _schema(),
            "handler": _h_alerts,
        },
        "guardrail_proof": {
            "description": "Agent identity and report proof (read-only).",
            "inputSchema": _schema(),
            "handler": _h_proof,
        },
        "guardrail_compile_policy": {
            "description": "Compile a natural-language mandate into a validated "
            "policy plus hash (read-only).",
            "inputSchema": _schema(
                {
                    "mandate": {
                        "type": "string",
                        "description": "Natural-language mandate to compile.",
                    }
                },
                required=["mandate"],
            ),
            "handler": _h_compile_policy,
        },
        "guardrail_indicators": {
            "description": "Deterministic synthetic indicators for a symbol "
            "(read-only).",
            "inputSchema": _schema(
                {
                    "symbol": {
                        "type": "string",
                        "description": "Asset symbol (e.g. 'BTC').",
                    },
                    "steps": {
                        "type": "integer",
                        "description": "Number of indicator steps.",
                    },
                }
            ),
            "handler": _h_indicators,
        },
        "guardrail_skill": {
            "description": "Agent skill descriptor / capability catalog "
            "(read-only).",
            "inputSchema": _schema(),
            "handler": _h_skill,
        },
        "guardrail_prizes": {
            "description": "Hackathon prize-track catalog and mapping "
            "(read-only).",
            "inputSchema": _schema(),
            "handler": _h_prizes,
        },
        "guardrail_readiness": {
            "description": "Operational readiness probe for the agent "
            "(read-only).",
            "inputSchema": _schema(),
            "handler": _h_readiness,
        },
        "guardrail_journal": {
            "description": "Decision journal: the agent's recent event log "
            "with timestamps and context (read-only).",
            "inputSchema": _schema(),
            "handler": _h_journal,
        },
        "guardrail_scenarios": {
            "description": "Stress-scenario catalog and each scenario's "
            "expected risk response (read-only).",
            "inputSchema": _schema(),
            "handler": _h_scenarios,
        },
        "guardrail_skill_catalog": {
            "description": "List the Guardrail strategy skills (id, name, "
            "thesis) from skills/INDEX.json. The catalog has 5 skills: the four "
            "regime-complementary skills that form the ensemble core plus "
            "volatility-targeted-risk-parity, an additional standalone sizing "
            "strategy (read-only).",
            "inputSchema": _schema(),
            "handler": _h_skill_catalog,
        },
        "guardrail_ensemble_routing": {
            "description": "Resolve the regime-routed ensemble: blend the "
            "embedded per-skill weights for a market regime. The ensemble core "
            "is the 4 regime-complementary skills (volatility-targeted-risk-"
            "parity is an additional standalone sizing strategy and is not part "
            "of the blended weights). Without arguments it fetches the live "
            "/regime classification; pass 'regime' to resolve a specific regime "
            "offline (read-only).",
            "inputSchema": _schema(
                {
                    "regime": {
                        "type": "string",
                        "description": "Optional regime label to resolve "
                        "(e.g. 'risk_on', 'risk_off', 'chop', 'breakout'). "
                        "When omitted, the live /regime classification is used.",
                    }
                }
            ),
            "handler": _h_ensemble_routing,
        },
    }


# --- Resource definitions ------------------------------------------------------
# Resources expose read-only Guardrail artifacts under the ``guardrail://`` URI
# scheme. Each resource maps to one read-only SDK accessor; reading a resource
# fetches that route and returns its content. A reader receives the client and
# returns a JSON-serializable payload.

ResourceReader = Callable[[GuardrailClient], Any]


def build_resources() -> Dict[str, Dict[str, Any]]:
    """Return the resource registry mapping ``guardrail://`` URI to a definition.

    Each value has ``name``, ``description``, ``mimeType`` and ``reader``. The
    reader fetches the underlying API route via the SDK and returns its content.
    """
    return {
        "guardrail://compete": {
            "name": "Competition status",
            "description": "Live competition standings and status (read-only).",
            "mimeType": "application/json",
            "reader": lambda c: c.compete(),
        },
        "guardrail://regime": {
            "name": "Market regime",
            "description": "Current market regime classification (read-only).",
            "mimeType": "application/json",
            "reader": lambda c: c.regime(),
        },
        "guardrail://skill": {
            "name": "Agent skill descriptor",
            "description": "Guardrail agent skill descriptor / capability "
            "catalog (read-only).",
            "mimeType": "application/json",
            "reader": lambda c: c.skill(),
        },
        "guardrail://prizes": {
            "name": "Prize catalog",
            "description": "Hackathon prize-track catalog and mapping "
            "(read-only).",
            "mimeType": "application/json",
            "reader": lambda c: c.prizes(),
        },
        "guardrail://readiness": {
            "name": "Readiness probe",
            "description": "Operational readiness probe for the agent "
            "(read-only).",
            "mimeType": "application/json",
            "reader": lambda c: c.readiness(),
        },
        # File-backed resources: surfaces with no API route are served by
        # reading the committed artifact off disk. The reader ignores the
        # client argument so the dispatch path stays uniform.
        "guardrail://ensemble": {
            "name": "Regime ensemble config",
            "description": "Embedded meta-allocator weights that blend the "
            "Track-2 strategy skills by market regime "
            "(skills/ensemble.json, read-only).",
            "mimeType": "application/json",
            "reader": lambda _c: _load_json_file(ENSEMBLE_FILE),
        },
        "guardrail://scenarios": {
            "name": "Scenario catalog",
            "description": "Committed stress-scenario catalog with each "
            "scenario's expected risk response "
            "(configs/scenarios/index.json, read-only).",
            "mimeType": "application/json",
            "reader": lambda _c: _load_json_file(SCENARIOS_FILE),
        },
        "guardrail://skills": {
            "name": "Skill catalog",
            "description": "Full Track-2 strategy skill catalog: the four "
            "regime-complementary ensemble-core skills plus the standalone "
            "volatility-targeted-risk-parity sizing strategy "
            "(skills/INDEX.json, read-only).",
            "mimeType": "application/json",
            "reader": lambda _c: _load_json_file(SKILLS_INDEX_FILE),
        },
        "guardrail://cmc/capabilities": {
            "name": "CMC Agent Hub capabilities",
            "description": "CMC dataset -> capability lineage: which CoinMarketCap "
            "datasets power which read-only analysis capability, with source files "
            "and API/MCP exposure (configs/cmc/capabilities.json, read-only). The "
            "agent never exposes trade execution to the hub.",
            "mimeType": "application/json",
            "reader": lambda _c: _load_json_file(CMC_CAPABILITIES_FILE),
        },
    }


# --- Prompt definitions --------------------------------------------------------
# Prompts are reusable message templates. Each prompt declares typed arguments
# and a renderer that turns the supplied arguments into MCP messages. Renderers
# are pure (no network) so prompts/get works without a live API.

PromptRenderer = Callable[[Dict[str, Any]], List[Dict[str, Any]]]


def _text_message(role: str, text: str) -> Dict[str, Any]:
    """Build a single MCP prompt message with a text content block."""
    return {"role": role, "content": {"type": "text", "text": text}}


def _prompt_arg(value: Any, default: str = "") -> str:
    """Coerce a prompt argument to a trimmed string with a fallback."""
    if value is None:
        return default
    return str(value).strip() or default


def _render_analyze_regime(args: Dict[str, Any]) -> List[Dict[str, Any]]:
    focus = _prompt_arg(args.get("focus"), "overall positioning")
    horizon = _prompt_arg(args.get("horizon"), "the next trading session")
    return [
        _text_message(
            "user",
            "You are a crypto market strategist. Read the Guardrail market "
            "regime resource (guardrail://regime) and funding data, then "
            f"analyze the current regime with a focus on {focus}. "
            f"Frame your guidance for {horizon}. "
            "Call out: (1) the classified regime and confidence, (2) the key "
            "drivers, (3) concrete risk-management implications, and "
            "(4) one falsifiable signal that would change the call.",
        )
    ]


def _render_explain_decision(args: Dict[str, Any]) -> List[Dict[str, Any]]:
    decision = _prompt_arg(args.get("decision"), "the most recent agent decision")
    audience = _prompt_arg(args.get("audience"), "a non-technical operator")
    return [
        _text_message(
            "user",
            f"Explain {decision} made by the Guardrail agent to {audience}. "
            "Use the Guardrail proof, alerts and regime resources as grounding. "
            "Structure the explanation as: what was decided, why (the evidence "
            "and policy that justified it), what guardrails constrained it, and "
            "what would have produced a different outcome. Be precise and avoid "
            "hindsight bias.",
        )
    ]


def _render_draft_mandate(args: Dict[str, Any]) -> List[Dict[str, Any]]:
    objective = _prompt_arg(args.get("objective"), "steady risk-adjusted growth")
    risk = _prompt_arg(args.get("risk_tolerance"), "moderate")
    constraints = _prompt_arg(args.get("constraints"), "none specified")
    return [
        _text_message(
            "system",
            "You draft natural-language trading mandates that the Guardrail "
            "policy compiler can validate. A mandate is a single, unambiguous "
            "instruction describing objective, risk limits and constraints.",
        ),
        _text_message(
            "user",
            f"Draft a Guardrail mandate. Objective: {objective}. "
            f"Risk tolerance: {risk}. Constraints: {constraints}. "
            "Return one concise mandate sentence, then a short bullet list of "
            "the explicit limits it encodes (max drawdown, position caps, "
            "allowed assets). Keep it compilable by guardrail_compile_policy.",
        ),
    ]


def _render_explain_ensemble_routing(args: Dict[str, Any]) -> List[Dict[str, Any]]:
    regime = _prompt_arg(args.get("regime"), "the current market regime")
    audience = _prompt_arg(args.get("audience"), "a portfolio operator")
    return [
        _text_message(
            "system",
            "You explain how the Guardrail regime-routed ensemble allocates "
            "across its strategy skills. The ensemble CORE is the four "
            "regime-complementary skills (regime-routed alpha, funding-rate "
            "carry, mean-reversion-chop, trend-breakout-momentum); the "
            "volatility-targeted-risk-parity skill is an ADDITIONAL STANDALONE "
            "sizing strategy and is NOT part of the blended ensemble weights. "
            "The ensemble takes the weighted average of each core skill's "
            "target weights for the classified regime, renormalizes to at most "
            "100%, and holds the remainder as a USDT reserve. The full 5-skill "
            "catalog is available via the guardrail_skill_catalog tool and the "
            "guardrail://skills resource. The Rust risk engine remains the sole "
            "execution gate; the ensemble only proposes a blended target book.",
        ),
        _text_message(
            "user",
            f"Explain the ensemble routing for {regime} to {audience}. "
            "Ground the explanation in the guardrail://ensemble resource and "
            "the guardrail_ensemble_routing tool (call it to resolve the live "
            "or specified regime). Cover: (1) which skills lead and why "
            "(cite their weights), (2) the rationale for this regime, (3) how "
            "the USDT reserve emerges from renormalization, and (4) one "
            "regime change that would materially shift the blend.",
        ),
    ]


def _render_summarize_journal(args: Dict[str, Any]) -> List[Dict[str, Any]]:
    window = _prompt_arg(args.get("window"), "the most recent entries")
    focus = _prompt_arg(args.get("focus"), "risk-relevant decisions")
    return [
        _text_message(
            "user",
            "You are an operations analyst reviewing the Guardrail decision "
            f"journal. Use the guardrail_journal tool (the /events log) for "
            f"{window} and summarize {focus}. Produce: (1) a concise timeline "
            "of what happened, (2) the decisions and the triggers behind them, "
            "(3) any risk events, alerts or kill-switch activity, and (4) open "
            "items an operator should follow up on. Quote event timestamps and "
            "avoid speculation beyond the logged data.",
        )
    ]


def build_prompts() -> Dict[str, Dict[str, Any]]:
    """Return the prompt registry mapping prompt name to its definition.

    Each value has ``description``, ``arguments`` (a list of MCP argument
    descriptors) and a ``renderer`` that produces the prompt messages.
    """
    return {
        "analyze-regime": {
            "description": "Analyze the current Guardrail market regime and its "
            "risk implications.",
            "arguments": [
                {
                    "name": "focus",
                    "description": "What to emphasize (e.g. 'BTC', "
                    "'altcoin rotation', 'overall positioning').",
                    "required": False,
                },
                {
                    "name": "horizon",
                    "description": "Time horizon for the analysis "
                    "(e.g. 'the next 24h').",
                    "required": False,
                },
            ],
            "renderer": _render_analyze_regime,
        },
        "explain-decision": {
            "description": "Explain a Guardrail agent decision with grounded "
            "evidence for a given audience.",
            "arguments": [
                {
                    "name": "decision",
                    "description": "The decision to explain (e.g. "
                    "'reducing BTC exposure').",
                    "required": False,
                },
                {
                    "name": "audience",
                    "description": "Who the explanation is for (e.g. "
                    "'a risk committee').",
                    "required": False,
                },
            ],
            "renderer": _render_explain_decision,
        },
        "draft-mandate": {
            "description": "Draft a compilable natural-language Guardrail "
            "mandate from objective and risk inputs.",
            "arguments": [
                {
                    "name": "objective",
                    "description": "The trading objective.",
                    "required": False,
                },
                {
                    "name": "risk_tolerance",
                    "description": "Risk tolerance (e.g. 'low', 'moderate', "
                    "'high').",
                    "required": False,
                },
                {
                    "name": "constraints",
                    "description": "Explicit constraints (assets, caps, "
                    "drawdown limits).",
                    "required": False,
                },
            ],
            "renderer": _render_draft_mandate,
        },
        "explain-ensemble-routing": {
            "description": "Explain how the regime-routed ensemble blends the "
            "strategy skills for a given regime.",
            "arguments": [
                {
                    "name": "regime",
                    "description": "Regime to explain (e.g. 'risk_on', "
                    "'risk_off', 'chop', 'breakout'). Defaults to the current "
                    "regime.",
                    "required": False,
                },
                {
                    "name": "audience",
                    "description": "Who the explanation is for (e.g. "
                    "'a risk committee').",
                    "required": False,
                },
            ],
            "renderer": _render_explain_ensemble_routing,
        },
        "summarize-journal": {
            "description": "Summarize the Guardrail decision journal "
            "(/events) into a timeline with decisions and follow-ups.",
            "arguments": [
                {
                    "name": "window",
                    "description": "Which entries to cover (e.g. 'the last "
                    "hour', 'today').",
                    "required": False,
                },
                {
                    "name": "focus",
                    "description": "What to emphasize (e.g. 'risk events', "
                    "'policy changes').",
                    "required": False,
                },
            ],
            "renderer": _render_summarize_journal,
        },
    }


# --- JSON-RPC response builders ------------------------------------------------

def _result(request_id: Any, result: Any) -> Dict[str, Any]:
    return {"jsonrpc": "2.0", "id": request_id, "result": result}


def _error(request_id: Any, code: int, message: str) -> Dict[str, Any]:
    return {
        "jsonrpc": "2.0",
        "id": request_id,
        "error": {"code": code, "message": message},
    }


def _tool_text(payload: Any, is_error: bool = False) -> Dict[str, Any]:
    """Wrap a payload as an MCP tools/call result with a text content block."""
    if isinstance(payload, str):
        text = payload
    else:
        text = json.dumps(payload, ensure_ascii=False)
    result: Dict[str, Any] = {"content": [{"type": "text", "text": text}]}
    if is_error:
        result["isError"] = True
    return result


# --- Server --------------------------------------------------------------------

class MCPServer:
    """Dispatches JSON-RPC requests to MCP method handlers.

    The server is constructed once and reused for the lifetime of the stdio
    stream. It is intentionally stateless beyond the configured base URL: a new
    :class:`GuardrailClient` is created per tool call so a slow or failing
    request never blocks unrelated calls.
    """

    def __init__(self, base_url: Optional[str] = None) -> None:
        self._base_url = base_url or os.environ.get(
            "GUARDRAIL_BASE_URL", DEFAULT_BASE_URL
        )
        self._tools = build_tools()
        self._resources = build_resources()
        self._prompts = build_prompts()

    @property
    def base_url(self) -> str:
        return self._base_url

    def _client(self) -> GuardrailClient:
        return GuardrailClient(base_url=self._base_url)

    def handle(self, request: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        """Handle a single parsed JSON-RPC request.

        Returns the response dict, or ``None`` for notifications (requests
        without an ``id``), which must not produce a response per JSON-RPC 2.0.
        """
        if not isinstance(request, dict):
            return _error(None, INVALID_REQUEST, "Request must be a JSON object")

        request_id = request.get("id")
        method = request.get("method")
        is_notification = "id" not in request

        if not isinstance(method, str):
            if is_notification:
                return None
            return _error(request_id, INVALID_REQUEST, "Missing 'method'")

        params = request.get("params") or {}
        if not isinstance(params, dict):
            if is_notification:
                return None
            return _error(request_id, INVALID_PARAMS, "'params' must be an object")

        # Notifications (e.g. notifications/initialized) get no response.
        if is_notification:
            return None

        if method == "initialize":
            return _result(request_id, self._initialize())
        if method == "tools/list":
            return _result(request_id, self._tools_list())
        if method == "tools/call":
            return _result(request_id, self._tools_call(params))
        if method == "resources/list":
            return _result(request_id, self._resources_list())
        if method == "resources/read":
            return _result(request_id, self._resources_read(params))
        if method == "prompts/list":
            return _result(request_id, self._prompts_list())
        if method == "prompts/get":
            return self._prompts_get(request_id, params)
        if method == "ping":
            return _result(request_id, {})

        return _error(request_id, METHOD_NOT_FOUND, f"Unknown method: {method}")

    def _initialize(self) -> Dict[str, Any]:
        return {
            "protocolVersion": PROTOCOL_VERSION,
            "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION},
            "capabilities": {
                "tools": {"listChanged": False},
                "resources": {"listChanged": False, "subscribe": False},
                "prompts": {"listChanged": False},
            },
        }

    def _tools_list(self) -> Dict[str, Any]:
        tools = [
            {
                "name": name,
                "description": spec["description"],
                "inputSchema": spec["inputSchema"],
            }
            for name, spec in self._tools.items()
        ]
        return {"tools": tools}

    def _tools_call(self, params: Dict[str, Any]) -> Dict[str, Any]:
        name = params.get("name")
        arguments = params.get("arguments") or {}
        if not isinstance(name, str):
            return _tool_text(
                {"error": "tools/call requires a string 'name'"}, is_error=True
            )
        if not isinstance(arguments, dict):
            return _tool_text(
                {"error": "'arguments' must be an object"}, is_error=True
            )

        spec = self._tools.get(name)
        if spec is None:
            return _tool_text(
                {"error": f"Unknown tool: {name}"}, is_error=True
            )

        handler: ToolHandler = spec["handler"]
        try:
            result = handler(self._client(), arguments)
        except ValueError as exc:
            # Argument validation failure.
            return _tool_text(
                {"error": f"Invalid arguments for {name}: {exc}"}, is_error=True
            )
        except GuardrailApiError as exc:
            return _tool_text(
                {
                    "error": str(exc),
                    "status": exc.status,
                    "path": exc.path,
                },
                is_error=True,
            )
        except Exception as exc:  # noqa: BLE001 - never crash the transport
            return _tool_text(
                {"error": f"Unexpected error calling {name}: {exc}"},
                is_error=True,
            )
        return _tool_text(result)

    # --- Resources -----------------------------------------------------------
    def _resources_list(self) -> Dict[str, Any]:
        resources = [
            {
                "uri": uri,
                "name": spec["name"],
                "description": spec["description"],
                "mimeType": spec["mimeType"],
            }
            for uri, spec in self._resources.items()
        ]
        return {"resources": resources}

    def _resources_read(self, params: Dict[str, Any]) -> Dict[str, Any]:
        uri = params.get("uri")
        if not isinstance(uri, str):
            return self._resource_contents(
                str(uri),
                {"error": "resources/read requires a string 'uri'"},
                is_error=True,
            )

        spec = self._resources.get(uri)
        if spec is None:
            return self._resource_contents(
                uri,
                {"error": f"Unknown resource: {uri}"},
                is_error=True,
            )

        reader: ResourceReader = spec["reader"]
        try:
            payload = reader(self._client())
        except GuardrailApiError as exc:
            # Backend down/unreachable: surface as resource content, not a crash.
            return self._resource_contents(
                uri,
                {"error": str(exc), "status": exc.status, "path": exc.path},
                is_error=True,
            )
        except Exception as exc:  # noqa: BLE001 - never crash the transport
            return self._resource_contents(
                uri,
                {"error": f"Unexpected error reading {uri}: {exc}"},
                is_error=True,
            )
        return self._resource_contents(uri, payload)

    def _resource_contents(
        self, uri: str, payload: Any, is_error: bool = False
    ) -> Dict[str, Any]:
        """Wrap a payload as an MCP resources/read result.

        Errors are encoded as JSON content (with an ``error`` key) so callers
        always receive a well-formed resource even when the API is unavailable.
        """
        spec = self._resources.get(uri)
        mime = spec["mimeType"] if spec else "application/json"
        if isinstance(payload, str) and mime.startswith("text/"):
            text = payload
        else:
            text = json.dumps(payload, ensure_ascii=False)
            mime = "application/json"
        return {
            "contents": [{"uri": uri, "mimeType": mime, "text": text}]
        }

    # --- Prompts -------------------------------------------------------------
    def _prompts_list(self) -> Dict[str, Any]:
        prompts = [
            {
                "name": name,
                "description": spec["description"],
                "arguments": spec["arguments"],
            }
            for name, spec in self._prompts.items()
        ]
        return {"prompts": prompts}

    def _prompts_get(
        self, request_id: Any, params: Dict[str, Any]
    ) -> Dict[str, Any]:
        name = params.get("name")
        arguments = params.get("arguments") or {}
        if not isinstance(name, str):
            return _error(
                request_id, INVALID_PARAMS, "prompts/get requires a string 'name'"
            )
        if not isinstance(arguments, dict):
            return _error(
                request_id, INVALID_PARAMS, "'arguments' must be an object"
            )

        spec = self._prompts.get(name)
        if spec is None:
            return _error(
                request_id, INVALID_PARAMS, f"Unknown prompt: {name}"
            )

        renderer: PromptRenderer = spec["renderer"]
        try:
            messages = renderer(arguments)
        except Exception as exc:  # noqa: BLE001 - never crash the transport
            return _error(
                request_id,
                INTERNAL_ERROR,
                f"Failed to render prompt {name}: {exc}",
            )
        return _result(
            request_id,
            {"description": spec["description"], "messages": messages},
        )


# --- stdio loop ----------------------------------------------------------------

def serve(
    server: Optional[MCPServer] = None,
    *,
    stdin: Optional[TextIO] = None,
    stdout: Optional[TextIO] = None,
) -> None:
    """Run the line-delimited JSON-RPC stdio loop until EOF.

    Each input line is parsed as one JSON-RPC request; each response is written
    as one JSON line followed by a flush. Malformed JSON yields a parse-error
    response. Blank lines are ignored.
    """
    server = server or MCPServer()
    in_stream = stdin or sys.stdin
    out_stream = stdout or sys.stdout

    def write(response: Dict[str, Any]) -> None:
        out_stream.write(json.dumps(response, ensure_ascii=False))
        out_stream.write("\n")
        out_stream.flush()

    for line in in_stream:
        line = line.strip()
        if not line:
            continue
        try:
            request = json.loads(line)
        except json.JSONDecodeError:
            write(_error(None, PARSE_ERROR, "Invalid JSON"))
            continue
        response = server.handle(request)
        if response is not None:
            write(response)


if __name__ == "__main__":
    serve()
