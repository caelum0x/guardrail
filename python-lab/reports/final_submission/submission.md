# Guardrail Alpha — Submission Report

## Agent Identity

- **Agent ID:** `e38b86d49c975f0b3428d973141b89cda5281c2b330bfc29d9c418fb078012a4`
- **Wallet:** `0xA9e5C0FfEe0000000000000000000000000A1b2C3`
- **Policy hash:** `e21adc91f722c0e7923f0d9546ea1766ad06d0155cd62972f8af4c9bc89a6ccb`
- **Report hash:** `2d9c3c81faae8e0d8e0b777811b703780bbac7cffd2d39ada5e6b7f07f41d483`
- **Run ID:** `run_292f307ab032417cbb3fbacfdfd95248`
- **Mode:** paper

## Run Statistics

- **Cycles:** 3
- **Starting NAV:** $10,000.00
- **Final NAV:** $9,995.68
- **Reported drawdown:** 4.32%
- **Observed max drawdown:** -0.02%
- **Confirmed trades:** 5
- **Reconciliation points:** 3
- **Total events:** 48

## Trade Attribution

_Confirmed swaps grouped by destination symbol._

| Destination | Confirmed Swaps | Total Amount (USD) |
| --- | ---: | ---: |
| CAKE | 2 | $1,700.85 |
| WBNB | 2 | $1,700.85 |
| USDT | 1 | $199.92 |

## Event Counts

| Event Type | Count |
| --- | ---: |
| agent_report_published | 1 |
| agent_started | 1 |
| asset_scored | 6 |
| daily_trade_requirement_satisfied | 3 |
| market_snapshot_received | 3 |
| order_proposed | 5 |
| portfolio_reconciled | 3 |
| portfolio_target_computed | 3 |
| regime_classified | 3 |
| risk_approved | 3 |
| risk_clipped | 2 |
| twak_quote_received | 5 |
| twak_swap_submitted | 5 |
| tx_confirmed | 5 |

## Regime Timeline

- **Classifications:** 3
- **First:** `risk_on` @ 2026-06-13T20:45:21.697417+00:00
- **Last:** `risk_on` @ 2026-06-13T20:45:23.747299+00:00

| Regime | Count |
| --- | ---: |
| risk_on | 3 |
