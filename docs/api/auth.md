# API: Auth

Reference for auth surface from `oxide-framework-core/src/auth/*`.

## Core Types

- `AuthConfig`
- `AuthClaims`
- `AuthLayer`
- `AuthRejection`
- `Authenticated`
- `OptionalAuth`
- `RequireRole<R>`
- `RoleName`
- `encode_token(...)`

## `AuthConfig`

- `AuthConfig::new(secret)`
- `.with_session_cookie(name)`
- `.with_issuer(issuer)`
- `.with_audience(aud)`
- `.enable_bearer(false)` for cookie-only mode

## Resolution Semantics

Token source order:

1. bearer token (when enabled)
2. configured session cookie
3. anonymous request

Malformed `Authorization` header rejects with 401 and does not fall back to cookie.

## Extractor Semantics

- `OptionalAuth` -> `Option<AuthClaims>`
- `Authenticated` -> 401 when missing/invalid
- `RequireRole<R>` -> 401 when unauthenticated, 403 when role missing
