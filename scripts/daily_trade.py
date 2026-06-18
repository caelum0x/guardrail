#!/usr/bin/env python3
"""Daily competition trader for the Track-1 window — TWAK-only, stdlib-only.

A deliberately conservative trader for a small self-custody portfolio:

- **Signal:** BNB momentum vs an EMA baseline (from `twak price BNB`).
  risk-on → target 50% BNB; risk-off → 0% BNB; neutral → hold.
- **Risk cap:** BNB allocation capped at 50% of NAV, so even a sharp BNB drop
  keeps drawdown well under the Track-1 30% disqualification gate.
- **Daily requirement:** always executes ≥1 swap (rebalance, or a tiny stable
  heartbeat if already balanced).
- **Window guard:** live trades only fire between WINDOW_START and WINDOW_END,
  so a daily cron can be scheduled now and stays dormant until the window.
- **Eligible only:** BNB / USDC / USDT. **Self-custody:** signs via `twak`.

Runs on plain system python3 (no third-party deps) so it works in CI / cron.
Balances + prices come from `twak`; signing uses `$TWAK_WALLET_PASSWORD` (CI) or
the local OS keychain (laptop). SAFE BY DEFAULT: dry-run unless `--live`.

Usage:
    python3 scripts/daily_trade.py          # dry-run (quotes only)
    python3 scripts/daily_trade.py --live   # real swap (gated + window-guarded)
"""

from __future__ import annotations

import argparse
import datetime
import json
import os
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
STATE = REPO / "data" / "daily_trade_state.json"
ENV = REPO / ".env"

# Track-1 live trading window (inclusive). Live trades fire only within it.
WINDOW_START = datetime.date(2026, 6, 22)
WINDOW_END = datetime.date(2026, 6, 28)

CHAIN = "bsc"
SLIPPAGE = "1"
MAX_BNB_PCT = 0.50       # hard cap on volatile exposure (drawdown safety)
MOMENTUM_BAND = 0.01     # ±1% vs EMA → regime switch
EMA_ALPHA = 0.3
MIN_TRADE_USD = 0.50
HEARTBEAT_USD = 0.50
GAS_RESERVE_BNB = 0.003  # keep this much BNB untouched for gas


def load_env() -> None:
    if ENV.exists():
        for line in ENV.read_text().splitlines():
            line = line.strip()
            if "=" in line and not line.startswith("#"):
                k, v = line.split("=", 1)
                os.environ.setdefault(k, v)


def twak(args: list[str]) -> dict:
    out = subprocess.run(["twak", *args, "--json"], capture_output=True, text=True)
    text = out.stdout.strip()
    start = text.find("{")
    if start == -1:
        raise RuntimeError(f"twak {' '.join(args)} → no JSON: {text or out.stderr.strip()}")
    data = json.loads(text[start:])
    if isinstance(data, dict) and "error" in data:
        raise RuntimeError(f"twak {' '.join(args)} → {data.get('errorCode')}: {data['error']}")
    return data


def balances() -> tuple[float, float, float]:
    d = twak(["wallet", "balance", "--chain", CHAIN])
    bnb = float(d.get("total", 0) or 0)
    toks = {t["symbol"]: float(t.get("balance", 0) or 0) for t in d.get("tokens", [])}
    return bnb, toks.get("USDC", 0.0), toks.get("USDT", 0.0)


def bnb_price() -> float:
    return float(twak(["price", "BNB"])["priceUsd"])


def read_state() -> dict:
    return json.loads(STATE.read_text()) if STATE.exists() else {"ema": None, "trades": []}


def write_state(state: dict) -> None:
    STATE.parent.mkdir(parents=True, exist_ok=True)
    STATE.write_text(json.dumps(state, indent=2))


def do_swap(frm: str, to: str, usd: float, live: bool) -> dict:
    args = ["swap", frm, to, "--usd", f"{usd:.4f}", "--chain", CHAIN, "--slippage", SLIPPAGE]
    pw = os.environ.get("TWAK_WALLET_PASSWORD")
    if live and pw:
        args += ["--password", pw]
    if not live:
        args.append("--quote-only")
    return twak(args)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="Daily Track-1 competition trade (TWAK-only).")
    ap.add_argument("--live", action="store_true", help="execute real swaps (default: dry)")
    args = ap.parse_args(argv)
    load_env()
    if not os.environ.get("TWAK_ACCESS_ID"):
        print("ERROR: TWAK_ACCESS_ID not set (check .env / CI secrets)", file=sys.stderr)
        return 2

    today = datetime.date.today()
    if args.live and not (WINDOW_START <= today <= WINDOW_END):
        print(f"{today}: outside live window {WINDOW_START}..{WINDOW_END} — no live trade.")
        return 0

    price = bnb_price()
    bnb_raw, usdc, usdt = balances()
    bnb_tradable = max(0.0, bnb_raw - GAS_RESERVE_BNB)
    bnb_val = bnb_tradable * price
    stable_val = usdc + usdt
    nav = stable_val + bnb_val

    state = read_state()
    ema = state["ema"] or price
    if price > ema * (1 + MOMENTUM_BAND):
        regime, target_pct = "risk_on", MAX_BNB_PCT
    elif price < ema * (1 - MOMENTUM_BAND):
        regime, target_pct = "risk_off", 0.0
    else:
        regime, target_pct = "neutral", (bnb_val / nav if nav else 0.0)

    target_bnb_val = min(MAX_BNB_PCT, target_pct) * nav
    delta = target_bnb_val - bnb_val

    print(f"{today} NAV ${nav:.2f} | BNB ${bnb_val:.2f} "
          f"({(bnb_val / nav * 100 if nav else 0):.0f}%) stable ${stable_val:.2f} | "
          f"price ${price:.2f} ema ${ema:.2f} | {regime} | delta ${delta:+.2f} | "
          f"{'LIVE' if args.live else 'DRY'}")

    try:
        if abs(delta) >= MIN_TRADE_USD:
            if delta > 0:
                src = "USDC" if usdc >= usdt else "USDT"
                cap = usdc if src == "USDC" else usdt
                res = do_swap(src, "BNB", min(delta, cap), args.live)
                action = f"BUY BNB ${min(delta, cap):.2f} from {src}"
            else:
                res = do_swap("BNB", "USDC", min(-delta, bnb_val), args.live)
                action = f"SELL BNB ${min(-delta, bnb_val):.2f} to USDC"
        else:
            src, dst = ("USDC", "USDT") if usdc >= HEARTBEAT_USD else ("USDT", "USDC")
            res = do_swap(src, dst, HEARTBEAT_USD, args.live)
            action = f"heartbeat {src}->{dst} ${HEARTBEAT_USD}"
        print(f"  {action}: {json.dumps(res)[:200]}")
    except RuntimeError as e:
        print(f"  trade failed: {e}", file=sys.stderr)
        return 1

    state["ema"] = EMA_ALPHA * price + (1 - EMA_ALPHA) * ema
    state["trades"].append(
        {"date": str(today), "price": price, "regime": regime, "action": action, "live": args.live}
    )
    write_state(state)
    return 0


if __name__ == "__main__":
    sys.exit(main())
