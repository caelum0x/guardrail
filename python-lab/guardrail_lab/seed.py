"""Deterministic synthetic-run seeder for the Guardrail Alpha analytics demo.

The real event log (``data/guardrail_alpha.db``) produced by a quick paper run
only carries a couple of NAV reconciliation points and a single regime, so the
analytics modules — regime transitions, drawdown episodes, the per-cycle
decision journal, Monte-Carlo risk, the dossier — render thin output. This
module fabricates a *rich but fully deterministic* synthetic run so the demo can
showcase every analytic against a realistic, multi-regime, multi-cycle history.

It writes to a SEPARATE demo location (``data/demo_guardrail_alpha.db`` +
``data/demo_run_report.json`` by default) and never touches the real files.

Design contract
---------------
* **Deterministic.** All randomness flows through a single
  :class:`random.Random` seeded with a caller-supplied integer. Running the
  seeder twice with the same seed produces byte-identical output.
* **Schema-faithful.** The SQLite schema, table name (``events``), columns, and
  the ``event_type`` snake_case labels exactly match what
  :func:`guardrail_lab.db.load_events` reads and what every analytic consumes
  (see :mod:`guardrail_lab.regime_analysis`, :mod:`guardrail_lab.metrics`,
  :mod:`guardrail_lab.journal`, :mod:`guardrail_lab.attribution`). The variants
  mirror the canonical Rust enum in ``crates/event-store/src/event.rs``.
* **Idempotent.** The target database is recreated from scratch on every run, so
  re-seeding never appends duplicate rows.
* **Standard-library only** (``sqlite3``, ``json``, ``uuid``, ``random``,
  ``datetime``, ``pathlib``) — runs with no pip installs.

The synthetic run walks a believable market arc::

    risk_on -> breakout -> chop -> risk_off -> risk_on

over ~40 decision cycles. Each cycle emits the full event sequence the agent
would emit in production: a market snapshot, a regime classification, a
portfolio target, scored assets, proposed orders, TWAP quotes, risk verdicts
(including clips and the occasional reject), confirmed trades, and a portfolio
reconciliation carrying an evolving NAV that draws down through the chop /
risk_off legs and recovers on the final risk_on leg.
"""

from __future__ import annotations

import json
import sqlite3
import uuid
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path
from random import Random

DEFAULT_DB = "data/demo_guardrail_alpha.db"
DEFAULT_REPORT = "data/demo_run_report.json"
DEFAULT_CYCLES = 40
DEFAULT_SEED = 20260614

#: Starting net asset value (USD) for the synthetic book.
STARTING_NAV = 10_000.0

#: Wall-clock spacing between consecutive decision cycles. A non-trivial gap
#: makes the time-in-regime / drawdown-duration / recovery-time analytics
#: produce meaningful, human-readable durations rather than sub-second noise.
CYCLE_INTERVAL = timedelta(hours=1)

#: Tiny spacing between the individual events *within* one cycle so they keep a
#: stable, strictly increasing timestamp order (the loaders sort by timestamp).
EVENT_INTERVAL = timedelta(milliseconds=80)

#: Tradable universe the synthetic agent scores and rotates through.
UNIVERSE = (
    "SHIB",
    "DOGE",
    "AVAX",
    "LTC",
    "UNI",
    "ETH",
    "BTC",
    "SOL",
    "LINK",
    "MATIC",
)

#: The canonical snake_case event-type labels (mirror of the Rust AgentEvent
#: enum) that the Python analytics consume.
EVT_AGENT_STARTED = "agent_started"
EVT_MARKET_SNAPSHOT = "market_snapshot_received"
EVT_REGIME_CLASSIFIED = "regime_classified"
EVT_PORTFOLIO_TARGET = "portfolio_target_computed"
EVT_ASSET_SCORED = "asset_scored"
EVT_ORDER_PROPOSED = "order_proposed"
EVT_TWAP_QUOTE = "twak_quote_received"
EVT_RISK_APPROVED = "risk_approved"
EVT_RISK_CLIPPED = "risk_clipped"
EVT_RISK_REJECTED = "risk_rejected"
EVT_TWAP_SUBMITTED = "twak_swap_submitted"
EVT_TX_CONFIRMED = "tx_confirmed"
EVT_PORTFOLIO_RECONCILED = "portfolio_reconciled"
EVT_DRAWDOWN_THROTTLE = "drawdown_throttle_activated"
EVT_DAILY_REQUIREMENT = "daily_trade_requirement_satisfied"
EVT_REPORT_PUBLISHED = "agent_report_published"


@dataclass(frozen=True)
class RegimeLeg:
    """One contiguous regime segment of the synthetic run.

    Attributes:
        regime: The regime label emitted in ``regime_classified`` payloads.
        cycles: Number of decision cycles spent in this regime.
        drift: Mean per-cycle NAV return (fraction) while the regime is active —
            positive for constructive legs, negative for the drawdown legs.
        volatility: Per-cycle NAV-return standard deviation (fraction).
        headline: Human-readable target headline for the cycle.
        exposure: Baseline proposed-order size (USD) — larger in risk_on /
            breakout (the agent leans in), smaller in chop / risk_off.
    """

    regime: str
    cycles: int
    drift: float
    volatility: float
    headline: str
    exposure: float


def _build_arc(total_cycles: int) -> list[RegimeLeg]:
    """Build the regime arc, scaled to roughly ``total_cycles`` cycles.

    The arc is ``risk_on -> breakout -> chop -> risk_off -> risk_on`` with the
    chop / risk_off legs carrying negative drift (the drawdown) and the final
    risk_on leg recovering. Cycle counts are distributed by fixed weights and
    rescaled to the requested total so the shape is preserved at any size.

    Args:
        total_cycles: Desired total number of decision cycles (clamped to a
            sensible minimum so every leg gets at least one cycle).

    Returns:
        The ordered list of :class:`RegimeLeg` segments.
    """
    total = max(total_cycles, 10)
    template: list[tuple[str, int, float, float, str, float]] = [
        ("risk_on", 5, 0.012, 0.006, "Risk-on — constructive allocation.", 1700.0),
        ("breakout", 4, 0.020, 0.010, "Breakout — leaning into momentum.", 1900.0),
        ("chop", 5, -0.004, 0.014, "Chop — trimming risk, holding core.", 1100.0),
        ("risk_off", 4, -0.018, 0.012, "Risk-off — de-risking the book.", 800.0),
        ("risk_on", 6, 0.016, 0.007, "Risk-on — re-engaging on recovery.", 1750.0),
    ]
    weight_total = sum(item[1] for item in template)

    legs: list[RegimeLeg] = []
    assigned = 0
    for index, (regime, weight, drift, vol, headline, exposure) in enumerate(
        template
    ):
        if index == len(template) - 1:
            cycles = total - assigned
        else:
            cycles = max(1, round(total * weight / weight_total))
        cycles = max(1, cycles)
        assigned += cycles
        legs.append(
            RegimeLeg(
                regime=regime,
                cycles=cycles,
                drift=drift,
                volatility=vol,
                headline=headline,
                exposure=exposure,
            )
        )
    return legs


def _iso(moment: datetime) -> str:
    """Render a UTC datetime as an ISO-8601 string with microseconds.

    Matches the timestamp format the real agent writes (e.g.
    ``2026-06-14T10:18:10.067295+00:00``), which the loaders parse.
    """
    return moment.astimezone(timezone.utc).isoformat()


def _decimal(value: float, places: int = 2) -> str:
    """Render a float as a fixed-precision decimal string.

    Payload amounts are decimal strings in the real log (the Rust side uses
    arbitrary-precision decimals); the analytics parse them back to float.
    """
    return f"{value:.{places}f}"


@dataclass
class _EventSink:
    """Accumulates ``(id, run_id, timestamp, event_type, payload)`` rows.

    A deterministic clock advances by :data:`EVENT_INTERVAL` per appended event,
    guaranteeing strictly increasing, reproducible timestamps.
    """

    run_id: str
    clock: datetime
    rows: list[tuple[str, str, str, str, str]]
    rng: Random

    def emit(self, event_type: str, payload: dict) -> None:
        """Append one event and advance the intra-cycle clock."""
        event_id = str(uuid.UUID(int=self.rng.getrandbits(128)))
        self.rows.append(
            (
                event_id,
                self.run_id,
                _iso(self.clock),
                event_type,
                json.dumps(payload, separators=(",", ":"), sort_keys=True),
            )
        )
        self.clock = self.clock + EVENT_INTERVAL


@dataclass(frozen=True)
class SeedResult:
    """Summary of a completed seeding run, for the CLI to print.

    Attributes:
        db_path: Path the event-log database was written to.
        report_path: Path the run report JSON was written to.
        run_id: The synthetic run identifier.
        cycles: Number of decision cycles generated.
        events_written: Total number of event rows inserted.
        regimes: Ordered, de-duplicated list of regimes covered.
        nav_min: Lowest reconciled NAV over the run.
        nav_max: Highest reconciled NAV over the run.
        final_nav: NAV at the end of the run.
        max_drawdown_pct: Worst peak-to-trough decline (negative percent).
        trades: Number of confirmed on-chain trades.
    """

    db_path: str
    report_path: str
    run_id: str
    cycles: int
    events_written: int
    regimes: list[str]
    nav_min: float
    nav_max: float
    final_nav: float
    max_drawdown_pct: float
    trades: int


def _score_assets(
    sink: _EventSink, rng: Random, leg: RegimeLeg
) -> list[tuple[str, float]]:
    """Emit ``asset_scored`` events for a shuffled top slice of the universe.

    Returns the ``(symbol, score)`` pairs in descending score order so the
    caller can turn the highest-conviction names into proposed orders.
    """
    universe = list(UNIVERSE)
    rng.shuffle(universe)
    top = universe[:5]
    scored: list[tuple[str, float]] = []
    base = 0.72 if leg.regime in ("risk_on", "breakout") else 0.55
    for offset, symbol in enumerate(top):
        score = round(base - offset * 0.012 + rng.uniform(-0.01, 0.01), 3)
        scored.append((symbol, score))
    scored.sort(key=lambda item: (-item[1], item[0]))
    for symbol, score in scored:
        sink.emit(EVT_ASSET_SCORED, {"symbol": symbol, "score": score})
    return scored


def _next_block(rng: Random) -> int:
    """Deterministic, monotonic-ish synthetic block number for confirmations."""
    return 40_000_000 + rng.randint(1, 9_999_999)


def _emit_cycle(
    sink: _EventSink,
    rng: Random,
    leg: RegimeLeg,
    nav: float,
    cycle_index: int,
    throttled: bool,
) -> tuple[float, int]:
    """Emit the full event sequence for a single decision cycle.

    Sequence (timestamp order): market snapshot -> regime classification ->
    portfolio target -> scored assets -> for each conviction name an
    order_proposed -> twap_quote -> risk verdict (approve / clip / reject) ->
    (on non-reject) twap_submitted -> tx_confirmed -> finally a
    portfolio_reconciled carrying the cycle's new NAV.

    Args:
        sink: The event sink to append to.
        rng: The shared deterministic RNG.
        leg: The active regime leg.
        nav: NAV at the start of the cycle.
        cycle_index: 1-based cycle number across the whole run.
        throttled: Whether a drawdown throttle is currently active (shrinks
            proposed sizing and is announced once on activation by the caller).

    Returns:
        ``(new_nav, confirmed_trades)`` — the NAV after this cycle's return is
        applied and the number of trades confirmed in the cycle.
    """
    sink.emit(
        EVT_MARKET_SNAPSHOT,
        {"assets": len(UNIVERSE), "ts": int(sink.clock.timestamp() * 1000)},
    )
    sink.emit(EVT_REGIME_CLASSIFIED, {"regime": leg.regime})

    scored = _score_assets(sink, rng, leg)
    n_orders = 3 if leg.regime in ("risk_on", "breakout") else 2
    sink.emit(
        EVT_PORTFOLIO_TARGET,
        {
            "headline": leg.headline,
            "orders": n_orders,
            "commentary": (
                "MOCK_RESPONSE: advisory text only; no execution performed."
            ),
        },
    )

    confirmed = 0
    throttle_factor = 0.6 if throttled else 1.0
    for order_index in range(n_orders):
        symbol, _score = scored[order_index % len(scored)]
        proposed = leg.exposure * throttle_factor * rng.uniform(0.92, 1.08)
        sink.emit(
            EVT_ORDER_PROPOSED,
            {
                "from": "USDT",
                "to": symbol,
                "amount_usd": _decimal(proposed),
            },
        )
        sink.emit(
            EVT_TWAP_QUOTE,
            {
                "route": f"q_{rng.randint(1, 9)}",
                "slippage_pct": _decimal(rng.uniform(0.04, 0.12), places=4),
            },
        )

        verdict = rng.random()
        # Risk-off / chop legs reject and clip more often; constructive legs
        # mostly approve. These ratios give every risk verdict real volume.
        if leg.regime in ("risk_off", "chop") and verdict < 0.12:
            sink.emit(
                EVT_RISK_REJECTED,
                {
                    "amount_usd": _decimal(proposed),
                    "reasons": [
                        "per-asset cap exceeded",
                        "regime risk budget exhausted",
                    ],
                },
            )
            continue  # rejected orders never reach the chain

        # A per-asset cap that sits above typical sizing, so well-sized orders
        # pass straight through (approved) while oversized ones are clipped to
        # the cap — giving both verdicts realistic volume.
        cap = leg.exposure * throttle_factor * 1.05
        if proposed > cap or verdict < 0.3:
            final_amount = min(proposed, cap)
            sink.emit(EVT_RISK_CLIPPED, {"amount_usd": _decimal(final_amount)})
        else:
            final_amount = proposed
            sink.emit(EVT_RISK_APPROVED, {"amount_usd": _decimal(final_amount)})

        sink.emit(EVT_TWAP_SUBMITTED, {"amount_usd": _decimal(final_amount)})
        sink.emit(
            EVT_TX_CONFIRMED,
            {
                "tx_hash": f"0x{rng.getrandbits(256):064x}",
                "block": _next_block(rng),
                "status": "confirmed",
            },
        )
        confirmed += 1

    # Apply this cycle's NAV return, then reconcile.
    ret = rng.gauss(leg.drift, leg.volatility)
    new_nav = max(nav * (1.0 + ret), 1.0)
    sink.emit(
        EVT_PORTFOLIO_RECONCILED,
        {
            "nav_usd": _decimal(new_nav),
            "positions": n_orders,
            "cycle": cycle_index,
        },
    )
    return new_nav, confirmed


def _create_schema(connection: sqlite3.Connection) -> None:
    """Create the ``events`` table + index exactly as the agent's store does."""
    connection.execute(
        """
        CREATE TABLE events (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            event_type TEXT NOT NULL,
            payload_json TEXT NOT NULL
        )
        """
    )
    connection.execute(
        """
        CREATE INDEX idx_events_run_timestamp
            ON events(run_id, timestamp)
        """
    )


def _write_run_report(
    report_path: Path,
    run_id: str,
    final_nav: float,
    nav_min: float,
    nav_max: float,
    cycles: int,
    events_written: int,
    final_regime: str,
    positions: list[tuple[str, float]],
) -> None:
    """Write a demo run report matching the shape the loaders/dossier read."""
    total = sum(value for _symbol, value in positions) or 1.0
    report = {
        "run_id": run_id,
        "mode": "paper",
        "regime": final_regime,
        "kill_switch": False,
        "agent_id": "demo-" + run_id.replace("run_", ""),
        "starting_nav_usd": _decimal(STARTING_NAV),
        "nav_usd": _decimal(final_nav),
        "total_drawdown_pct": _decimal(
            max((nav_max - nav_min) / nav_max, 0.0), places=6
        ),
        "cycles": cycles,
        "events": events_written,
        "nav_min_usd": _decimal(nav_min),
        "nav_max_usd": _decimal(nav_max),
        "wallet_address": "0xDEM00000000000000000000000000000000DEM0",
        "policy_hash": "demo_policy_hash_seeded_synthetic_run",
        "commentary": (
            "Synthetic demo run generated by guardrail_lab.seed — multi-regime, "
            "multi-cycle event log for analytics demonstration."
        ),
        "positions": [
            {
                "symbol": symbol,
                "value_usd": _decimal(value),
                "weight_pct": _decimal(value / total * 100.0),
            }
            for symbol, value in positions
        ],
        "trades": [],
    }
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


def seed_demo(
    db_path: str = DEFAULT_DB,
    report_path: str = DEFAULT_REPORT,
    cycles: int = DEFAULT_CYCLES,
    seed: int = DEFAULT_SEED,
) -> SeedResult:
    """Generate a deterministic synthetic run and write it to the demo files.

    The target database is recreated from scratch (idempotent), then a full
    multi-regime, multi-cycle event log is generated and inserted, and a
    matching run report JSON is written. The real ``data/guardrail_alpha.db`` /
    ``data/run_report.json`` are never read or written.

    Args:
        db_path: Destination SQLite database path (created/overwritten).
        report_path: Destination run report JSON path (created/overwritten).
        cycles: Approximate number of decision cycles to generate (the regime
            arc is scaled to fit this total).
        seed: Integer seed for the deterministic RNG.

    Returns:
        A :class:`SeedResult` summarizing what was written.
    """
    rng = Random(seed)
    run_id = "run_demo_" + uuid.UUID(int=rng.getrandbits(128)).hex[:24]
    start_time = datetime(2026, 5, 1, 9, 0, 0, tzinfo=timezone.utc)

    sink = _EventSink(run_id=run_id, clock=start_time, rows=[], rng=rng)

    # Run-level startup events (mirror the real log's preamble).
    sink.emit(
        EVT_AGENT_STARTED,
        {
            "agent_id": "demo-" + run_id.replace("run_demo_", ""),
            "mode": "paper",
            "policy_hash": "demo_policy_hash_seeded_synthetic_run",
            "wallet": "0xDEM00000000000000000000000000000000DEM0",
        },
    )

    arc = _build_arc(cycles)
    regimes_covered: list[str] = []

    nav = STARTING_NAV
    nav_min = nav
    nav_max = nav
    total_trades = 0
    throttled = False
    cycle_index = 0
    daily_announced = False

    for leg in arc:
        if leg.regime not in regimes_covered:
            regimes_covered.append(leg.regime)
        for _ in range(leg.cycles):
            cycle_index += 1
            # Align each cycle's first event to the cycle clock so wall-clock
            # durations between regime classifications are meaningful.
            sink.clock = start_time + CYCLE_INTERVAL * cycle_index

            peak_so_far = nav_max
            # Announce a drawdown throttle once the book is meaningfully
            # underwater, and lift it once recovered — gives the throttle event
            # a realistic trigger tied to NAV, not a coin flip.
            underwater = nav < peak_so_far * 0.94
            if underwater and not throttled:
                throttled = True
                sink.emit(
                    EVT_DRAWDOWN_THROTTLE,
                    {
                        "drawdown_pct": _decimal(
                            (nav - peak_so_far) / peak_so_far * 100.0, places=4
                        ),
                        "action": "halved_position_sizing",
                    },
                )
            elif throttled and nav >= peak_so_far * 0.985:
                throttled = False

            nav, confirmed = _emit_cycle(
                sink, rng, leg, nav, cycle_index, throttled
            )
            total_trades += confirmed
            nav_min = min(nav_min, nav)
            nav_max = max(nav_max, nav)

            if not daily_announced and total_trades >= 1:
                daily_announced = True
                sink.emit(
                    EVT_DAILY_REQUIREMENT,
                    {"trades": total_trades, "satisfied": True},
                )

    final_regime = arc[-1].regime if arc else "unknown"

    # Final positions reconstructed from the last cycle's conviction names.
    final_universe = list(UNIVERSE)
    rng.shuffle(final_universe)
    per_position = nav / 5.0
    positions = [
        (symbol, round(per_position * rng.uniform(0.85, 1.15), 2))
        for symbol in final_universe[:5]
    ]

    sink.emit(
        EVT_REPORT_PUBLISHED,
        {
            "final_nav": _decimal(nav),
            "cycles": cycle_index,
            "regime": final_regime,
        },
    )

    # --- Write the database (idempotent: recreate cleanly). ---
    destination = Path(db_path)
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists():
        destination.unlink()

    connection = sqlite3.connect(str(destination))
    try:
        _create_schema(connection)
        connection.executemany(
            "INSERT INTO events "
            "(id, run_id, timestamp, event_type, payload_json) "
            "VALUES (?, ?, ?, ?, ?)",
            sink.rows,
        )
        connection.commit()
    finally:
        connection.close()

    _write_run_report(
        Path(report_path),
        run_id=run_id,
        final_nav=nav,
        nav_min=nav_min,
        nav_max=nav_max,
        cycles=cycle_index,
        events_written=len(sink.rows),
        final_regime=final_regime,
        positions=positions,
    )

    max_dd_pct = (
        round((nav_min - nav_max) / nav_max * 100.0, 4) if nav_max > 0 else 0.0
    )

    return SeedResult(
        db_path=str(destination),
        report_path=str(report_path),
        run_id=run_id,
        cycles=cycle_index,
        events_written=len(sink.rows),
        regimes=regimes_covered,
        nav_min=round(nav_min, 2),
        nav_max=round(nav_max, 2),
        final_nav=round(nav, 2),
        max_drawdown_pct=max_dd_pct,
        trades=total_trades,
    )
