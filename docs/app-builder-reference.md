# App Builder Reference

This is the canonical reference for methods on `App` in `oxide-framework-core/src/app.rs`.

## Construction

### `App::new() -> App`

Creates an app with:

- default config (`127.0.0.1:3000`, app name `oxide-app`)
- empty router
- empty state container
- request logging enabled

Also initializes `tracing` once (idempotent).

## Configuration

### `.config(path: &str) -> Self`

Stores YAML path; file is loaded later during `.run()` or `.serve()`.

### `.state<T: Send + Sync + 'static>(value: T) -> Self`

Registers singleton app state accessible via `Data<T>` / `Inject<T>`.

## Routing

### Generic

- `.route(method: Method, path: &str, handler)`

### Convenience

- `.get(path, handler)`
- `.post(path, handler)`
- `.put(path, handler)`
- `.delete(path, handler)`
- `.patch(path, handler)`

### Composition

- `.routes(router: OxideRouter)` merge without prefix
- `.nest(prefix: &str, router: OxideRouter)` mount under prefix

### Controllers

- `.controller::<C: Controller>()`

Controller startup flow:

1. `C::from_state(&AppState)`
2. `C::register(Arc<C>)`
3. optional `C::configure_router(...)`
4. nested under `C::PREFIX`

## Built-in Middleware Toggles

- `.rate_limit(max_requests: u64, window_secs: u64)`
- `.cors_permissive()`
- `.cors_origins(origins)`
- `.request_timeout(secs: u64)`
- `.disable_request_logging()`
- `.request_id_header(name: &str)`
- `.disable_response_request_id_header()`
- `.auth(config: AuthConfig)`

## Hooks and Custom Middleware

- `.before(f)` run pre-handler hook (`Request`, `Next`) on every request
- `.after(f)` transform every response
- `.layer(layer)` register a custom Tower layer

## Health and Readiness

- `.readiness_check(check)` register a typed readiness check (`ReadinessCheck`).
- `.disable_default_health_routes()` disable built-in health endpoints.

By default Oxide mounts:

- `GET /health/live` (always live)
- `GET /health/ready` (runs registered readiness checks)

## Request-scoped Dependencies

### `.scoped_state(factory) -> Self`

Registers a per-request factory. The factory runs for every incoming request and
its produced value is inserted into request extensions.

Extract it in handlers with `Scoped<T>`.

```rust
use oxide_framework_core::{App, ApiResponse, Scoped};

#[derive(Clone)]
struct RequestId(u64);

async fn whoami(Scoped(id): Scoped<RequestId>) -> ApiResponse<String> {
    ApiResponse::ok(format!("request-id={}", id.0))
}

fn main() {
    App::new()
        .scoped_state(|_parts| async move { RequestId(1) })
        .get("/id", whoami)
        .run();
}
```

If a handler asks for `Scoped<T>` and `T` was not injected for that request,
Oxide returns HTTP 500 with a JSON error.

## Running

### `.run()`

Creates a Tokio runtime and blocks the current thread.

### `.serve().await`

Uses the current Tokio runtime (async contexts, tests, custom launchers).

### `.into_test_server().await -> TestServer`

Binds to `127.0.0.1:0` and returns a helper with:

- `addr()`
- `url(path)`

The background server task is aborted when `TestServer` is dropped.
