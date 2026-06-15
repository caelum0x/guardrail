use common::Decimal;

#[derive(Debug, Clone, Default)]
pub struct KillSwitch {
    triggered: bool,
    reason: Option<String>,
}

impl KillSwitch {
    pub fn trigger(&mut self, reason: impl Into<String>) {
        self.triggered = true;
        self.reason = Some(reason.into());
    }

    pub fn is_triggered(&self) -> bool {
        self.triggered
    }

    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

pub fn should_trigger(drawdown_pct: Decimal, threshold_pct: Decimal) -> bool {
    drawdown_pct >= threshold_pct
}
