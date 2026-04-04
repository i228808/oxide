# Database (SQLx Integration)

Oxide ships a separate crate, `oxide-framework-db`, that adds SQLx pool
registration to the `App` builder.

This page reflects current behavior in `oxide-framework-db/src/lib.rs`.

## Install

Add to your app:

```toml
[dependencies]
oxide_framework_core = { path = "../oxide-framework-core" }
oxide_framework_db = { path = "../oxide-framework-db" }
```

## Core API

- `AppDbExt::database::<D>(url, opts)`
- `DbPool<D>` type alias (`sqlx::Pool<D>`)
- Re-exports: `sqlx`, `Sqlite`, `Postgres`, `MySql`, `Database`

The extension method registers the pool through `App::state(...)`, so handlers
can extract it with `Data<DbPool<D>>`.

## SQLite Example

```rust
use oxide_framework_core::{App, ApiResponse, Data};
use oxide_framework_db::{AppDbExt, DbPool, Sqlite};

async fn ping_db(Data(pool): Data<DbPool<Sqlite>>) -> ApiResponse<String> {
    let row: (String,) = sqlx::query_as("SELECT 'ok'").fetch_one(&*pool).await.unwrap();
    ApiResponse::ok(row.0)
}

fn main() {
    App::new()
        .database::<Sqlite>("sqlite::memory:", |opts| opts.max_connections(5))
        .get("/db", ping_db)
        .run();
}
```

## Postgres Example

```rust
use oxide_framework_core::{App, ApiResponse, Data};
use oxide_framework_db::{AppDbExt, DbPool, Postgres};

async fn health(Data(pool): Data<DbPool<Postgres>>) -> ApiResponse<&'static str> {
    let _: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&*pool).await.unwrap();
    ApiResponse::ok("ok")
}

fn main() {
    App::new()
        .database::<Postgres>(
            "postgres://postgres:postgres@localhost:5432/app",
            |opts| opts.max_connections(20),
        )
        .get("/health", health)
        .run();
}
```

## MySQL Example

```rust
use oxide_framework_core::{App, ApiResponse, Data};
use oxide_framework_db::{AppDbExt, DbPool, MySql};

async fn health(Data(pool): Data<DbPool<MySql>>) -> ApiResponse<&'static str> {
    let _: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&*pool).await.unwrap();
    ApiResponse::ok("ok")
}

fn main() {
    App::new()
        .database::<MySql>(
            "mysql://root:password@localhost:3306/app",
            |opts| opts.max_connections(20),
        )
        .get("/health", health)
        .run();
}
```

## Behavior Notes

- Pool creation uses `connect_lazy(...)`.
- That means pool construction is synchronous during app build.
- Connectivity is validated on first query, not necessarily at app startup.
- Invalid URLs can still fail immediately while constructing the lazy pool.

If you want hard fail-fast connectivity checks, issue a startup query before
serving traffic.

## Testing

The workspace includes integration coverage in
`oxide-framework-db/tests/sqlite_integration_test.rs` for:

- pool injection into handlers
- query execution through extracted pool
- concurrency behavior with constrained pool size
