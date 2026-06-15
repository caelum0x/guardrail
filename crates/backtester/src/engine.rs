//! The backtest engine. Runs the *real* strategy, risk gate, and portfolio
//! accounting over a synthetic price path and records the equity curve.
//!
//! It deliberately reuses the production crates — `StrategyEngine`,
//! `RiskEngine`, `PortfolioState` — so a backtest validates the same logic that
//! trades live. Only the data source (synthetic) and the fill (simulated with
//! slippage + gas) are substituted.

use crate::metrics::BacktestMetrics;
use crate::{gas, slippage, synthetic};
use common::constants::RESERVE_SYMBOL;
use common::{Asset, Decimal, OrderIntent, OrderSide};
use market_data::MarketSnapshot;
use market_data::Universe;
use portfolio::trade_accounting::{apply_fill, Fill};
use portfolio::{DrawdownTracker, PortfolioState};
use risk_engine::{RiskContext, RiskDecision, RiskEngine, RiskPolicy};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strategy_engine::{CurrentAllocation, StrategyConfig, StrategyEngine};

#[derive(Debug, Clone)]
pub struct BacktestConfig {
    pub steps: u32,
    pub starting_usd: Decimal,
    pub fear_greed: u32,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        BacktestConfig {
            steps: 60,
            starting_usd: Decimal::from(10_000),
            fear_greed: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestRun {
    pub metrics: BacktestMetrics,
    pub equity_curve: Vec<Decimal>,
    pub starting_nav_usd: Decimal,
    pub final_nav_usd: Decimal,
    pub steps: u32,
    /// Equal-weight buy-and-hold return over the same path, for comparison.
    pub benchmark_return_pct: Decimal,
    /// Strategy return minus benchmark return (alpha over buy-and-hold).
    pub excess_return_pct: Decimal,
}

/// Run a backtest end to end and return the run summary.
pub fn run_backtest(
    universe: &Universe,
    policy: RiskPolicy,
    strat_cfg: StrategyConfig,
    cfg: BacktestConfig,
) -> BacktestRun {
    let assets: Vec<Asset> = universe.enabled_assets();
    let risk = RiskEngine::new(policy);
    let strategy = StrategyEngine::new(strat_cfg);

    let mut portfolio = PortfolioState::seed_stable(cfg.starting_usd);
    let mut drawdown = DrawdownTracker::new(cfg.starting_usd, 0);

    // Evolving price book; stables pinned at $1.
    let mut prices: HashMap<String, Decimal> = HashMap::new();
    prices.insert(RESERVE_SYMBOL.to_string(), Decimal::ONE);
    for a in &assets {
        if !a.category.is_stable() {
            prices.insert(a.symbol.clone(), synthetic::initial_price(&a.symbol));
        }
    }

    let mut equity_curve = Vec::with_capacity(cfg.steps as usize);
    let mut trades = 0u64;

    // Equal-weight buy-and-hold benchmark over the non-stable universe.
    let non_stable: Vec<String> = assets
        .iter()
        .filter(|a| !a.category.is_stable())
        .map(|a| a.symbol.clone())
        .collect();
    let mut benchmark = crate::benchmark::BuyAndHold::new();

    for step in 0..cfg.steps {
        // Evolve prices by this step's 24h return.
        for a in &assets {
            if a.category.is_stable() {
                continue;
            }
            let r = synthetic::step_return_24h_pct(&a.symbol, step, cfg.fear_greed);
            if let Some(p) = prices.get_mut(&a.symbol) {
                *p = (*p * (Decimal::ONE + r / Decimal::from(100))).max(Decimal::new(1, 4));
            }
        }

        // Establish the benchmark basket on the first step's prices.
        benchmark.establish(cfg.starting_usd, &prices, &non_stable);

        let snapshot = synthetic::build_snapshot(&assets, &prices, step, cfg.fear_greed);
        portfolio.mark_all(&prices);
        let nav = portfolio.nav_usd();
        drawdown.observe(nav, 0);

        let current = CurrentAllocation {
            weights_pct: portfolio.risk_weights_pct(),
        };
        let decision = strategy.decide(&snapshot, &current, nav);

        for intent in &decision.proposed_orders {
            if simulate_order(intent, &snapshot, &risk, &mut portfolio) {
                trades += 1;
            }
        }

        equity_curve.push(portfolio.nav_usd());
    }

    let final_nav = portfolio.nav_usd();
    let metrics = BacktestMetrics::from_curve(cfg.starting_usd, &equity_curve, trades);

    let benchmark_value = benchmark.value(&prices);
    let benchmark_return_pct = crate::benchmark::return_pct(cfg.starting_usd, benchmark_value);
    let excess_return_pct = (metrics.total_return_pct - benchmark_return_pct).round_dp(3);

    BacktestRun {
        metrics,
        equity_curve,
        starting_nav_usd: cfg.starting_usd,
        final_nav_usd: final_nav,
        steps: cfg.steps,
        benchmark_return_pct,
        excess_return_pct,
    }
}

/// Run one order through the risk gate and, if approved, simulate the fill.
/// Returns true when a trade was booked.
fn simulate_order(
    intent: &OrderIntent,
    snapshot: &MarketSnapshot,
    risk: &RiskEngine,
    portfolio: &mut PortfolioState,
) -> bool {
    let nav = portfolio.nav_usd();
    let ctx = risk_context(intent, snapshot, portfolio, nav);
    if let RiskDecision::Rejected { .. } = risk.pre_trade(intent, &ctx) {
        return false;
    }
    let amount = match risk.pre_trade(intent, &ctx) {
        RiskDecision::Clipped { new_amount_usd, .. } => new_amount_usd,
        _ => intent.amount_usd,
    };
    if amount <= Decimal::ZERO {
        return false;
    }

    // Simulated execution: liquidity-based slippage + fixed gas.
    let liquidity = snapshot
        .get(if intent.side == OrderSide::Buy {
            &intent.to_symbol
        } else {
            &intent.from_symbol
        })
        .and_then(|s| s.liquidity_usd)
        .unwrap_or(Decimal::from(1_000_000));
    let slippage_pct = slippage::estimate_pct(amount, liquidity);
    let gas = gas::fixed_gas_usd();

    let fill = Fill {
        from_symbol: intent.from_symbol.clone(),
        to_symbol: intent.to_symbol.clone(),
        notional_usd: amount,
        to_price_usd: price_of(&intent.to_symbol, snapshot),
        from_price_usd: price_of(&intent.from_symbol, snapshot),
        fee_usd: amount * slippage_pct / Decimal::from(100) + gas,
    };
    apply_fill(portfolio, &fill);
    true
}

fn price_of(symbol: &str, snapshot: &MarketSnapshot) -> Decimal {
    if symbol == RESERVE_SYMBOL {
        return Decimal::ONE;
    }
    snapshot
        .get(symbol)
        .map(|s| s.price_usd)
        .unwrap_or(Decimal::ONE)
}

fn risk_context(
    intent: &OrderIntent,
    snapshot: &MarketSnapshot,
    portfolio: &PortfolioState,
    nav: Decimal,
) -> RiskContext {
    let (risk_symbol, projected) = match intent.side {
        OrderSide::Buy => {
            let cur = portfolio.weight_pct(&intent.to_symbol);
            let add = if nav > Decimal::ZERO {
                intent.amount_usd / nav * Decimal::from(100)
            } else {
                Decimal::ZERO
            };
            (intent.to_symbol.clone(), cur + add)
        }
        OrderSide::Sell => (
            intent.from_symbol.clone(),
            portfolio.weight_pct(&intent.from_symbol),
        ),
    };
    RiskContext {
        nav_usd: nav,
        stable_reserve_pct: portfolio.stable_reserve_pct(),
        total_drawdown_pct: Decimal::ZERO,
        daily_drawdown_pct: Decimal::ZERO,
        target_position_pct: projected,
        security_flags: snapshot
            .get(&risk_symbol)
            .map(|s| s.security_flags.clone())
            .unwrap_or_default(),
    }
}
