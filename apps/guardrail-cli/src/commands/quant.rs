//! Quant tool subcommands: compute indicators, swap costs, position sizes, and
//! run the order-book matching engine locally — no API or network required.
//! These call the `ta-signals`, `fee-model`, `position-sizer`, and `orderbook`
//! crates directly.

use std::str::FromStr;

use rust_decimal::Decimal;

/// Convert an f64 CLI arg to a Decimal (via its string form, exact for inputs
/// a user would type).
fn d(x: f64) -> Decimal {
    Decimal::from_str(&x.to_string()).unwrap_or_default()
}

fn parse_series(raw: &str) -> Vec<f64> {
    raw.split(',').filter_map(|s| s.trim().parse::<f64>().ok()).collect()
}

fn fmt(v: f64) -> String {
    if v.is_nan() {
        "   —   ".to_string()
    } else {
        format!("{v:.4}")
    }
}

/// `ta` — compute a technical indicator over a close-price series.
pub fn run_ta(indicator: &str, series: &str, period: usize) -> anyhow::Result<()> {
    let data = parse_series(series);
    if data.is_empty() {
        anyhow::bail!("--series must be comma-separated numbers, e.g. 1,2,3,4,5");
    }
    println!("{indicator} (period {period}) over {} points:", data.len());
    match indicator.to_lowercase().as_str() {
        "sma" => print_one(&ta_signals::sma(&data, period)),
        "ema" => print_one(&ta_signals::ema(&data, period)),
        "rsi" => print_one(&ta_signals::rsi(&data, period)),
        "bollinger" => {
            let (u, m, l) = ta_signals::bollinger(&data, period, 2.0);
            println!("  upper:  {}", join(&u));
            println!("  middle: {}", join(&m));
            println!("  lower:  {}", join(&l));
        }
        "macd" => {
            let (macd, sig, hist) = ta_signals::macd(&data, 12, 26, 9);
            println!("  macd:   {}", join(&macd));
            println!("  signal: {}", join(&sig));
            println!("  hist:   {}", join(&hist));
        }
        other => anyhow::bail!("unknown indicator '{other}' (sma|ema|rsi|macd|bollinger)"),
    }
    Ok(())
}

fn print_one(values: &[f64]) {
    println!("  {}", join(values));
}

fn join(values: &[f64]) -> String {
    values.iter().map(|v| fmt(*v)).collect::<Vec<_>>().join("  ")
}

/// `fees` — estimate the all-in cost of a swap.
pub fn run_fees(notional: f64, quantity: f64, side: &str) -> anyhow::Result<()> {
    let swap_side = if side.eq_ignore_ascii_case("sell") {
        fee_model::SwapSide::Sell
    } else {
        fee_model::SwapSide::Buy
    };
    let model = fee_model::SwapCostModel::builder()
        .notional_usd(d(notional))
        .quantity(d(quantity))
        .side(swap_side)
        .gas(d(150_000.0), d(1.0), d(600.0))
        .pool_liquidity_usd(d(2_000_000.0))
        .linear_slippage_bps(d(5.0))
        .protocol_fee_bps(d(30.0))
        .build();
    let b = model.estimate();
    println!("swap cost ({side}, notional ${notional}, qty {quantity}):");
    println!("  gas:            ${}", b.gas_usd);
    println!("  slippage:       ${}", b.slippage_usd);
    println!("  protocol fee:   ${}", b.fee_usd);
    println!("  total:          ${}", b.total_usd);
    println!("  effective px:   {}", b.effective_price);
    println!("  cost fraction:  {}", b.total_cost_fraction);
    Ok(())
}

/// `size` — compute a position size by method.
pub fn run_size(
    method: &str,
    capital: f64,
    win_prob: f64,
    odds: f64,
) -> anyhow::Result<()> {
    match method.to_lowercase().as_str() {
        "kelly" => {
            let out = position_sizer::kelly_fraction(position_sizer::KellyInput {
                win_prob,
                odds,
                fraction: 0.5,
                cap: 0.25,
            })
            .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("kelly: edge {:.4}  full {:.4}  fractional {:.4}", out.edge, out.full_kelly, out.fractional_kelly);
        }
        "vol_target" => {
            let out = position_sizer::vol_target(position_sizer::VolTargetInput {
                capital,
                target_vol: 0.15,
                asset_vol: 0.6,
                max_leverage: 3.0,
            })
            .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("vol_target: leverage {:.4}  notional ${:.2}  capped {}", out.leverage, out.notional, out.capped);
        }
        other => anyhow::bail!("unknown method '{other}' (kelly|vol_target)"),
    }
    Ok(())
}

/// `pnl` — average-cost realized/unrealized PnL attribution from a fill spec.
/// fills: `symbol,side,qty,price[,fee];…`  marks: `SYM:price,SYM2:price2`.
pub fn run_pnl(fills: &str, marks: &str) -> anyhow::Result<()> {
    use pnl_attribution::{Attributor, Fill, Side};
    use std::collections::BTreeMap;

    let mut attr = Attributor::new();
    for (i, raw) in fills.split(';').filter(|s| !s.trim().is_empty()).enumerate() {
        let p: Vec<&str> = raw.split(',').map(str::trim).collect();
        if p.len() < 4 {
            anyhow::bail!("fill {i}: expected 'symbol,side,qty,price[,fee]'");
        }
        let side = match p[1].to_lowercase().as_str() {
            "buy" | "b" => Side::Buy,
            "sell" | "s" => Side::Sell,
            o => anyhow::bail!("fill {i}: bad side '{o}'"),
        };
        let fee = p.get(4).and_then(|s| Decimal::from_str(s).ok()).unwrap_or_default();
        attr.apply(&Fill::new(
            p[0],
            side,
            Decimal::from_str(p[2])?,
            Decimal::from_str(p[3])?,
            fee,
        ));
    }
    let mut mark_map: BTreeMap<String, Decimal> = BTreeMap::new();
    for pair in marks.split(',').filter(|s| !s.trim().is_empty()) {
        if let Some((s, px)) = pair.split_once(':') {
            if let Ok(p) = Decimal::from_str(px.trim()) {
                mark_map.insert(s.trim().to_string(), p);
            }
        }
    }
    let report = attr.report(&mark_map);
    println!("PnL attribution:");
    for r in &report.by_symbol {
        println!(
            "  {:<6} pos {} @ {}  realized {}  unrealized {}  fees {}  total {}",
            r.symbol, r.position, r.avg_cost, r.realized, r.unrealized, r.fees, r.total
        );
    }
    let t = &report.total;
    println!(
        "  TOTAL  realized {}  unrealized {}  fees {}  total {}",
        t.realized, t.unrealized, t.fees, t.total
    );
    Ok(())
}

/// `corr` — pairwise Pearson correlation matrix over named return series.
/// `series` is `name:v1,v2,…;name2:…`.
pub fn run_corr(series: &str) -> anyhow::Result<()> {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for (i, raw) in series.split(';').filter(|s| !s.trim().is_empty()).enumerate() {
        let (name, vals) = raw
            .split_once(':')
            .ok_or_else(|| anyhow::anyhow!("series {i}: expected 'name:v1,v2,…'"))?;
        let parsed: Vec<f64> = vals.split(',').filter_map(|v| v.trim().parse().ok()).collect();
        if parsed.len() < 2 {
            anyhow::bail!("series '{}' needs at least 2 values", name.trim());
        }
        map.insert(name.trim().to_string(), parsed);
    }
    if map.len() < 2 {
        anyhow::bail!("need at least 2 named series");
    }
    let m = correlation::correlation_matrix(&map);
    print!("{:>8}", "");
    for n in &m.names {
        print!("{n:>8}");
    }
    println!();
    for (i, row) in m.matrix.iter().enumerate() {
        print!("{:>8}", m.names[i]);
        for v in row {
            print!("{v:>8.2}");
        }
        println!();
    }
    Ok(())
}

/// `book` — run the matching engine over a compact order spec.
pub fn run_book(orders: &str) -> anyhow::Result<()> {
    use orderbook::{Order, OrderBook, Side};
    let mut book = OrderBook::new();
    let mut trades = 0usize;
    for (i, raw) in orders.split(';').filter(|s| !s.trim().is_empty()).enumerate() {
        let p: Vec<&str> = raw.split(',').map(str::trim).collect();
        if p.len() != 4 {
            anyhow::bail!("order {i}: expected 'side,kind,price,qty'");
        }
        let side = match p[0].to_lowercase().as_str() {
            "b" | "buy" => Side::Buy,
            "s" | "sell" => Side::Sell,
            o => anyhow::bail!("order {i}: bad side '{o}'"),
        };
        let qty = Decimal::from_str(p[3])?;
        let id = (i + 1) as u64;
        let order = if p[1].eq_ignore_ascii_case("market") {
            Order::market(id, side, qty, i as u64)
        } else {
            Order::limit(id, side, Decimal::from_str(p[2])?, qty, i as u64)
        };
        for t in book.submit(order) {
            trades += 1;
            println!("  trade: {} @ {} (taker {} / maker {})", t.quantity, t.price, t.taker_id, t.maker_id);
        }
    }
    println!(
        "{trades} trades · best bid {:?} · best ask {:?} · resting {}",
        book.best_bid(),
        book.best_ask(),
        book.len()
    );
    Ok(())
}
