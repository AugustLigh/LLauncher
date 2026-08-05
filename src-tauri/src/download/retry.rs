//! Retry wrapper for individual file downloads inside a concurrent batch.

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::error::AppError;

/// How many extra attempts a single file gets before its failure is allowed
/// to take down the whole batch.
pub const MAX_RETRIES: u32 = 2;

/// Delay between retries. Fixed rather than exponential: these are typically
/// transient CDN/network blips on multi-GB transfers, not a server under
/// sustained load that backoff would need to ease off from.
pub const RETRY_DELAY: Duration = Duration::from_secs(2);

/// Run a single file's download, retrying it in place on failure instead of
/// letting one flaky file abort every other file downloading concurrently in
/// the same batch. Cancellation is never retried — it propagates immediately.
pub async fn with_retry<F, Fut>(
    cancel_flag: &Arc<AtomicBool>,
    max_retries: u32,
    retry_delay: Duration,
    mut attempt: F,
) -> Result<(), AppError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<(), AppError>>,
{
    let mut retries_left = max_retries;
    loop {
        if !cancel_flag.load(Ordering::SeqCst) {
            return Err(AppError::Cancelled);
        }
        match attempt().await {
            Ok(()) => return Ok(()),
            Err(AppError::Cancelled) => return Err(AppError::Cancelled),
            Err(_) if retries_left > 0 => {
                retries_left -= 1;
                tokio::time::sleep(retry_delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    #[tokio::test]
    async fn retries_a_failing_attempt_before_giving_up() {
        // A flaky download that keeps failing must still bail out eventually
        // (not retry forever) — exactly max_retries + 1 attempts total.
        let cancel_flag = Arc::new(AtomicBool::new(true));
        let calls = Arc::new(AtomicU32::new(0));
        let calls2 = calls.clone();

        let result = with_retry(&cancel_flag, 2, Duration::from_millis(1), move || {
            let calls = calls2.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Err(AppError::Api("transient".to_string()))
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn succeeds_without_exhausting_retries() {
        // A file that fails once and then succeeds on retry must return Ok,
        // not propagate the first attempt's transient error.
        let cancel_flag = Arc::new(AtomicBool::new(true));
        let calls = Arc::new(AtomicU32::new(0));
        let calls2 = calls.clone();

        let result = with_retry(&cancel_flag, 2, Duration::from_millis(1), move || {
            let calls = calls2.clone();
            async move {
                if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                    Err(AppError::Api("transient".to_string()))
                } else {
                    Ok(())
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn never_retries_a_cancellation() {
        // Cancellation must propagate immediately — retrying it would delay
        // the user's "Cancel" button by up to MAX_RETRIES * RETRY_DELAY.
        let cancel_flag = Arc::new(AtomicBool::new(true));
        let calls = Arc::new(AtomicU32::new(0));
        let calls2 = calls.clone();

        let result = with_retry(&cancel_flag, 2, Duration::from_millis(1), move || {
            let calls = calls2.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Err(AppError::Cancelled)
            }
        })
        .await;

        assert!(matches!(result, Err(AppError::Cancelled)));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
