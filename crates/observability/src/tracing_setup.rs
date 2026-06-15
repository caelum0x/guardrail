//! Process-wide tracing initialization.
//!
//! Applications call [`init`] once at startup. It is safe to call more than
//! once: a [`std::sync::Once`] guard plus `try_init` ensures a second call is a
//! no-op rather than a panic.

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize the global tracing subscriber.
///
/// Idempotent: only the first call installs a subscriber. Subsequent calls
/// return without effect, so binaries and tests can call it freely.
pub fn init() {
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();
    });
}
