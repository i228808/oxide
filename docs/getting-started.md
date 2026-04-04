# Getting Started

## Prerequisites

- **Rust 1.85+** (edition 2024 workspace)
- **Cargo** (comes with Rust)

Verify your toolchain:

```bash
rustc --version
cargo --version
```

## Adding Oxide to Your Project

Oxide is structured as a Cargo workspace. The framework library is `oxide_framework_core`.

If you're building within the Oxide workspace, add it as a path dependency:

```toml
[dependencies]
oxide_framework_core = { path = "../oxide-framework-core" }
serde = { version = "1", features = ["derive"] }
```

The `serde` dependency is needed for your own types that you want to serialize in responses.

## Your First Application

Create a file `src/main.rs`:

```rust
use oxide_framework_core::{App, ApiResponse};
use serde::Serialize;

#[derive(Serialize)]
struct Message {
    text: String,
}

async fn hello() -> ApiResponse<Message> {
    ApiResponse::ok(Message {
        text: "Hello, world!".into(),
    })
}

fn main() {
    App::new()
        .get("/", hello)
        .run();
}
```

Run it:

```bash
cargo run
```

You should see:

```
2026-03-26T00:00:00.000000Z  INFO oxide_framework_core::app: Oxide server started name=oxide-app address=127.0.0.1:3000
```

Visit `http://127.0.0.1:3000/` and you'll get:

```json
{"status": 200, "data": {"text": "Hello, world!"}}
```

## Adding Configuration

Create an `app.yaml` in your project root:

```yaml
host: "127.0.0.1"
port: 8080
app_name: "my-app"
```

Point the app at it:

```rust
fn main() {
    App::new()
        .config("app.yaml")
        .get("/", hello)
        .run();
}
```

The server now starts on port 8080. See [Configuration](configuration.md) for the full config reference.

## Adding More Routes

```rust
use oxide_framework_core::{App, ApiResponse, Path};

async fn get_item(Path(id): Path<u64>) -> ApiResponse<String> {
    ApiResponse::ok(format!("Item #{id}"))
}

fn main() {
    App::new()
        .get("/", hello)
        .get("/items/{id}", get_item)
        .post("/items", create_item)
        .run();
}
```

See [Routing](routing.md) for nesting, merging, and all HTTP methods.

## Returning Errors

```rust
async fn find_user(Path(id): Path<u64>) -> ApiResponse<User> {
    if id == 0 {
        return ApiResponse::not_found("user does not exist");
    }
    ApiResponse::ok(User { id, name: "Alice".into() })
}
```

This returns HTTP 404 with:

```json
{"status": 404, "error": "user does not exist"}
```

See [Responses](responses.md) for the full response API.

## What `App::new()` Does Automatically

1. **Initializes structured logging** via `tracing-subscriber` with an `info` default level (override with the `RUST_LOG` environment variable)
2. **Creates a default `AppConfig`** — host `127.0.0.1`, port `3000`, app name `oxide-app`
3. **Creates an empty router** ready for route registration

## What `App::run()` Does

1. Loads configuration from the YAML file (if `.config()` was called) and overlays environment variables
2. Builds the final `axum::Router` from all registered routes
3. Creates a Tokio runtime
4. Binds a TCP listener to the configured address
5. Logs the startup message
6. Serves requests until the process is terminated

## Next Steps

- [API Reference](api-reference.md) — Complete exported API map
- [App Builder Reference](app-builder-reference.md) — All `App` builder/runtime methods
- [Routing](routing.md) — Organize routes with nesting and modular routers
- [Responses](responses.md) — Understand the standardized JSON envelope
- [Configuration](configuration.md) — YAML, env vars, and precedence rules
- [Architecture](architecture.md) — How the internals fit together

