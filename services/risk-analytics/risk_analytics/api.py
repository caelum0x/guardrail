"""FastAPI surface for the risk-analytics service.

Import-guarded: the module imports without FastAPI installed (so `metrics` and
the CLI work standalone); `create_app()` raises a clear error if FastAPI is
missing. Run with: `uvicorn risk_analytics.api:app` once `fastapi`/`uvicorn`
are installed (see pyproject.toml).
"""

from __future__ import annotations

from . import metrics, store

try:
    from fastapi import FastAPI
    from pydantic import BaseModel

    _HAVE_FASTAPI = True
except ImportError:  # pragma: no cover - exercised only without the dep
    _HAVE_FASTAPI = False


def create_app():
    """Build the FastAPI app. Raises RuntimeError if FastAPI is not installed."""
    if not _HAVE_FASTAPI:
        raise RuntimeError("FastAPI is not installed — `pip install fastapi uvicorn`")

    app = FastAPI(title="guardrail-risk-analytics", version="0.1.0")

    class MetricsRequest(BaseModel):
        equity: list[float]
        periods_per_year: float = metrics.DEFAULT_PERIODS_PER_YEAR

    class CorrelationRequest(BaseModel):
        series: dict[str, list[float]]

    @app.get("/health")
    def health() -> dict:
        return {"status": "ok", "service": "risk-analytics"}

    @app.post("/metrics")
    def compute_metrics(req: MetricsRequest) -> dict:
        return metrics.summary(req.equity, req.periods_per_year)

    @app.post("/correlation")
    def correlation(req: CorrelationRequest) -> dict:
        return metrics.correlation_matrix(req.series)

    @app.get("/metrics/live")
    def live_metrics(db: str = store.DEFAULT_DB) -> dict:
        equity = store.equity_series(db)
        if len(equity) < 2:
            return {"error": "not enough NAV points in the event log", "points": len(equity)}
        return metrics.summary(equity)

    return app


# Module-level app for `uvicorn risk_analytics.api:app` (only when FastAPI exists).
app = create_app() if _HAVE_FASTAPI else None
