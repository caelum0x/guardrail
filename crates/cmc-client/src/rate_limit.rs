//! A small token-bucket rate limiter so we stay inside CMC plan limits.

use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub struct RateLimiter {
    inner: Mutex<Bucket>,
}

struct Bucket {
    capacity: f64,
    tokens: f64,
    refill_per_sec: f64,
    last: Instant,
}

impl RateLimiter {
    /// `per_minute` requests allowed, refilled continuously.
    pub fn per_minute(per_minute: u32) -> Self {
        let capacity = per_minute.max(1) as f64;
        RateLimiter {
            inner: Mutex::new(Bucket {
                capacity,
                tokens: capacity,
                refill_per_sec: capacity / 60.0,
                last: Instant::now(),
            }),
        }
    }

    /// Wait until a token is available, then consume it.
    pub async fn acquire(&self) {
        loop {
            let wait = {
                let mut b = self.inner.lock().await;
                let now = Instant::now();
                let elapsed = now.duration_since(b.last).as_secs_f64();
                b.tokens = (b.tokens + elapsed * b.refill_per_sec).min(b.capacity);
                b.last = now;
                if b.tokens >= 1.0 {
                    b.tokens -= 1.0;
                    return;
                }
                let needed = 1.0 - b.tokens;
                Duration::from_secs_f64(needed / b.refill_per_sec)
            };
            tokio::time::sleep(wait).await;
        }
    }
}
