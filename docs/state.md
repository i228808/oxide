# State Management

Oxide provides a thread-safe shared state system that makes configuration and custom data accessible in every handler — including nested routes.

## How It Works

When `App::run()` is called, Oxide builds an `AppState` containing:

1. **`config`** — the loaded `AppConfig` (always present)
2. **User state** — any values registered via `App::state()`

This `AppState` is injected into every request via a middleware layer and extracted in handlers using `Config` and `Data<T>`.

## Accessing Configuration

Use the `Config` extractor to access `AppConfig` in any handler:

```rust
use oxide_framework_core::{ApiResponse, Config};

async fn handler(Config(cfg): Config) -> ApiResponse<String> {
    ApiResponse::ok(format!("Running on port {}", cfg.port))
}
```

`Config` wraps `Arc<AppConfig>`, so cloning is cheap.

## Registering Custom State

Use `App::state()` to register any `Send + Sync + 'static` value:

```rust
use oxide_framework_core::App;

struct DbPool { /* ... */ }
struct CacheClient { /* ... */ }

fn main() {
    let pool = DbPool { /* ... */ };
    let cache = CacheClient { /* ... */ };

    App::new()
        .state(pool)
        .state(cache)
        .get("/", handler)
        .run();
}
```

Each type can only be registered once (last write wins for the same type).

## Extracting Custom State

Use the `Data<T>` extractor to access registered state in handlers:

```rust
use oxide_framework_core::{ApiResponse, Data};
use std::sync::Arc;

async fn handler(Data(pool): Data<DbPool>) -> ApiResponse<String> {
    // pool is Arc<DbPool>
    ApiResponse::ok("connected".into())
}
```

`Data<T>` wraps `Arc<T>`, so the data is shared across all concurrent handlers without copying.

## Combining Extractors

You can use multiple extractors in a single handler:

```rust
async fn dashboard(
    Config(cfg): Config,
    Data(pool): Data<DbPool>,
    Data(cache): Data<CacheClient>,
) -> ApiResponse<DashboardInfo> {
    // Access config, database, and cache
    ApiResponse::ok(DashboardInfo { /* ... */ })
}
```

## State in Nested Routes

State is injected at the middleware level, so it works in nested routes too:

```rust
fn user_routes() -> OxideRouter {
    OxideRouter::new()
        .post("/", create_user)  // create_user can use Data<DbPool>
}

App::new()
    .state(pool)
    .nest("/api/users", user_routes())
    .run();
```

## Thread Safety

All state is wrapped in `Arc` internally:

- `Config` provides `Arc<AppConfig>`
- `Data<T>` provides `Arc<T>`

For mutable shared state, use interior mutability patterns:

```rust
use std::sync::atomic::{AtomicU64, Ordering};

struct Counter {
    value: AtomicU64,
}

impl Counter {
    fn new() -> Self {
        Self { value: AtomicU64::new(0) }
    }

    fn increment(&self) -> u64 {
        self.value.fetch_add(1, Ordering::Relaxed) + 1
    }
}

// Register it:
App::new().state(Counter::new())

// Use it (Data<Counter> gives Arc<Counter>):
async fn create(Data(counter): Data<Counter>) -> ApiResponse<Item> {
    let id = counter.increment();
    ApiResponse::created(Item { id })
}
```

For more complex mutable state, use `Arc<Mutex<T>>` or `Arc<RwLock<T>>`:

```rust
use std::sync::{Arc, RwLock};

let shared_list = Arc::new(RwLock::new(Vec::<String>::new()));
App::new().state(shared_list)
```

## `AppState` Direct Access

For advanced use cases, you can access the full `AppState` directly:

```rust
use oxide_framework_core::AppState;
use axum::extract::Extension;

async fn handler(Extension(state): Extension<AppState>) -> ApiResponse<Info> {
    let config = &state.config;
    let pool = state.get::<DbPool>();
    // ...
}
```

## Error Handling

If a handler requests `Data<T>` for a type that was never registered, Oxide returns a `500 Internal Server Error` with the message:

```
internal error: missing state (my_app::DbPool)
```

This makes it easy to spot missing `.state()` calls during development.

