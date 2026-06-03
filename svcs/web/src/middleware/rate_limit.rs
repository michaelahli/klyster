//! Rate limiting middleware using token bucket algorithm.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

/// Rate limiter state.
#[derive(Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<RateLimiterState>>,
    requests_per_minute: u32,
    cleanup_interval: Duration,
}

struct RateLimiterState {
    buckets: HashMap<IpAddr, TokenBucket>,
    last_cleanup: Instant,
}

struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter.
    #[must_use] 
    pub fn new(requests_per_minute: u32) -> Self {
        Self {
            state: Arc::new(Mutex::new(RateLimiterState {
                buckets: HashMap::new(),
                last_cleanup: Instant::now(),
            })),
            requests_per_minute,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Check if a request from the given IP should be allowed.
    async fn check_rate_limit(&self, ip: IpAddr) -> Result<(), u64> {
        let mut state = self.state.lock().await;

        // Cleanup old buckets periodically
        if state.last_cleanup.elapsed() > self.cleanup_interval {
            state
                .buckets
                .retain(|_, bucket| bucket.last_refill.elapsed() < Duration::from_secs(120));
            state.last_cleanup = Instant::now();
        }

        let bucket = state.buckets.entry(ip).or_insert_with(|| TokenBucket {
            tokens: f64::from(self.requests_per_minute),
            last_refill: Instant::now(),
        });

        // Refill tokens based on time elapsed
        let elapsed = bucket.last_refill.elapsed();
        let tokens_to_add = (elapsed.as_secs_f64() / 60.0) * f64::from(self.requests_per_minute);
        bucket.tokens = (bucket.tokens + tokens_to_add).min(f64::from(self.requests_per_minute));
        bucket.last_refill = Instant::now();

        // Check if we have tokens available
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(())
        } else {
            // Calculate retry-after in seconds
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let retry_after =
                ((1.0 - bucket.tokens) / f64::from(self.requests_per_minute) * 60.0).ceil() as u64;
            Err(retry_after)
        }
    }
}

/// Rate limiting middleware.
pub async fn rate_limit_middleware(
    limiter: RateLimiter,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
       + Clone {
    move |request: Request, next: Next| {
        let limiter = limiter.clone();
        Box::pin(async move {
            // Skip rate limiting for health check endpoints
            let path = request.uri().path();
            if path == "/healthz" || path == "/readyz" || path == "/metrics" {
                return next.run(request).await;
            }

            // Extract IP address from request
            // In production, you'd want to check X-Forwarded-For header
            let ip = request
                .extensions()
                .get::<std::net::SocketAddr>().map_or_else(|| IpAddr::from([127, 0, 0, 1]), std::net::SocketAddr::ip);

            match limiter.check_rate_limit(ip).await {
                Ok(()) => next.run(request).await,
                Err(retry_after) => {
                    let body = serde_json::json!({
                        "error": {
                            "code": "rate_limit_exceeded",
                            "message": "Too many requests. Please try again later.",
                            "retry_after": retry_after,
                        }
                    });

                    (
                        StatusCode::TOO_MANY_REQUESTS,
                        [("Retry-After", retry_after.to_string())],
                        axum::Json(body),
                    )
                        .into_response()
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_rate_limiter_allows_requests_within_limit() {
        let limiter = RateLimiter::new(10); // 10 requests per minute
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // First 10 requests should succeed
        for _ in 0..10 {
            assert!(limiter.check_rate_limit(ip).await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_excess_requests() {
        let limiter = RateLimiter::new(5); // 5 requests per minute
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // First 5 requests should succeed
        for _ in 0..5 {
            assert!(limiter.check_rate_limit(ip).await.is_ok());
        }

        // 6th request should fail
        assert!(limiter.check_rate_limit(ip).await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_different_ips() {
        let limiter = RateLimiter::new(5);
        let ip1 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2));

        // Each IP should have its own bucket
        for _ in 0..5 {
            assert!(limiter.check_rate_limit(ip1).await.is_ok());
            assert!(limiter.check_rate_limit(ip2).await.is_ok());
        }

        // Both should be rate limited independently
        assert!(limiter.check_rate_limit(ip1).await.is_err());
        assert!(limiter.check_rate_limit(ip2).await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_token_refill() {
        let limiter = RateLimiter::new(60); // 60 requests per minute = 1 per second
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // Use up all tokens
        for _ in 0..60 {
            assert!(limiter.check_rate_limit(ip).await.is_ok());
        }

        // Should be rate limited
        assert!(limiter.check_rate_limit(ip).await.is_err());

        // Wait for 2 seconds to refill ~2 tokens
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should be able to make 2 more requests
        assert!(limiter.check_rate_limit(ip).await.is_ok());
        assert!(limiter.check_rate_limit(ip).await.is_ok());
    }
}
