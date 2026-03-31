use axum::extract::connect_info::ConnectInfo;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tower::{Layer, Service};
use tracing::warn;

// ---------------------------------------------------------------------------
// Public config
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct RateLimiterConfig {
    pub max_requests: u64,
    pub window: Duration,
}

// ---------------------------------------------------------------------------
// Internal state
// ---------------------------------------------------------------------------

struct ClientWindow {
    count: u64,
    started: Instant,
}

struct RateLimiterInner {
    config: RateLimiterConfig,
    clients: Mutex<HashMap<String, ClientWindow>>,
    check_count: AtomicU64,
}

impl RateLimiterInner {
    fn new(config: RateLimiterConfig) -> Self {
        Self {
            config,
            clients: Mutex::new(HashMap::new()),
            check_count: AtomicU64::new(0),
        }
    }

    /// Returns `true` if the request is allowed, `false` if rate-limited.
    fn check(&self, ip: &str) -> bool {
        let mut clients = self.clients.lock().unwrap();
        let now = Instant::now();
        let window = self.config.window;

        // Periodic eviction: every 100 checks OR when map grows large.
        // Prevents unbounded memory growth from many unique IPs.
        let count = self.check_count.fetch_add(1, Ordering::Relaxed);
        if count % 100 == 0 || clients.len() > 10_000 {
            clients.retain(|_, w| now.duration_since(w.started) < window);
        }

        let entry = clients.entry(ip.to_string()).or_insert(ClientWindow {
            count: 0,
            started: now,
        });

        if now.duration_since(entry.started) >= window {
            entry.count = 0;
            entry.started = now;
        }

        entry.count += 1;
        entry.count <= self.config.max_requests
    }
}

// ---------------------------------------------------------------------------
// IP extraction
// ---------------------------------------------------------------------------

/// Extract client IP with this priority:
/// 1. `X-Forwarded-For` header (first IP, for reverse-proxy setups)
/// 2. `X-Real-IP` header
/// 3. Actual TCP peer address via `ConnectInfo<SocketAddr>`
/// 4. `"unknown"` fallback
fn extract_client_ip(req: &Request) -> String {
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(value) = forwarded.to_str() {
            if let Some(first_ip) = value.split(',').next() {
                let ip = first_ip.trim();
                if !ip.is_empty() {
                    return ip.to_string();
                }
            }
        }
    }

    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(value) = real_ip.to_str() {
            let ip = value.trim();
            if !ip.is_empty() {
                return ip.to_string();
            }
        }
    }

    if let Some(ConnectInfo(addr)) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
        return addr.ip().to_string();
    }

    "unknown".to_string()
}

// ---------------------------------------------------------------------------
// Tower Layer / Service
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(crate) struct RateLimitLayer {
    state: Arc<RateLimiterInner>,
}

impl RateLimitLayer {
    pub fn new(max_requests: u64, window: Duration) -> Self {
        Self {
            state: Arc::new(RateLimiterInner::new(RateLimiterConfig {
                max_requests,
                window,
            })),
        }
    }
}

impl<S: Clone> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> RateLimitService<S> {
        RateLimitService {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct RateLimitService<S> {
    inner: S,
    state: Arc<RateLimiterInner>,
}

impl<S> Service<Request> for RateLimitService<S>
where
    S: Service<Request> + Clone + Send + 'static,
    S::Response: IntoResponse + 'static,
    S::Error: Into<Infallible> + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let ip = extract_client_ip(&req);

        if !self.state.check(&ip) {
            warn!(client_ip = %ip, "rate limit exceeded");

            let retry_secs = self.state.config.window.as_secs().to_string();
            let body = serde_json::json!({
                "status": 429,
                "error": "rate limit exceeded"
            });
            let mut response =
                (StatusCode::TOO_MANY_REQUESTS, axum::Json(body)).into_response();
            response.headers_mut().insert(
                axum::http::header::RETRY_AFTER,
                axum::http::HeaderValue::from_str(&retry_secs).unwrap(),
            );
            return Box::pin(async move { Ok(response) });
        }

        let future = self.inner.call(req);
        Box::pin(async move { future.await.map(|r| r.into_response()) })
    }
}

