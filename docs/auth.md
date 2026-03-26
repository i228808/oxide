# Authentication (JWT & session cookie)

Oxide provides HS256 JWT validation via `App::auth` and extractors for handlers.

## Setup

```rust
use oxide_core::{App, AuthConfig};

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

## Token payload

The middleware decodes a JSON payload with:

- `sub` — subject (e.g. user id)
- `exp` — Unix expiry (seconds)
- `roles` — optional JSON array of role strings

Mint tokens in a login handler with `encode_token`:

```rust
use oxide_core::{encode_token, AuthClaims};

let claims = AuthClaims::new("user-42", vec!["user".into()], 3600);
let jwt = encode_token(&claims, &secret)?;
```

## Extractors

| Extractor | Behavior |
|-----------|----------|
| `OptionalAuth` | `Option<AuthClaims>` — anonymous if no token; invalid `Authorization: Bearer` → **401**. |
| `Authenticated` | Valid JWT required — **401** if missing. |
| `RequireRole<R>` | `R: RoleName` — **401** if not logged in, **403** if role missing. |

### Role guard (`RoleName`)

```rust
use oxide_core::{RequireRole, RoleName};

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

## Dependencies

Uses `jsonwebtoken` (HS256) and `cookie` for the `Cookie` header.
