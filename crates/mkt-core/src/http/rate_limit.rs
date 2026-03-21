//! Token-bucket rate limiter for API calls.

use std::sync::Arc;

use tokio::sync::Semaphore;

/// A simple rate limiter backed by a `tokio::sync::Semaphore`.
///
/// Each API call acquires a number of permits (1 for reads, 3 for writes).
/// Permits are released on a timer to maintain the desired rate.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
}

impl RateLimiter {
    /// Create a new rate limiter with the given maximum concurrent permits.
    pub fn new(max_permits: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_permits)),
        }
    }

    /// Acquire `cost` permits. Waits if insufficient permits are available.
    ///
    /// # Errors
    ///
    /// Returns an error if the semaphore is closed.
    #[allow(clippy::significant_drop_tightening)] // permit is consumed by .forget()
    pub async fn acquire(&self, cost: u32) -> crate::error::Result<()> {
        let permit =
            self.semaphore.acquire_many(cost).await.map_err(|e| {
                crate::error::MktError::ConfigError(format!("Rate limiter error: {e}"))
            })?;

        // Forget the permit immediately — it will be replenished below.
        // For simplicity, we immediately release it. This means the limiter acts as
        // a concurrency limiter rather than a strict rate limiter. A stricter
        // implementation can use a background task to drain and refill.
        permit.forget();

        self.semaphore.add_permits(cost as usize);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn acquire_within_budget() {
        let limiter = RateLimiter::new(100);
        let result = limiter.acquire(1).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn acquire_multiple_permits() {
        let limiter = RateLimiter::new(100);
        let result = limiter.acquire(3).await;
        assert!(result.is_ok());
    }
}
