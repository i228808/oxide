# Troubleshooting

Common runtime and setup issues, with quick fixes.

## 401: `"malformed authorization"`

Cause:

- `Authorization` header is present but not in `Bearer <token>` format.

Fix:

- Send `Authorization: Bearer <jwt>`.
- Or disable bearer and use cookie-only auth with
  `AuthConfig::enable_bearer(false)`.

## 401: `"invalid or expired token"`

Cause:

- Token signature is invalid, expired, wrong issuer/audience, or wrong secret.

Fix:

- Verify signing secret matches `AuthConfig::new(secret)`.
- Check `exp`, and `iss`/`aud` if configured.

## 401 from `Authenticated`

Cause:

- Handler requires login (`Authenticated`) but no valid claims are present.

Fix:

- Enable auth middleware with `App::auth(...)`.
- Send valid bearer/cookie token.

## 403 from `RequireRole<R>`

Cause:

- User is authenticated but missing required role string.

Fix:

- Ensure JWT `roles` includes `R::ROLE`.
- Verify role naming/normalization in token issuance path.

## 500: Missing `Data<T>`

Cause:

- Handler/controller requested `Data<T>`/`Inject<T>` but `App::state(T)` was
  not registered.

Fix:

- Register dependency in app bootstrap:

```rust
App::new().state(MyDependency::new())
```

## 500: Missing `Scoped<T>`

Cause:

- Handler requested request-scoped dependency, but no matching
  `App::scoped_state(...)` inserted `T`.

Fix:

- Add scoped factory for `T` and ensure it always inserts the expected type.

## Server fails on startup when loading config

Cause:

- Config file exists but is unreadable or invalid YAML.

Fix:

- Validate YAML syntax.
- Confirm path and file permissions.
- Remember missing file is allowed, malformed file is not.

## App binds wrong host/port

Cause:

- `OXIDE_HOST` / `OXIDE_PORT` env vars override YAML values.

Fix:

- Inspect environment in the running shell.
- Remove or correct `OXIDE_*` overrides.

## Unexpected 429 responses

Cause:

- Rate limit configured too low for current traffic pattern.

Fix:

- Increase `rate_limit(max, window_secs)` values.
- Offload to proxy/edge limiter if running multi-instance.

## Unexpected 408 responses

Cause:

- Handler exceeds timeout set by `request_timeout(secs)`.

Fix:

- Increase timeout threshold.
- Optimize slow I/O and avoid blocking work in async handlers.

## 400 with code `validation_error`

Cause:

- A `Validated<T>` extractor failed `validator` rules.

Fix:

- Check response `details` payload for field-level errors.
- Ensure your request body satisfies `#[derive(Validate)]` constraints.

## 503 on `/health/ready`

Cause:

- One or more registered readiness checks returned an error.

Fix:

- Inspect `failures` list in response body.
- Resolve failing dependency and re-check readiness.

## Missing or unexpected request ID behavior

Cause:

- Custom request id header name differs from client header.
- Response echo may be disabled via `disable_response_request_id_header()`.

Fix:

- Ensure header name matches `request_id_header(...)`.
- If needed, re-enable response echo by removing the disable call.

## CLI bench/test behavior differs by directory

Cause:

- `oxide test` / `oxide bench` detect whether you are in Oxide repo root.

Fix:

- In repo root: workspace-wide behavior.
- In app project: local crate behavior.
