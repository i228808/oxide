# API: App

Reference for `App` in `oxide-framework-core/src/app.rs`.

## Construction

- `App::new()` initializes defaults, empty router/state, and logging.

## Configuration and State

- `.config(path)` stores YAML path for startup load.
- `.state(value)` registers singleton state (`Data<T>` / `Inject<T>`).
- `.scoped_state(factory)` registers per-request state factory (`Scoped<T>`).

## Route Registration

- `.route(Method, path, handler)` generic registration.
- `.get/.post/.put/.delete/.patch(path, handler)` convenience methods.
- `.routes(router)` flat merge.
- `.nest(prefix, router)` nested merge.
- `.controller::<C>()` register macro/manual controller.

## Middleware and Hooks

- `.rate_limit(max, window_secs)` enables per-IP limiter.
- `.cors_permissive()` allows all origins/headers/methods.
- `.cors_origins([...])` restricts allowed origins.
- `.request_timeout(secs)` enables request timeout.
- `.disable_request_logging()` disables request logger.
- `.request_id_header(name)` sets correlation header name (default `x-request-id`).
- `.disable_response_request_id_header()` stops echoing request id in responses.
- `.auth(config)` enables JWT/cookie auth layer.
- `.before(f)` request hook.
- `.after(f)` response hook.
- `.layer(layer)` custom tower layer.

## Health and Readiness

- `.readiness_check(check)` registers a `ReadinessCheck` for readiness evaluation.
- `.disable_default_health_routes()` disables built-in:
  - `GET /health/live`
  - `GET /health/ready`

## Runtime

- `.run()` creates runtime and blocks.
- `.serve().await` runs on existing runtime.
- `.into_test_server().await` starts ephemeral test server.

## TestServer API

- `addr()` returns bound `SocketAddr`.
- `url(path)` returns full `http://...` URL for requests.
