#!/usr/bin/env python3
"""Daily competition trader for the Track-1 window — TWAK-only, no CMC key needed.

A deliberately conservative trader for a small self-custody portfolio:

- **Signal:** BNB momentum vs an EMA baseline (read from `twak price BNB`).
  risk-on → target 50% BNB; risk-off → 0% BNB; neutral → hold.
- **Risk cap:** BNB allocation is capped at 50% of NAV, so even a sharp BNB drop
  keeps portfolio drawdown well under the Track-1 30% disqualification gate.
- **Daily requirement:** always executes ≥1 swap (rebalance, or a tiny stable
  heartbeat if already balanced) to satisfy "≥1 trade/day".
- **Eligible only:** trades among BNB / USDC / USDT (all on the eligible list).
- **Self-custody:** execution + signing go through `twak` (keychain-backed).

SAFE BY DEFAULT: dry-run (quote-only, no spend) unless `--live` is passed.

Usage:
    python scripts/daily_trade.py            # dry-run (quotes only)
    python scripts/daily_trade.py --live     # real swap (gated, spends funds)

State persists in data/daily_trade_state.json (EMA baseline + trade log).
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
STATE = REPO / "data" / "daily_trade_state.json"
ENV = REPO / ".env"

BNB = "BNB"
STABLES = ("USDC", "USDT")
USDC_ADDR = "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d"
USDT_ADDR = "0x55d398326f99059fF775485246999027B3197955"
BNB_RPC = "https://bsc-dataseed.binance.org"
ADDR = "0x0c2cC53a2F8368e8FFF9D277DEEAddD08Be6f83E"

MAX_BNB_PCT = 0.50          # hard cap on volatile exposure (drawdown safety)
MOMENTUM_BAND = 0.01        # ±1% vs EMA → regime switch
EMA_ALPHA = 0.3             # baseline smoothing
MIN_TRADE_USD = 0.50        # below this, do a heartbeat instead
HEARTBEAT_USD = 0.50        # tiny stable↔stable trade to satisfy ≥1/day
SLIPPAGE = "1"
CHAIN = "bsc"


def load_env() -> None:
    if not ENV.exists():
        return
    for line in ENV.read_text().splitlines():
        line = line.strip()
        if "=" in line and not line.startswith("#"):
            k, v = line.split("=", 1)
            os.environ.setdefault(k, v)


def twak(args: list[str]) -> dict:
    """Run `twak <args> --json` and return parsed JSON (raises on the error envelope)."""
    out = subprocess.run(["twak", *args, "--json"], capture_output=True, text=True)
    # twak prints a human line before JSON on some commands; grab the JSON object.
    text = out.stdout.strip()
    start = text.find("{")
    if start == -1:
        raise RuntimeError(f"twak {' '.join(args)} → no JSON: {text or out.stderr.strip()}")
    data = json.loads(text[start:])
    if "error" in data:
        raise RuntimeError(f"twak {' '.join(args)} → {data.get('errorCode')}: {data['error']}")
    return data


def erc20_balance(token: str) -> float:
    from web3 import Web3
    w3 = Web3(Web3.HTTPProvider(BNB_RPC, request_kwargs={"timeout": 12}))
    a = Web3.to_checksum_address(ADDR)
    if token == "BNB":
        return w3.eth.get_balance(a) / 1e18
    t = Web3.to_checksum_address(token)
    data = "0x70a08231" + a[2:].rjust(64, "0")
    return int(w3.eth.call({"to": t, "data": data}).hex(), 16) / 1e18


def bnb_price() -> float:
    return float(twak(["price", BNB])["priceUsd"])


def read_state() -> dict:
    if STATE.exists():
        return json.loads(STATE.read_text())
    return {"ema": None, "trades": []}


def write_state(state: dict) -> None:
    STATE.parent.mkdir(parents=True, exist_ok=True)
    STATE.write_text(json.dumps(state, indent=2))


def do_swap(frm: str, to: str, usd: float, live: bool) -> dict:
    args = ["swap", frm, to, "--usd", f"{usd:.4f}", "--chain", CHAIN, "--slippage", SLIPPAGE]
    if not live:
        args.append("--quote-only")
    return twak(args)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="Daily Track-1 competition trade (TWAK-only).")
    ap.add_argument("--live", action="store_true", help="execute real swaps (default: dry quote-only)")
    args = ap.parse_args(argv)
    load_env()
    if not os.environ.get("TWAK_ACCESS_ID"):
        print("ERROR: TWAK_ACCESS_ID not set (check .env)", file=sys.stderr)
        return 2

    price = bnb_price()
    usdc, usdt, bnb = (erc20_balance(USDC_ADDR), erc20_balance(USDT_ADDR), erc20_balance("BNB"))
    # Reserve a little BNB for gas; only treat the rest as tradable.
    gas_reserve = 0.003
    bnb_tradable = max(0.0, bnb - gas_reserve)
    nav = usdc + usdt + bnb_tradable * price
    bnb_val = bnb_tradable * price
    stable_val = usdc + usdt

    state = read_state()
    ema = state["ema"] or price
    regime = "neutral"
    if price > ema * (1 + MOMENTUM_BAND):
        regime, target_pct = "risk_on", MAX_BNB_PCT
    elif price < ema * (1 - MOMENTUM_BAND):
        regime, target_pct = "risk_off", 0.0
    else:
        target_pct = min(MAX_BNB_PCT, bnb_val / nav if nav else 0.0)  # hold

    target_bnb_val = min(MAX_BNB_PCT, target_pct) * nav
    delta = target_bnb_val - bnb_val  # >0 buy BNB, <0 sell BNB

    print(f"NAV ${nav:.2f} | BNB ${bnb_val:.2f} ({(bnb_val/nav*100 if nav else 0):.0f}%) "
          f"stable ${stable_val:.2f} | price ${price:.2f} ema ${ema:.2f} | regime {regime} "
          f"| target BNB ${target_bnb_val:.2f} delta ${delta:+.2f} | {'LIVE' if args.live else 'DRY'}")

    try:
        if abs(delta) >= MIN_TRADE_USD:
            if delta > 0:  # buy BNB with a stable (prefer the larger stable balance)
                src = "USDC" if usdc >= usdt else "USDT"
                res = do_swap(src, BNB, min(delta, usdc if src == "USDC" else usdt), args.live)
            else:          # sell BNB into USDC
                res = do_swap(BNB, "USDC", min(-delta, bnb_val), args.live)
            action = f"rebalance {('BUY BNB' if delta>0 else 'SELL BNB')} ${abs(delta):.2f}"
        else:
            # Already balanced → tiny heartbeat to satisfy the daily-trade rule.
            src, dst = ("USDC", "USDT") if usdc >= HEARTBEAT_USD else ("USDT", "USDC")
            res = do_swap(src, dst, HEARTBEAT_USD, args.live)
            action = f"heartbeat {src}->{dst} ${HEARTBEAT_USD}"
        print(f"  {action}: {json.dumps(res)[:200]}")
    except RuntimeError as e:
        print(f"  trade failed: {e}", file=sys.stderr)
        return 1

    # Persist EMA + trade record (only count live trades as executed).
    state["ema"] = EMA_ALPHA * price + (1 - EMA_ALPHA) * ema
    state["trades"].append({"price": price, "regime": regime, "action": action, "live": args.live})
    write_state(state)
    return 0


if __name__ == "__main__":
    sys.exit(main())
