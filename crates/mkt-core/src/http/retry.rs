//! Retry with exponential backoff for transient provider errors.
//!
//! Providers wrap their HTTP calls in [`retry`]: reads repeat on any
//! transient failure, writes only when the request provably did not
//! execute (rate-limited before processing, or the connection was never
//! established), so a timed-out create cannot duplicate spend. Server
//! `Retry-After` hints take precedence over the computed backoff.

use std::time::Duration;

use crate::error::{MktError, Result};

/// Server hints above this are clamped: a CLI should fail with the
/// documented rate-limit exit code rather than sleep for many minutes.
const MAX_HINT_SECS: u64 = 120;

/// Parse a `Retry-After` response header into seconds.
///
/// Only the delta-seconds form is honored; the HTTP-date form is rare on
/// ad APIs and falls back to the caller's default.
#[must_use]
pub fn retry_after_secs(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// Whether the operation is safe to repeat after a failed attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    /// Idempotent reads: any transient failure is retryable.
    Read,
    /// Writes: retry only failures that happened before the API could
    /// act (rate limits, connection failures).
    Write,
}

/// Exponential backoff policy.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Total attempts, including the first (1 = no retries).
    pub max_attempts: u32,
    /// Delay before the first retry; doubles each attempt.
    pub min_delay: Duration,
    /// Ceiling for the computed backoff.
    pub max_delay: Duration,
}

impl RetryPolicy {
    /// Production default: 4 attempts backing off 1s → 2s → 4s (+ jitter).
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            max_attempts: 4,
            min_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
        }
    }

    /// Single attempt, no retries — for tests and latency-sensitive paths.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            max_attempts: 1,
            min_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
        }
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::standard()
    }
}

/// Whether `error` is worth retrying for this kind of operation.
fn is_retryable(kind: OpKind, error: &MktError) -> bool {
    match kind {
        OpKind::Read => error.is_transient(),
        OpKind::Write => match error {
            MktError::RateLimited { .. } => true,
            MktError::Http(e) => e.is_connect(),
            _ => false,
        },
    }
}

/// The server-suggested wait, when the error carries one.
fn retry_hint(error: &MktError) -> Option<Duration> {
    let secs = match error {
        MktError::RateLimited {
            retry_after_secs, ..
        } => Some(*retry_after_secs),
        MktError::ApiError {
            retry_after: Some(secs),
            ..
        } => Some(*secs),
        _ => None,
    }?;
    Some(Duration::from_secs(secs.min(MAX_HINT_SECS)))
}

/// Add up to 20% of `delay` as jitter so synchronized clients spread out.
fn with_jitter(delay: Duration) -> Duration {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    delay + delay.mul_f64(f64::from(nanos % 21) / 100.0)
}

/// Run `op`, retrying per `policy` while failures stay retryable.
///
/// # Errors
///
/// Returns the last error once attempts are exhausted or the failure is
/// not retryable for this [`OpKind`].
pub async fn retry<T, F, Fut>(policy: &RetryPolicy, kind: OpKind, mut op: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        let error = match op().await {
            Ok(value) => return Ok(value),
            Err(error) => error,
        };
        if attempt >= policy.max_attempts || !is_retryable(kind, &error) {
            return Err(error);
        }

        let backoff = policy
            .min_delay
            .saturating_mul(2_u32.saturating_pow(attempt - 1))
            .min(policy.max_delay);
        let delay = retry_hint(&error).unwrap_or_else(|| with_jitter(backoff));
        tracing::warn!(
            attempt,
            delay_ms = u64::try_from(delay.as_millis()).unwrap_or(u64::MAX),
            error = %error,
            "transient provider error; retrying"
        );
        tokio::time::sleep(delay).await;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    fn transient_error() -> MktError {
        MktError::ApiError {
            provider: "test".into(),
            status: 503,
            message: "unavailable".into(),
            retry_after: None,
        }
    }

    fn rate_limited(secs: u64) -> MktError {
        MktError::RateLimited {
            provider: "test".into(),
            retry_after_secs: secs,
        }
    }

    fn validation_error() -> MktError {
        MktError::ValidationError {
            field: "f".into(),
            message: "bad".into(),
        }
    }

    #[allow(clippy::future_not_send)] // single-threaded test helper
    async fn run_counting(
        policy: &RetryPolicy,
        kind: OpKind,
        failures: u32,
        error_fn: impl Fn() -> MktError,
    ) -> (Result<u32>, u32) {
        let calls = AtomicU32::new(0);
        let result = retry(policy, kind, || {
            let n = calls.fetch_add(1, Ordering::SeqCst) + 1;
            let error = (n <= failures).then(&error_fn);
            async move { error.map_or_else(|| Ok(n), Err) }
        })
        .await;
        (result, calls.load(Ordering::SeqCst))
    }

    #[tokio::test(start_paused = true)]
    async fn read_retries_transient_until_success() {
        let (result, calls) =
            run_counting(&RetryPolicy::standard(), OpKind::Read, 2, transient_error).await;
        assert_eq!(result.unwrap(), 3);
        assert_eq!(calls, 3);
    }

    #[tokio::test(start_paused = true)]
    async fn exhausted_attempts_return_last_error() {
        let (result, calls) =
            run_counting(&RetryPolicy::standard(), OpKind::Read, 99, transient_error).await;
        assert!(result.unwrap_err().is_transient());
        assert_eq!(calls, 4, "standard policy makes 4 attempts");
    }

    #[tokio::test(start_paused = true)]
    async fn non_transient_errors_never_retry() {
        let (result, calls) =
            run_counting(&RetryPolicy::standard(), OpKind::Read, 99, validation_error).await;
        assert!(matches!(
            result.unwrap_err(),
            MktError::ValidationError { .. }
        ));
        assert_eq!(calls, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn policy_none_makes_a_single_attempt() {
        let (result, calls) =
            run_counting(&RetryPolicy::none(), OpKind::Read, 99, transient_error).await;
        assert!(result.is_err());
        assert_eq!(calls, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn writes_do_not_retry_server_errors() {
        let (result, calls) =
            run_counting(&RetryPolicy::standard(), OpKind::Write, 99, transient_error).await;
        assert!(result.is_err());
        assert_eq!(calls, 1, "a 503 may have executed the write");
    }

    #[tokio::test(start_paused = true)]
    async fn writes_retry_rate_limits() {
        let (result, calls) = run_counting(&RetryPolicy::standard(), OpKind::Write, 1, || {
            rate_limited(7)
        })
        .await;
        assert_eq!(result.unwrap(), 2);
        assert_eq!(calls, 2);
    }

    #[tokio::test(start_paused = true)]
    async fn server_hint_overrides_backoff() {
        let start = tokio::time::Instant::now();
        let (result, _) = run_counting(&RetryPolicy::standard(), OpKind::Read, 1, || {
            rate_limited(7)
        })
        .await;
        assert!(result.is_ok());
        let waited = start.elapsed();
        assert!(
            waited >= Duration::from_secs(7) && waited < Duration::from_secs(8),
            "should sleep the hinted 7s, slept {waited:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn absurd_hints_are_clamped() {
        let start = tokio::time::Instant::now();
        let (result, _) = run_counting(&RetryPolicy::standard(), OpKind::Read, 1, || {
            rate_limited(86_400)
        })
        .await;
        assert!(result.is_ok());
        assert!(
            start.elapsed() <= Duration::from_secs(MAX_HINT_SECS + 1),
            "hints are clamped to {MAX_HINT_SECS}s"
        );
    }
}
