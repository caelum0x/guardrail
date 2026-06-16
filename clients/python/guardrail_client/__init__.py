"""Stdlib-only Python client for the Guardrail Alpha read-only API.

Dependency-free: uses only ``urllib.request`` and ``json`` from the standard
library (no ``requests``/``httpx``). Every method maps to one API route. The
API is read-only -- this client never mutates agent state.

Decimal-valued fields are serialized as strings by the Rust backend to avoid
float drift, so they are returned here as strings inside the parsed dicts.
"""

from __future__ import annotations

import json
import urllib.error
import urllib.parse
import urllib.request
from typing import Any, Dict, List, Optional

from .proof import (
    BSCSCAN_BASE_URL,
    COMPETITION_CONTRACT,
    COMPETITION_CONTRACT_BSCTRACE,
    REPORT_CORE_FIELDS,
    Check,
    VerifyResult,
    agent_id_for,
    render_report,
    report_hash_for,
    verify_proof,
)

__all__ = [
    "GuardrailClient",
    "GuardrailApiError",
    "DEFAULT_BASE_URL",
    # Proof verifier (offline, stdlib-only).
    "verify_proof",
    "render_report",
    "VerifyResult",
    "Check",
    "agent_id_for",
    "report_hash_for",
    "BSCSCAN_BASE_URL",
    "COMPETITION_CONTRACT",
    "COMPETITION_CONTRACT_BSCTRACE",
    "REPORT_CORE_FIELDS",
]

DEFAULT_BASE_URL = "http://localhost:8080"


class GuardrailApiError(Exception):
    """Raised when the API responds with a non-2xx status or is unreachable.

    Attributes:
        status: HTTP status code, or ``None`` when the request never completed
            (for example a connection error or timeout).
        path: The request path that failed.
        body: The raw response body, when one was returned.
    """

    def __init__(
        self,
        message: str,
        *,
        status: Optional[int] = None,
        path: Optional[str] = None,
        body: Optional[str] = None,
    ) -> None:
        super().__init__(message)
        self.status = status
        self.path = path
        self.body = body


class GuardrailClient:
    """Read-only client for the Guardrail Alpha API.

    Example:
        >>> client = GuardrailClient(base_url="http://localhost:8080")
        >>> health = client.health()
        >>> bt = client.backtest(steps=60, fear_greed=70, preset="balanced")
    """

    def __init__(
        self,
        base_url: str = DEFAULT_BASE_URL,
        timeout: float = 10.0,
    ) -> None:
        self._base_url = base_url.rstrip("/")
        self._timeout = timeout

    # --- Internal helpers ----------------------------------------------------
    def _request(self, path: str, accept: str) -> str:
        url = f"{self._base_url}{path}"
        req = urllib.request.Request(url, headers={"Accept": accept})
        try:
            with urllib.request.urlopen(req, timeout=self._timeout) as resp:
                raw = resp.read()
                charset = resp.headers.get_content_charset() or "utf-8"
                return raw.decode(charset)
        except urllib.error.HTTPError as exc:
            body: Optional[str] = None
            try:
                body = exc.read().decode("utf-8", errors="replace")
            except Exception:  # noqa: BLE001 - body is best-effort only
                body = None
            raise GuardrailApiError(
                f"GET {path} failed: {exc.code} {exc.reason}",
                status=exc.code,
                path=path,
                body=body,
            ) from exc
        except urllib.error.URLError as exc:
            raise GuardrailApiError(
                f"GET {path} failed: {exc.reason}",
                path=path,
            ) from exc

    def _get_json(self, path: str) -> Dict[str, Any]:
        text = self._request(path, accept="application/json")
        try:
            parsed = json.loads(text)
        except json.JSONDecodeError as exc:
            raise GuardrailApiError(
                f"GET {path} returned invalid JSON: {exc}",
                path=path,
                body=text,
            ) from exc
        if not isinstance(parsed, dict):
            raise GuardrailApiError(
                f"GET {path} expected a JSON object, got {type(parsed).__name__}",
                path=path,
                body=text,
            )
        return parsed

    def _get_text(self, path: str) -> str:
        return self._request(path, accept="text/plain")

    @staticmethod
    def _build_path(path: str, params: Dict[str, str]) -> str:
        if not params:
            return path
        return f"{path}?{urllib.parse.urlencode(params)}"

    # --- Status & state ------------------------------------------------------
    def health(self) -> Dict[str, Any]:
        """API + database status (``/health``)."""
        return self._get_json("/health")

    def cockpit(self) -> Dict[str, Any]:
        """Aggregated live view (``/cockpit``)."""
        return self._get_json("/cockpit")

    def portfolio(self) -> Dict[str, Any]:
        """Latest reconciliation (``/portfolio``)."""
        return self._get_json("/portfolio")

    def risk(self) -> Dict[str, Any]:
        """Risk events + kill switch (``/risk``)."""
        return self._get_json("/risk")

    def alerts(self) -> Dict[str, Any]:
        """Evaluated alerts (``/alerts``)."""
        return self._get_json("/alerts")

    def proof(self) -> Dict[str, Any]:
        """Agent identity + report proof (``/proof``)."""
        return self._get_json("/proof")

    def verify_proof(self, policy_raw: Optional[bytes] = None) -> VerifyResult:
        """Fetch ``/proof`` and verify its commitments offline.

        Fetches the agent identity + report proof envelope, then independently
        re-derives ``agent_id`` / ``report_hash`` (and optionally ``policy_hash``
        when ``policy_raw`` is supplied) using only the standard library. The
        result agrees byte-for-byte with the Go and TypeScript ports.
        """
        proof = self.proof()
        return verify_proof(proof, policy_raw=policy_raw)

    def events(self) -> Dict[str, Any]:
        """Recent event log (``/events``)."""
        return self._get_json("/events")

    def history(self) -> Dict[str, Any]:
        """NAV equity series (``/history``)."""
        return self._get_json("/history")

    def metrics(self) -> str:
        """Prometheus exposition text from the API ``/metrics`` route."""
        return self._get_text("/metrics")

    # --- Research ------------------------------------------------------------
    def backtest(
        self,
        steps: Optional[int] = None,
        fear_greed: Optional[int] = None,
        preset: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Strategy vs benchmark backtest (``/backtest``)."""
        params: Dict[str, str] = {}
        if steps is not None:
            params["steps"] = str(steps)
        if fear_greed is not None:
            params["fear_greed"] = str(fear_greed)
        if preset:
            params["preset"] = preset
        return self._get_json(self._build_path("/backtest", params))

    def walkforward(
        self,
        windows: Optional[int] = None,
        steps: Optional[int] = None,
        preset: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Rolling walk-forward windows (``/walkforward``)."""
        params: Dict[str, str] = {}
        if windows is not None:
            params["windows"] = str(windows)
        if steps is not None:
            params["steps"] = str(steps)
        if preset:
            params["preset"] = preset
        return self._get_json(self._build_path("/walkforward", params))

    def sweep(
        self,
        steps: Optional[int] = None,
        fear_greed: Optional[List[int]] = None,
        preset: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Sentiment comparison sweep (``/sweep``)."""
        params: Dict[str, str] = {}
        if steps is not None:
            params["steps"] = str(steps)
        if fear_greed:
            params["fear_greed"] = ",".join(str(v) for v in fear_greed)
        if preset:
            params["preset"] = preset
        return self._get_json(self._build_path("/sweep", params))

    def trades(self) -> Dict[str, Any]:
        """Recent trades (``/trades``)."""
        return self._get_json("/trades")

    def signals(self) -> Dict[str, Any]:
        """Latest signals (``/signals``)."""
        return self._get_json("/signals")

    def readiness(self) -> Dict[str, Any]:
        """Readiness probe (``/readiness``)."""
        return self._get_json("/readiness")

    def exposure(self) -> Dict[str, Any]:
        """Portfolio exposure (``/exposure``)."""
        return self._get_json("/exposure")

    def briefing(self) -> Dict[str, Any]:
        """Operator briefing (``/briefing``)."""
        return self._get_json("/briefing")

    def budget(self) -> Dict[str, Any]:
        """Budget status (``/budget``)."""
        return self._get_json("/budget")

    def heartbeat(self) -> Dict[str, Any]:
        """Heartbeat status (``/heartbeat``)."""
        return self._get_json("/heartbeat")

    def costs(self) -> Dict[str, Any]:
        """Cost accounting (``/costs``)."""
        return self._get_json("/costs")

    def drift(self) -> Dict[str, Any]:
        """Allocation drift (``/drift``)."""
        return self._get_json("/drift")

    def exit_triggers(self) -> Dict[str, Any]:
        """Exit triggers (``/exit-triggers``)."""
        return self._get_json("/exit-triggers")

    def liquidity(self) -> Dict[str, Any]:
        """Liquidity view (``/liquidity``)."""
        return self._get_json("/liquidity")

    def quotes(self) -> Dict[str, Any]:
        """Latest quotes (``/quotes``)."""
        return self._get_json("/quotes")

    def watchlist(self) -> Dict[str, Any]:
        """Watchlist (``/watchlist``)."""
        return self._get_json("/watchlist")

    def rebalance(self) -> Dict[str, Any]:
        """Rebalance plan (``/rebalance``)."""
        return self._get_json("/rebalance")

    def scenarios(self) -> Dict[str, Any]:
        """Stress scenarios (``/scenarios``)."""
        return self._get_json("/scenarios")

    # --- Market & research ---------------------------------------------------
    def assets(self) -> Dict[str, Any]:
        """Tracked assets (``/assets``)."""
        return self._get_json("/assets")

    def trending(self) -> Dict[str, Any]:
        """Trending assets (``/trending``)."""
        return self._get_json("/trending")

    def regime(self) -> Dict[str, Any]:
        """Market regime (``/regime``)."""
        return self._get_json("/regime")

    def funding(self) -> Dict[str, Any]:
        """Funding rates (``/funding``)."""
        return self._get_json("/funding")

    def mandates(self) -> Dict[str, Any]:
        """Mandate catalog (``/mandates``)."""
        return self._get_json("/mandates")

    def experiments(self) -> Dict[str, Any]:
        """Experiment log (``/experiments``)."""
        return self._get_json("/experiments")

    def indicators(
        self,
        symbol: Optional[str] = None,
        steps: Optional[int] = None,
    ) -> Dict[str, Any]:
        """Deterministic synthetic indicators for a symbol (``/indicators``)."""
        params: Dict[str, str] = {}
        if symbol:
            params["symbol"] = symbol
        if steps is not None:
            params["steps"] = str(steps)
        return self._get_json(self._build_path("/indicators", params))

    def optimize(
        self,
        symbols: Optional[List[str]] = None,
        scores: Optional[List[float]] = None,
        vols: Optional[List[float]] = None,
    ) -> Dict[str, Any]:
        """Portfolio weight optimization for a basket (``/optimize``)."""
        params: Dict[str, str] = {}
        if symbols:
            params["symbols"] = ",".join(symbols)
        if scores:
            params["scores"] = ",".join(str(v) for v in scores)
        if vols:
            params["vols"] = ",".join(str(v) for v in vols)
        return self._get_json(self._build_path("/optimize", params))

    # --- Governance & catalog ------------------------------------------------
    def universe(self) -> Dict[str, Any]:
        """Trading universe (``/universe``)."""
        return self._get_json("/universe")

    def config(self) -> Dict[str, Any]:
        """Config inventory (``/config``)."""
        return self._get_json("/config")

    def ops(self) -> Dict[str, Any]:
        """Ops status (``/ops``)."""
        return self._get_json("/ops")

    def policy(self) -> Dict[str, Any]:
        """Active policy (``/policy``)."""
        return self._get_json("/policy")

    def signing_policy(self) -> Dict[str, Any]:
        """Signing policy (``/signing-policy``)."""
        return self._get_json("/signing-policy")

    def wallet_controls(self) -> Dict[str, Any]:
        """Wallet controls (``/wallet-controls``)."""
        return self._get_json("/wallet-controls")

    def playbook(self) -> Dict[str, Any]:
        """Operator playbook (``/playbook``)."""
        return self._get_json("/playbook")

    def prizes(self) -> Dict[str, Any]:
        """Prize catalog (``/prizes``)."""
        return self._get_json("/prizes")

    def commerce(self) -> Dict[str, Any]:
        """Commerce view (``/commerce``)."""
        return self._get_json("/commerce")

    def sdk_catalog(self) -> Dict[str, Any]:
        """SDK catalog (``/sdk-catalog``)."""
        return self._get_json("/sdk-catalog")

    def bnb_sdk(self) -> Dict[str, Any]:
        """BNB SDK metadata (``/bnb-sdk``)."""
        return self._get_json("/bnb-sdk")

    # --- Reporting & proof ---------------------------------------------------
    def report(self) -> Dict[str, Any]:
        """Structured report JSON (``/report``)."""
        return self._get_json("/report")

    def report_markdown(self) -> str:
        """Human-readable Markdown report (``/report/markdown``)."""
        return self._get_text("/report/markdown")

    def export_submission_markdown(self) -> str:
        """Competition submission Markdown (``/export/submission.md``)."""
        return self._get_text("/export/submission.md")

    def scorecard(self) -> Dict[str, Any]:
        """Judge scorecard (``/scorecard``)."""
        return self._get_json("/scorecard")

    def audit_manifest(self) -> Dict[str, Any]:
        """Submission audit manifest (``/audit-manifest``)."""
        return self._get_json("/audit-manifest")

    def skill(self) -> Dict[str, Any]:
        """Skill descriptor (``/skill``)."""
        return self._get_json("/skill")

    def compete(self) -> Dict[str, Any]:
        """Competition status (``/compete``)."""
        return self._get_json("/compete")

    def job_simulator(self) -> Dict[str, Any]:
        """Job simulator (``/job-simulator``)."""
        return self._get_json("/job-simulator")

    # --- Agent identity ------------------------------------------------------
    def agent_services(self) -> Dict[str, Any]:
        """Agent services (``/agent-services``)."""
        return self._get_json("/agent-services")

    def agent_card(self) -> Dict[str, Any]:
        """Agent card (``/agent-card``)."""
        return self._get_json("/agent-card")

    def well_known_agent_card(self) -> Dict[str, Any]:
        """ERC-8004 well-known agent card (``/.well-known/agent-card.json``)."""
        return self._get_json("/.well-known/agent-card.json")

    # --- Policy --------------------------------------------------------------
    def compile_policy(self, mandate: str) -> Dict[str, Any]:
        """Compile a natural-language mandate into a validated policy + hash.

        Maps to ``/policy/compile``.
        """
        params = {"mandate": mandate}
        return self._get_json(self._build_path("/policy/compile", params))

    # --- Quant tools ---------------------------------------------------------
    def ta(
        self,
        indicator: str,
        series: List[float],
        period: Optional[int] = None,
        mult: Optional[float] = None,
    ) -> Dict[str, Any]:
        """Compute a technical indicator over a close-price series (``/ta``)."""
        params: Dict[str, Any] = {
            "indicator": indicator,
            "series": ",".join(str(x) for x in series),
        }
        if period is not None:
            params["period"] = period
        if mult is not None:
            params["mult"] = mult
        return self._get_json(self._build_path("/ta", params))

    def fees(self, **params: Any) -> Dict[str, Any]:
        """Estimate the all-in cost of a swap (``/fees``).

        Accepts notional_usd, quantity, side, gas_units, gas_price_gwei,
        native_usd, pool_liquidity_usd, linear_slippage_bps, protocol_fee_bps.
        """
        clean = {k: v for k, v in params.items() if v is not None}
        return self._get_json(self._build_path("/fees", clean) if clean else "/fees")

    def sizer(self, method: str, **params: Any) -> Dict[str, Any]:
        """Compute a position size by method (``/sizer``)."""
        clean = {"method": method, **{k: v for k, v in params.items() if v is not None}}
        return self._get_json(self._build_path("/sizer", clean))

    def cmc_capabilities(self) -> Dict[str, Any]:
        """CMC data -> capability lineage descriptor (``/cmc/capabilities``)."""
        return self._get_json("/cmc/capabilities")

    def pnl(
        self, fills: Optional[str] = None, marks: Optional[str] = None
    ) -> Dict[str, Any]:
        """Average-cost PnL attribution from a fill spec (``/pnl``).

        ``fills`` is ``symbol,side,qty,price[,fee];…``; ``marks`` is ``SYM:price,…``.
        """
        params = {k: v for k, v in {"fills": fills, "marks": marks}.items() if v is not None}
        return self._get_json(self._build_path("/pnl", params) if params else "/pnl")

    def correlation(self, series: Optional[str] = None) -> Dict[str, Any]:
        """Pairwise correlation matrix over named return series (``/correlation``).

        ``series`` is ``name:v1,v2,…;name2:…``.
        """
        if series:
            return self._get_json(self._build_path("/correlation", {"series": series}))
        return self._get_json("/correlation")
