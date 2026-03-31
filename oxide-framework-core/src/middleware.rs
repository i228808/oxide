use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tower::{Layer, Service};
use tracing::{info, warn};

use crate::state::AppState;

// ---------------------------------------------------------------------------
// Built-in: panic recovery (JSON 500 response)
// ---------------------------------------------------------------------------

/// Produces a JSON 500 response when a handler panics.
/// Used with `tower_http::catch_panic::CatchPanicLayer`.
pub(crate) fn panic_json_response(
    _err: Box<dyn std::any::Any + Send + 'static>,
) -> axum::http::Response<axum::body::Body> {
    let body = serde_json::json!({
        "status": 500,
        "error": "internal server error"
    });
    let bytes = serde_json::to_vec(&body).unwrap_or_default();
    axum::http::Response::builder()
        .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(bytes))
        .unwrap()
}

// ---------------------------------------------------------------------------
// Built-in: request logging middleware
// ---------------------------------------------------------------------------

/// Logs every request's method, path, response status, and latency.
///
/// Applied automatically by `App::run()`. Disable with `App::disable_request_logging()`.
pub async fn request_logger(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let start = Instant::now();

    let response = next.run(request).await;

    let latency = start.elapsed();
    let status = response.status().as_u16();

    info!(
        method = %method,
        path = %path,
        status = status,
        latency_ms = latency.as_millis() as u64,
        "request completed"
    );

    response
}

// ---------------------------------------------------------------------------
// Built-in: request timeout layer
// ---------------------------------------------------------------------------

/// Tower `Layer` that enforces a maximum request processing duration.
///
/// Returns `408 Request Timeout` with a JSON error body if the handler
/// does not complete within the configured duration.
#[derive(Clone)]
pub(crate) struct RequestTimeoutLayer {
    duration: Duration,
}

impl RequestTimeoutLayer {
    pub fn new(duration: Duration) -> Self {
        Self { duration }
    }
}

impl<S: Clone> Layer<S> for RequestTimeoutLayer {
    type Service = RequestTimeoutService<S>;

    fn layer(&self, inner: S) -> RequestTimeoutService<S> {
        RequestTimeoutService {
            inner,
            duration: self.duration,
        }
    }
}

#[derive(Clone)]
pub(crate) struct RequestTimeoutService<S> {
    inner: S,
    duration: Duration,
}

impl<S> Service<Request> for RequestTimeoutService<S>
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
        let duration = self.duration;
        let path = req.uri().path().to_string();
        let future = self.inner.call(req);

        Box::pin(async move {
            match tokio::time::timeout(duration, future).await {
                Ok(result) => result.map(|r| r.into_response()),
                Err(_) => {
                    warn!(
                        path = %path,
                        timeout_ms = duration.as_millis() as u64,
                        "request timed out"
                    );
                    let body = serde_json::json!({
                        "status": 408,
                        "error": "request timeout"
                    });
                    Ok((StatusCode::REQUEST_TIMEOUT, axum::Json(body)).into_response())
                }
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Internal: state injection layer
// ---------------------------------------------------------------------------

/// Tower `Layer` that injects `AppState` into every request's extensions.
#[derive(Clone)]
pub(crate) struct InjectStateLayer {
    state: AppState,
}

impl InjectStateLayer {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl<S: Clone> Layer<S> for InjectStateLayer {
    type Service = InjectState<S>;

    fn layer(&self, inner: S) -> InjectState<S> {
        InjectState {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct InjectState<S> {
    inner: S,
    state: AppState,
}

impl<S> Service<Request> for InjectState<S>
where
    S: Service<Request> + Clone + Send + 'static,
    S::Response: IntoResponse + 'static,
    S::Error: Into<Infallible> + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<S::Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        req.extensions_mut().insert(self.state.clone());
        Box::pin(self.inner.call(req))
    }
}

