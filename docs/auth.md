# Authentication (JWT & session cookie)

Oxide provides HS256 JWT validation via `App::auth` and extractors for handlers.

This page reflects current behavior in:

- `oxide-framework-core/src/auth/layer.rs`
- `oxide-framework-core/src/auth/extract.rs`
- `oxide-framework-core/src/auth/config.rs`

## Setup

```rust
use oxide_framework_core::{App, AuthConfig};

App::new()
    .auth(AuthConfig::new(std::env::var("JWT_SECRET").unwrap().into_bytes()))
    .get("/api/me", my_handler)
    .run();
```

### `AuthConfig`

- `AuthConfig::new(secret)` — required non-empty HMAC secret.
- `with_session_cookie("name")` — also read the JWT from that cookie (browser sessions).
- `with_issuer` / `with_audience` — optional `iss` / `aud` validation.
- `enable_bearer(false)` — disable `Authorization: Bearer` (cookie-only).

Example with stricter validation:

```rust
use oxide_framework_core::AuthConfig;

let auth = AuthConfig::new(b"super-secret".to_vec())
    .with_issuer("https://auth.example.com")
    .with_audience("oxide-api")
    .with_session_cookie("oxide_session");
```

## Token payload

The middleware decodes a JSON payload with:

- `sub` — subject (e.g. user id)
- `exp` — Unix expiry (seconds)
- `roles` — optional JSON array of role strings

Mint tokens in a login handler with `encode_token`:

```rust
use oxide_framework_core::{encode_token, AuthClaims};

let claims = AuthClaims::new("user-42", vec!["user".into()], 3600);
let jwt = encode_token(&claims, &secret)?;
```

## Extractors

| Extractor | Behavior |
|-----------|----------|
| `OptionalAuth` | `Option<AuthClaims>` — anonymous if no token; invalid `Authorization: Bearer` → **401**. |
| `Authenticated` | Valid JWT required — **401** if missing. |
| `RequireRole<R>` | `R: RoleName` — **401** if not logged in, **403** if role missing. |

All auth extractors depend on `App::auth(...)` being enabled in your app.

### Role guard (`RoleName`)

```rust
use oxide_framework_core::{RequireRole, RoleName};

struct Admin;
impl RoleName for Admin {
    const ROLE: &'static str = "admin";
}

async fn admin_panel(_: RequireRole<Admin>) -> ApiResponse<()> {
    ApiResponse::ok(())
}
```

## Middleware order

Application state is injected, then JWT is validated, then global `before` / `App::layer` hooks, then the handler.

## Token Source and Precedence

Token resolution works in this order:

1. If bearer auth is enabled and `Authorization` is present, it must be a valid
   Bearer value (`Bearer <jwt>`). Otherwise request is rejected with **401**
   (`"malformed authorization"`).
2. If no bearer token is resolved, and `with_session_cookie(...)` is configured,
   Oxide checks the cookie header for that cookie name.
3. If no token is found, request proceeds as anonymous.

Note: if an `Authorization` header is present but malformed, Oxide rejects the
request and does not fall back to cookie extraction.

## Error Responses

JWT middleware errors are standardized JSON envelopes:

```json
{"status":401,"error":"invalid or expired token"}
```

or

```json
{"status":401,"error":"malformed authorization"}
```

Extractor rejections use:

- `Authenticated` missing auth -> `401` / `"authentication required"`
- `RequireRole<R>` role missing -> `403` / `"insufficient permissions"`

## Cookie-Only Sessions

```rust
use oxide_framework_core::{App, AuthConfig};

App::new()
    .auth(
        AuthConfig::new(std::env::var("JWT_SECRET").unwrap().into_bytes())
            .with_session_cookie("oxide_session")
            .enable_bearer(false),
    )
    .run();
```

## Dependencies

Uses `jsonwebtoken` (HS256) and `cookie` for the `Cookie` header.

