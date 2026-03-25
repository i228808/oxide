# Architecture

## Workspace Layout

Oxide is a Cargo workspace with three crates:

```
Oxide/
├── Cargo.toml          # Workspace definition
├── oxide_core/         # Framework library (the main crate)
├── oxide_macros/       # Proc-macro crate (placeholder — Month 2)
└── oxide_cli/          # CLI tooling (placeholder — future)
```

Only `oxide_core` contains active code. The other crates are scaffolded and will be developed in later phases.

## Module Map (`oxide_core`)

```
oxide_core/src/
├── lib.rs          Public API surface — re-exports everything users need
├── app.rs          App struct, builder pattern, server lifecycle
├── router.rs       OxideRouter, Method enum, route registration
├── response.rs     ApiResponse<T>, SuccessBody, ErrorBody, IntoResponse impl
├── config.rs       AppConfig, YAML + env loader
├── state.rs        AppState, TypeMap (thread-safe shared state container)
├── extract.rs      Config, Data<T> extractors for handlers
├── middleware.rs    Request logger + state injection tower Layer
└── logging.rs      tracing-subscriber initialization
```

### `lib.rs` — Public Exports

The public API is deliberately flat. Users import from `oxide_core` directly:

```rust
pub use app::App;
pub use config::AppConfig;
pub use extract::{Config, Data};
pub use response::ApiResponse;
pub use router::{Method, OxideRouter};
pub use state::AppState;

pub use axum::extract::Path;
pub use axum::http::StatusCode;
pub use axum::Json;
```

### `app.rs` — Application Builder

The `App` struct is the framework's entry point. It follows the **builder pattern**:

```
App::new()                     → init logging, default config, empty router
    .config(path)              → store YAML path (loaded lazily at .run())
    .state(value)              → register shared state (accessible via Data<T>)
    .get/post/put/delete/...   → register routes on internal OxideRouter
    .nest(prefix, router)      → mount a sub-router under a prefix
    .routes(router)            → flat-merge a standalone router
    .disable_request_logging() → opt out of per-request logging
    .run()                     → load config, build state, apply middleware, serve
```

`run()` creates its own `tokio::runtime::Runtime` and blocks on it. This lets the user write a synchronous `fn main()` without needing `#[tokio::main]`.

### `router.rs` — Routing Abstraction

`OxideRouter<S>` wraps `axum::Router<S>` and provides:

- **Method-based registration** via `.route(Method, path, handler)` and convenience methods (`.get()`, `.post()`, etc.)
- **Composition** via `.merge()` (flat) and `.nest()` (prefixed)
- **Decoupling** — user code never touches `axum::Router` directly

The `Method` enum maps 1:1 to Axum's routing functions (`get()`, `post()`, `put()`, `delete()`, `patch()`, `head()`, `options()`).

### `response.rs` — Standardized Responses

`ApiResponse<T>` is an enum with two variants:

```
ApiResponse::Success(StatusCode, T)  → {"status": N, "data": ...}
ApiResponse::Error(StatusCode, String) → {"status": N, "error": "..."}
```

It implements `IntoResponse`, so Axum accepts it as a handler return type. The envelope ensures every endpoint returns predictable JSON.

### `config.rs` — Configuration Loading

`AppConfig` supports a three-tier precedence model:

```
Defaults → YAML file → Environment variables (OXIDE_*)
```

Loading is intentionally synchronous (called once at startup inside `run()`).

### `state.rs` — Shared State

`AppState` holds:

- `config: Arc<AppConfig>` — always present
- `extensions: Arc<TypeMap>` — user-provided values registered via `App::state()`

`TypeMap` is a type-safe `HashMap<TypeId, Arc<dyn Any>>` that allows storing and retrieving values by their concrete type.

### `extract.rs` — Handler Extractors

Custom axum extractors that pull data from request extensions:

- `Config` — extracts `Arc<AppConfig>` from `AppState`
- `Data<T>` — extracts `Arc<T>` from `AppState`'s TypeMap

Both implement `FromRequestParts<S>` for any `S`, so they work regardless of the router's state type.

### `middleware.rs` — Middleware

Contains two components:

1. **`request_logger`** (public) — async function used with `axum::middleware::from_fn`. Logs method, path, status, and latency for every request.

2. **`InjectStateLayer`** (internal) — custom Tower `Layer` / `Service` pair that inserts `AppState` into every request's extensions. This enables the `Config` and `Data<T>` extractors.

### `logging.rs` — Tracing Setup

Initializes `tracing-subscriber` with:

- An `EnvFilter` that reads `RUST_LOG` (defaults to `info`)
- Formatted output with module targets enabled

Called once from `App::new()`.

## Data Flow

```
                    ┌──────────────────────────┐
                    │       App::new()          │
                    │  • init logging           │
                    │  • default config         │
                    │  • empty router           │
                    │  • empty type map         │
                    └────────────┬─────────────┘
                                 │
               .config() / .state() / .get() / .nest() / ...
                                 │
                    ┌────────────▼─────────────┐
                    │       App::run()          │
                    │  • load YAML + env vars   │
                    │  • build AppState (Arc)   │
                    │  • build axum::Router     │
                    │  • apply InjectStateLayer │
                    │  • apply request_logger   │
                    │  • create tokio Runtime   │
                    │  • bind TcpListener       │
                    │  • axum::serve()          │
                    └────────────┬─────────────┘
                                 │
                    ┌────────────▼─────────────┐
                    │     Request Handling      │
                    │                           │
                    │  Client Request           │
                    │    → request_logger       │
                    │    → InjectState (adds    │
                    │      AppState to exts)    │
                    │    → axum Router          │
                    │    → matched Handler      │
                    │      (extracts Config,    │
                    │       Data<T>, Path, etc) │
                    │    → ApiResponse<T>       │
                    │    → JSON envelope        │
                    │    ← request_logger       │
                    │      (logs result)        │
                    │    → HTTP Response        │
                    └──────────────────────────┘
```

## Dependency Graph

```
oxide_core
├── axum 0.8           ← HTTP routing + server
│   ├── tokio 1        ← async runtime
│   └── hyper 1        ← HTTP implementation
├── tower 0.5          ← middleware Layer/Service traits
├── serde 1            ← (de)serialization traits
├── serde_json 1       ← JSON encoding/decoding
├── serde_yaml 0.9     ← YAML config parsing
├── tracing 0.1        ← instrumentation API
└── tracing-subscriber 0.3  ← log output formatting
```

## Design Principles

### Convention Over Configuration

Sensible defaults everywhere. A one-line `App::new().run()` gives you a working server with logging, config defaults, and JSON responses.

### Minimal Boilerplate

The framework handles runtime setup, config loading, state injection, middleware, and response formatting. The developer writes handlers and wires routes.

### Composition Over Inheritance

Route groups are composed via `OxideRouter::nest()` and `merge()`. State is composed via `App::state()`. No trait hierarchies or controller base classes.

### Compile-Time Safety

No runtime reflection, no string-based dispatch. Route handlers are type-checked by the Rust compiler through Axum's `Handler` trait. State extraction is type-safe via generics.

### Thin Abstractions

Every Oxide type is a thin wrapper over a battle-tested crate (Axum, Tokio, Tower, Serde). The framework adds ergonomics, not runtime overhead.

## Out of Scope (Current Phase)

These are planned for future phases:

| Feature | Target Phase |
|---|---|
| Dependency injection | Month 2 |
| Procedural macros (`#[get("/")]`) | Month 2 |
| CLI scaffolding tool | Month 2+ |
| ORM / database layer | Month 3+ |
