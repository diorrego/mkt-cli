//! Token-bucket rate limiter for API calls.
//!
//! Backed by [`governor`] (GCRA): `acquire` genuinely waits until the
//! bucket has room, unlike the previous semaphore that released permits
//! immediately and limited nothing. Each provider client constructs its
//! limiter with a conservative requests-per-second default; reads cost 1
//! cell and writes 3, so writes consume budget faster.

use std::num::NonZeroU32;
use std::sync::Arc;

use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter as GovernorLimiter};

type DirectLimiter = GovernorLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// A client-side requests-per-second limiter shared by one provider client.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    limiter: Arc<DirectLimiter>,
    max_cost: u32,
}

impl RateLimiter {
    /// Create a limiter allowing `per_second` request cells per second
    /// (burst capacity equals one second of budget, minimum 3 so a
    /// single write always fits).
    #[must_use]
    pub fn new(per_second: u32) -> Self {
        let rate = NonZeroU32::new(per_second.max(1)).unwrap_or(NonZeroU32::MIN);
        let burst = NonZeroU32::new(per_second.max(3)).unwrap_or(NonZeroU32::MIN);
        Self {
            limiter: Arc::new(GovernorLimiter::direct(
                Quota::per_second(rate).allow_burst(burst),
            )),
            max_cost: burst.get(),
        }
    }

    /// Wait until `cost` cells are available (1 = read, 3 = write).
    ///
    /// Costs above the burst capacity are clamped so the call can ever
    /// complete.
    ///
    /// # Errors
    ///
    /// Currently infallible; `Result` is kept for API stability.
    pub async fn acquire(&self, cost: u32) -> crate::error::Result<()> {
        let cost = cost.clamp(1, self.max_cost);
        let cells = NonZeroU32::new(cost).unwrap_or(NonZeroU32::MIN);
        // until_n_ready only errors when cells > burst, which the clamp
        // above rules out.
        if let Err(error) = self.limiter.until_n_ready(cells).await {
            return Err(crate::error::MktError::ConfigError(format!(
                "rate limiter misconfigured: {error}"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::time::{Duration, Instant};

    use super::*;

    #[tokio::test]
    async fn burst_within_budget_is_immediate() {
        let limiter = RateLimiter::new(100);
        let start = Instant::now();
        for _ in 0..5 {
            limiter.acquire(1).await.unwrap();
        }
        assert!(
            start.elapsed() < Duration::from_millis(100),
            "within-burst acquires must not block, took {:?}",
            start.elapsed()
        );
    }

    #[tokio::test]
    async fn exceeding_the_rate_actually_waits() {
        // 5 cells/s, burst 5: the 6th cell must wait ~200ms for refill.
        let limiter = RateLimiter::new(5);
        let start = Instant::now();
        for _ in 0..6 {
            limiter.acquire(1).await.unwrap();
        }
        assert!(
            start.elapsed() >= Duration::from_millis(120),
            "the limiter must actually delay past the burst, took {:?}",
            start.elapsed()
        );
    }

    #[tokio::test]
    async fn write_cost_exceeding_burst_is_clamped_not_stuck() {
        let limiter = RateLimiter::new(1); // burst raised to 3 internally
        limiter.acquire(3).await.unwrap();
        // A pathological cost above burst still completes.
        let limiter = RateLimiter::new(1);
        limiter.acquire(100).await.unwrap();
    }
}
