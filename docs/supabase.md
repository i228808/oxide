# Supabase Integration

Oxide supports Supabase PostgREST/RPC through `oxide-framework-supabase`.

## Install

```toml
[dependencies]
oxide_framework_core = { path = "../oxide-framework-core" }
oxide_framework_supabase = { path = "../oxide-framework-supabase" }
```

## Register in App

```rust
use oxide_framework_core::App;
use oxide_framework_supabase::{AppSupabaseExt, SupabaseConfig};

let cfg = SupabaseConfig::new("https://project.supabase.co", "service-role-key")
    .with_schema("public")
    .with_timeout(10)
    .strict(true);

App::new().supabase(cfg);
```

Strict mode adds a readiness check used by `/health/ready`.

## Client Capabilities

Injected `SupabaseClient` supports:

- `select(table, query)`
- `insert(table, payload)`
- `rpc(function, payload)`

Use `Data<SupabaseClient>` in handlers.
