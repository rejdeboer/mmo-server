use std::time::Instant;

/// A simple token bucket rate limiter.
///
/// Tokens are replenished at a fixed rate. Each `allow()` call consumes one token.
/// If no tokens remain, the request is denied.
pub struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    /// Create a new token bucket.
    ///
    /// - `max_tokens`: burst capacity
    /// - `refill_rate`: tokens added per second
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Try to consume one token. Returns `true` if allowed.
    pub fn allow(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Returns `true` if the bucket is full (no recent activity).
    /// Used for cleanup of stale entries.
    pub fn is_full(&mut self) -> bool {
        self.refill();
        self.tokens >= self.max_tokens
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }
}
