# Middleware

Oxide includes built-in middleware for request logging, rate limiting, CORS, request timeouts, and graceful shutdown — all configurable via the `App` builder.

## Middleware Stack

When `App::run()` is called, middleware is applied in this order (outermost first):

```
Request
  → Request Logger  (captures start time)
  → Request Timeout (enforces max duration)
  → Rate Limiter    (per-IP request counting)
  → CORS            (adds cross-origin headers)
  → State Injection (adds AppState to request)
  → Router → Handler
  ← (response flows back through each layer)
```

## Built-in: Request Logger

Logs every request with method, path, status, and latency. Enabled by default.

```
INFO oxide_core::middleware: request completed method=GET path=/ status=200 latency_ms=0
INFO oxide_core::middleware: request completed method=POST path=/api/users status=201 latency_ms=1
INFO oxide_core::middleware: request completed method=GET path=/api/users/0 status=404 latency_ms=0
```

Disable it:

```rust
App::new()
    .disable_request_logging()
    .run();
```

## Built-in: Rate Limiting

Per-IP rate limiter using a fixed-window counter. Returns a standardized 429 JSON error when the limit is exceeded.

```rust
App::new()
    .rate_limit(100, 60)  // 100 requests per 60 seconds per IP
    .run();
```

When rate-limited:

```
HTTP/1.1 429 Too Many Requests

{"status":429,"error":"rate limit exceeded"}
```

Server log:

```
WARN oxide_core::rate_limit: rate limit exceeded client_ip=192.168.1.1
```

### IP Detection

The rate limiter identifies clients by IP address using these sources (in priority order):

1. `X-Forwarded-For` header (first IP in comma-separated list)
2. `X-Real-IP` header
3. Falls back to `"unknown"` (all unidentified clients share one bucket)

This works behind reverse proxies (Nginx, CloudFlare, etc.) as long as the proxy sets the forwarded headers.

### Memory Management

Expired client windows are lazily cleaned up when the tracking map exceeds 1000 entries. For high-traffic production deployments behind a load balancer, consider an external rate limiter (Redis, etc.) applied at the proxy layer.

## Built-in: CORS

Cross-Origin Resource Sharing headers. Two convenience methods:

### Permissive (Development / Public APIs)

Allows any origin, method, and header:

```rust
App::new()
    .cors_permissive()
    .run();
```

Response includes `access-control-allow-origin: *`.

### Restricted Origins

Allow only specific origins:

```rust
App::new()
    .cors_origins(["https://example.com", "https://app.example.com"])
    .run();
```

All standard methods and headers are allowed; only the origin is restricted.

## Built-in: Request Timeout

Enforces a maximum processing time per request. Returns a standardized 408 JSON error on timeout.

```rust
App::new()
    .request_timeout(30)  // 30 seconds
    .run();
```

When a handler exceeds the timeout:

```
HTTP/1.1 408 Request Timeout

{"status":408,"error":"request timeout"}
```

Server log:

```
WARN oxide_core::middleware: request timed out path=/slow timeout_ms=30000
INFO oxide_core::middleware: request completed method=GET path=/slow status=408 latency_ms=30001
```

## Graceful Shutdown

`App::run()` automatically handles graceful shutdown. When the process receives:

- **Ctrl+C** (all platforms)
- **SIGTERM** (Unix/Linux — used by Docker, Kubernetes, systemd)

The server stops accepting new connections, finishes processing in-flight requests, and shuts down cleanly:

```
INFO oxide_core::app: received Ctrl+C, shutting down…
INFO oxide_core::app: Oxide server shut down gracefully
```

No configuration needed — it's always on.

## Full Example

```rust
App::new()
    .config("app.yaml")
    .state(db_pool)
    .rate_limit(200, 60)           // 200 req/min per IP
    .cors_origins(["https://myapp.com"])
    .request_timeout(15)           // 15-second timeout
    .get("/", index)
    .nest("/api", api_routes())
    .run();
```

## Writing Custom Middleware

Oxide uses Axum's middleware system which is built on Tower. You can write custom middleware as async functions:

```rust
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

async fn require_auth(request: Request, next: Next) -> Result<Response, StatusCode> {
    match request.headers().get("authorization") {
        Some(value) if value == "Bearer secret-token" => {
            Ok(next.run(request).await)
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
```

Apply it to a specific router group:

```rust
use axum::middleware;

fn protected_routes() -> axum::Router {
    OxideRouter::new()
        .get("/secret", secret_handler)
        .into_inner()
        .layer(middleware::from_fn(require_auth))
}
```

## Summary

| Feature | Builder Method | Default | Error Response |
|---|---|---|---|
| Request Logging | `.disable_request_logging()` | On | — |
| Rate Limiting | `.rate_limit(max, window_secs)` | Off | 429 JSON |
| CORS | `.cors_permissive()` / `.cors_origins([...])` | Off | — |
| Request Timeout | `.request_timeout(secs)` | Off | 408 JSON |
| Graceful Shutdown | — | Always on | — |
