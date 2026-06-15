use common::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrottleState {
    Normal,
    Soft,
    Hard,
}

pub fn drawdown_throttle(
    drawdown_pct: Decimal,
    soft_pct: Decimal,
    hard_pct: Decimal,
) -> ThrottleState {
    if drawdown_pct >= hard_pct {
        ThrottleState::Hard
    } else if drawdown_pct >= soft_pct {
        ThrottleState::Soft
    } else {
        ThrottleState::Normal
    }
}
