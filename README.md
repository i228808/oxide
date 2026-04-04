# Oxide

A **Rust-native, opinionated web framework** that delivers a Spring Boot-like developer experience — without runtime reflection.

Built on top of [Axum](https://github.com/tokio-rs/axum) and [Tokio](https://tokio.rs), Oxide provides convention-first project structure, a clean chainable API, and standardized patterns for routing, configuration, middleware, and shared state.

## Why Not Just Use Axum?

Axum is an incredible, high-performance routing library, but it is **not a full framework**. 

When building production services with raw Axum, you repeatedly wire together the same boilerplate: configuring tracing subscribers, stacking Tower middleware correctly (CORS, timeouts, panic recovery, rate limiting), creating standardized JSON error envelopes, and managing dependency injection lifecycles.

**Oxide removes that boilerplate and enforces correct conventions while keeping Axum-level performance.**

### The Killer Feature: Zero-Config Production APIs

Oxide ships with secure, production-tested defaults out of the box. You get automatic request logging, global panic recovery, standardized JSON success/error envelopes, and deterministic middleware ordering—all without writing a single line of configuration.

### Axum vs. Oxide: The Boilerplate Difference

**Raw Axum (30+ lines of exact middleware ordering & setup):**
```rust
// A typical production Axum setup
let app = Router::new()
    .route("/api/users", get(list_users))
    // Middleware order is critical and easy to get wrong!
    .layer(CatchPanicLayer::new()) 
    .layer(CorsLayer::permissive())
    .layer(TimeoutLayer::new(Duration::from_secs(30)))
    .layer(TraceLayer::new_for_http()) 
    .with_state(db_pool);

let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
axum::serve(listener, app).await.unwrap();
```

**Oxide (5 lines, zero configuration for the same production readiness):**
```rust
// Everything above is handled automatically and correctly.
App::new()
    .controller::<UserController>()
    .run();
```

## Quickstart — Controller Style

```rust
use oxide_framework_core::{controller, App, AppState, ApiResponse, Json, Path};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct User { id: u64, name: String }

#[derive(Deserialize)]
struct CreateUser { name: String }

struct UserController;

#[controller("/api/users")]
impl UserController {
    #[get("/")]
    async fn list(&self) -> ApiResponse<Vec<User>> {
        ApiResponse::ok(vec![User { id: 1, name: "Alice".into() }])
    }

    #[get("/{id}")]
    async fn get_one(&self, Path(id): Path<u64>) -> ApiResponse<User> {
        ApiResponse::ok(User { id, name: format!("User#{id}") })
    }

    #[post("/")]
    async fn create(&self, Json(body): Json<CreateUser>) -> ApiResponse<User> {
        ApiResponse::created(User { id: 42, name: body.name })
    }
}

fn main() {
    App::new()
        .config("app.yaml")
        .rate_limit(100, 60)
        .cors_permissive()
        .request_timeout(30)
        .controller::<UserController>()
        .run();
}
```

## Quickstart — Functional Style

```rust
use oxide_framework_core::{App, ApiResponse, Config};
use serde::Serialize;

#[derive(Serialize)]
struct Message { text: String }

async fn index(Config(cfg): Config) -> ApiResponse<Message> {
    ApiResponse::ok(Message { text: format!("Hello from {}!", cfg.app_name) })
}

fn main() {
    App::new()
        .config("app.yaml")
        .rate_limit(100, 60)
        .cors_permissive()
        .request_timeout(30)
        .get("/", index)
        .run();
}
```

That gives you:

- A running HTTP server on `127.0.0.1:3000`
- Configuration loaded from YAML + environment variables
- Shared state accessible in handlers via extractors
- Per-request logging (method, path, status, latency)
- Per-IP rate limiting (429 JSON on exceeded)
- CORS headers for cross-origin requests
- Request timeout enforcement (408 JSON on timeout)
- Graceful shutdown on Ctrl+C / SIGTERM
- Standardized JSON response envelopes

## CLI (`oxide`)

Scaffold apps and generate controllers without boilerplate:

```bash
cargo install --path oxide-framework-cli   # installs `oxide` on PATH
oxide new my-api --oxide path=../oxide-framework-core
cd my-api && cargo run

oxide generate controller Product --prefix /api/products
oxide generate route ProductController GET /featured
oxide run -- --release
```

From the Oxide repo, `oxide test` runs the full workspace test suite; `oxide bench` runs Criterion benchmarks plus the load-test example. See [docs/cli.md](docs/cli.md) for all commands and flags.

## Project Structure

```
Oxide/
├── Cargo.toml                 # Workspace root
├── app.yaml                   # Application config
│
├── oxide-framework-core/                # Framework library
│   ├── src/
│   │   ├── lib.rs             # Public API exports
│   │   ├── app.rs             # App builder + server lifecycle
│   │   ├── router.rs          # OxideRouter, Method enum
│   │   ├── response.rs        # ApiResponse, JSON envelopes
│   │   ├── config.rs          # AppConfig, YAML + env loading
│   │   ├── state.rs           # AppState, TypeMap (shared state)
│   │   ├── extract.rs         # Config, Data<T> extractors
│   │   ├── middleware.rs       # Request logger, state injection layer
│   │   └── logging.rs         # tracing-subscriber init
│   └── examples/
│       └── hello.rs           # Full working example
│
├── oxide-framework-macros/              # Proc-macro crate (#[controller], route attrs)
├── oxide-framework-db/                  # SQLx integration crate
└── oxide-framework-cli/                 # `oxide` CLI — scaffold, generate, run, test, bench
```

## Public API at a Glance

| Export | Description |
|---|---|
| `App` | Builder for creating and running an Oxide application |
| `#[controller]` | Proc-macro: turns an `impl` block into a routable controller |
| `Controller` | Trait generated by `#[controller]` (also usable manually) |
| `OxideRouter` | Standalone router for modular route groups |
| `Method` | Enum: `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD`, `OPTIONS` |
| `ApiResponse<T>` | Standardized JSON response with success/error envelopes |
| `AppConfig` | Configuration struct (YAML + env vars) |
| `AppState` | Shared state container (config + user extensions) |
| `FrameworkError` | Typed framework/core error mapped to JSON envelopes |
| `Config` | Extractor for `AppConfig` in handlers |
| `Data<T>` | Extractor for user-provided state in handlers |
| `Inject<T>` | Alias for `Data<T>`, reads naturally in controller methods |
| `RequestId` | Extractor for correlation/request id |
| `Validated<T>` | JSON + validator-backed request extractor |
| `AuthConfig` / `App::auth` | HS256 JWT from `Authorization: Bearer` and/or a session cookie |
| `AuthClaims` | Decoded JWT subject + roles (in request extensions) |
| `Authenticated`, `OptionalAuth` | Extractors for logged-in / optional identity |
| `RequireRole<R>`, `RoleName` | Role guard (403 when role missing) |
| `encode_token` | Mint a JWT for login handlers / tests |
| `Json` | Re-export of `axum::Json` for request/response bodies |
| `Path` | Re-export of `axum::extract::Path` for path parameters |
| `StatusCode` | Re-export of `axum::http::StatusCode` |

## Documentation

- [API Reference](docs/api-reference.md) — Canonical public API map from `lib.rs`
- [App Builder Reference](docs/app-builder-reference.md) — Full `App` methods and runtime behavior
- [CLI Reference](docs/cli-reference.md) — Canonical command reference from CLI source
- [Getting Started](docs/getting-started.md) — Setup, first app, running the server
- [Routing](docs/routing.md) — Methods, nesting, merging, path parameters
- [Responses](docs/responses.md) — ApiResponse, JSON envelopes, error handling
- [Configuration](docs/configuration.md) — YAML files, environment variables, defaults
- [State Management](docs/state.md) — Shared state, Config/Data extractors, thread safety
- [Database](docs/db.md) — SQLx pool injection with `oxide-framework-db`
- [Supabase](docs/supabase.md) — PostgREST/RPC integration and readiness checks
- [MongoDB](docs/mongodb.md) — Mongo client injection and readiness checks
- [Middleware](docs/middleware.md) — Request logging, middleware architecture, custom middleware
- [Authentication](docs/auth.md) — JWT, session cookies, role guards
- [Controllers](docs/controllers.md) — `#[controller]` macro behavior and controller middleware
- [Architecture](docs/architecture.md) — Crate layout, data flow, design principles
- [CLI](docs/cli.md) — `oxide new`, `generate`, `run`, `test`, `bench`
- [Versioning](docs/versioning.md) — SemVer policy and public API scope
- [Upgrade Notes](docs/upgrade-notes.md) — Migration notes between releases
- [Roadmap and Status](docs/roadmap.md) — Stable vs evolving vs planned areas
- [Troubleshooting](docs/troubleshooting.md) — Common 401/403/500 and setup fixes
- [Docs Drift Checklist](docs/docs-drift-checklist.md) — Keep docs aligned with code changes

Split API pages:

- [API: App](docs/api/app.md)
- [API: Extractors](docs/api/extractors.md)
- [API: Controllers](docs/api/controllers.md)
- [API: Auth](docs/api/auth.md)

## Dependencies

| Crate | Purpose |
|---|---|
| `axum` 0.8 | HTTP routing and handler framework |
| `tokio` 1 | Async runtime |
| `tower` 0.5 | Middleware layer/service abstractions |
| `tower-http` 0.6 | CORS, panic recovery |
| `syn` 2 / `quote` 1 | Proc-macro parsing and code generation |
| `serde` / `serde_json` / `serde_yaml` | Serialization and config parsing |
| `tracing` / `tracing-subscriber` | Structured logging |

## Running the Example

```bash
cd oxide-framework-core
cargo run --example hello
```

Then:

```bash
curl http://127.0.0.1:3000/
# {"status":200,"data":{"text":"Hello from my-oxide-app!"}}

curl http://127.0.0.1:3000/stats
# {"status":200,"data":{"app_name":"my-oxide-app","total_users_created":0}}

curl http://127.0.0.1:3000/api/users
# {"status":200,"data":[{"id":1,"name":"Alice"},{"id":2,"name":"Bob"}]}

curl -X POST http://127.0.0.1:3000/api/users -H "Content-Type: application/json" -d '{"name":"Charlie"}'
# {"status":201,"data":{"id":1,"name":"Charlie"}}

curl http://127.0.0.1:3000/stats
# {"status":200,"data":{"app_name":"my-oxide-app","total_users_created":1}}
```

Server logs:

```
INFO oxide_framework_core::app: Oxide server started name=my-oxide-app address=127.0.0.1:3000
INFO oxide_framework_core::middleware: request completed method=GET path=/ status=200 latency_ms=0
INFO oxide_framework_core::middleware: request completed method=POST path=/api/users status=201 latency_ms=0
INFO oxide_framework_core::middleware: request completed method=GET path=/api/users/0 status=404 latency_ms=0
```

## License

MIT

