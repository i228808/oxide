# Configuration

Oxide uses a layered configuration system: YAML files provide the base, and environment variables override individual values.

This page reflects current behavior in `oxide-framework-core/src/config.rs` and
`oxide-framework-core/src/app.rs`.

## Config File (YAML)

Create an `app.yaml` (or any `.yaml` file) in your project:

```yaml
host: "127.0.0.1"
port: 3000
app_name: "my-app"
```

Point the app at it:

```rust
App::new()
    .config("app.yaml")
    .run();
```

If the file doesn't exist, Oxide silently falls back to defaults — this makes the config file optional during development.

## Defaults

If no config file is provided (or it's missing fields), these defaults apply:

| Field | Default | Description |
|---|---|---|
| `host` | `127.0.0.1` | Address to bind to |
| `port` | `3000` | Port to listen on |
| `app_name` | `oxide-app` | Application name (shown in logs) |

## Environment Variable Overrides

Environment variables take highest precedence. They use the `OXIDE_` prefix:

| Variable | Overrides | Example |
|---|---|---|
| `OXIDE_HOST` | `host` | `OXIDE_HOST=0.0.0.0` |
| `OXIDE_PORT` | `port` | `OXIDE_PORT=8080` |
| `OXIDE_APP_NAME` | `app_name` | `OXIDE_APP_NAME=production-api` |

Example:

```bash
OXIDE_PORT=9000 cargo run -p oxide-framework-core --example hello
```

The server starts on port 9000 regardless of what `app.yaml` says.

## Precedence Order

From lowest to highest priority:

```
1. Built-in defaults
2. YAML config file
3. Environment variables (OXIDE_*)
```

A field set in the environment always wins, even if the YAML file specifies a different value.

## `AppConfig` Struct

The configuration is deserialized into:

```rust
pub struct AppConfig {
    pub host: String,      // default: "127.0.0.1"
    pub port: u16,         // default: 3000
    pub app_name: String,  // default: "oxide-app"
}
```

All fields support `serde` defaults, so a partial YAML file works fine:

```yaml
# Only override the port, everything else stays at defaults
port: 8080
```

## Loading Config Manually

You can load config outside of the `App` builder if needed:

```rust
use oxide_framework_core::AppConfig;

let config = AppConfig::load(Some("app.yaml"));
println!("Binding to {}:{}", config.host, config.port);
```

Pass `None` to load only defaults + env vars:

```rust
let config = AppConfig::load(None);
```

## When Config is Loaded

Config loading happens at server start (`.run()` and `.serve().await`), not at
`.config()` call time. This means:

1. `.config("app.yaml")` only stores the path
2. When the server starts, it reads the file, parses it, applies env overrides
3. The final config is used to bind the server

This design lets you set up the config path early in the builder chain without worrying about file availability at that point.

## Logging Level

Logging is controlled separately via the standard `RUST_LOG` environment variable (not part of `AppConfig`):

```bash
# Debug level for your app, info for everything else
RUST_LOG=debug cargo run

# Only show warnings
RUST_LOG=warn cargo run

# Fine-grained control
RUST_LOG=oxide_framework_core=debug,hyper=warn cargo run
```

The default level is `info`.

## Failure Modes

- Missing config file: falls back to defaults + env overrides.
- Existing but unreadable file: startup panic with read error.
- Existing but invalid YAML: startup panic with parse error.
- Invalid `OXIDE_PORT`: ignored; current `port` value is kept.

