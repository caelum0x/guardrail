#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Starting,
    Running,
    Throttled,
    Killed,
    Stopped,
}
