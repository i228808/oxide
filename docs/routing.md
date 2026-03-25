# Routing

Oxide wraps Axum's router behind a simplified, chainable API. You can register routes directly on `App` or build modular route groups with `OxideRouter`.

## HTTP Methods

The `Method` enum covers all standard HTTP methods:

```rust
use oxide_core::Method;

// Available variants:
// Method::GET
// Method::POST
// Method::PUT
// Method::DELETE
// Method::PATCH
// Method::HEAD
// Method::OPTIONS
```

## Registering Routes

### Generic `.route()` Method

```rust
use oxide_core::{App, Method};

App::new()
    .route(Method::GET, "/", index)
    .route(Method::POST, "/submit", submit)
    .run();
```

### Convenience Methods

For common HTTP methods, use the shorthand:

```rust
App::new()
    .get("/", index)
    .post("/items", create_item)
    .put("/items/{id}", update_item)
    .delete("/items/{id}", delete_item)
    .patch("/items/{id}", patch_item)
    .run();
```

These are available on both `App` and `OxideRouter`.

## Path Parameters

Use `{param}` syntax in the path and extract with `Path`:

```rust
use oxide_core::{ApiResponse, Path};

async fn get_user(Path(id): Path<u64>) -> ApiResponse<User> {
    ApiResponse::ok(User { id, name: format!("User#{id}") })
}

// Register:
app.get("/users/{id}", get_user)
```

Multiple path parameters use a tuple:

```rust
async fn get_comment(
    Path((post_id, comment_id)): Path<(u64, u64)>,
) -> ApiResponse<Comment> {
    // ...
}

app.get("/posts/{post_id}/comments/{comment_id}", get_comment)
```

## Modular Routers with `OxideRouter`

For larger applications, define route groups as standalone functions that return an `OxideRouter`:

```rust
use oxide_core::OxideRouter;

fn user_routes() -> OxideRouter {
    OxideRouter::new()
        .get("/", list_users)
        .get("/{id}", get_user)
        .post("/", create_user)
        .delete("/{id}", delete_user)
}

fn product_routes() -> OxideRouter {
    OxideRouter::new()
        .get("/", list_products)
        .get("/{id}", get_product)
}
```

## Nesting (Prefix-Based)

Mount a router under a path prefix with `.nest()`:

```rust
App::new()
    .nest("/api/users", user_routes())
    .nest("/api/products", product_routes())
    .run();
```

This produces:

| Handler | Final Path |
|---|---|
| `list_users` | `GET /api/users/` |
| `get_user` | `GET /api/users/{id}` |
| `create_user` | `POST /api/users/` |
| `delete_user` | `DELETE /api/users/{id}` |
| `list_products` | `GET /api/products/` |
| `get_product` | `GET /api/products/{id}` |

Nesting works on `OxideRouter` too, so you can compose multi-level trees:

```rust
fn api_v1() -> OxideRouter {
    OxideRouter::new()
        .nest("/users", user_routes())
        .nest("/products", product_routes())
}

App::new()
    .nest("/api/v1", api_v1())
    .run();
```

## Merging (Flat)

If your sub-router already has full paths, use `.routes()` to merge without a prefix:

```rust
fn health_routes() -> OxideRouter {
    OxideRouter::new()
        .get("/health", health_check)
        .get("/ready", readiness_check)
}

App::new()
    .routes(health_routes())
    .nest("/api", api_routes())
    .run();
```

`.merge()` is also available on `OxideRouter` for composing routers together before attaching them to the app.

## Handler Signatures

Handlers are async functions. Oxide uses Axum's handler system, so any function that implements `Handler<T, ()>` works. Common patterns:

```rust
// No input, plain response
async fn index() -> ApiResponse<Message> { ... }

// Path parameter
async fn get_item(Path(id): Path<u64>) -> ApiResponse<Item> { ... }

// JSON body
async fn create_item(Json(body): Json<CreateItemRequest>) -> ApiResponse<Item> { ... }

// Multiple extractors
async fn update_item(
    Path(id): Path<u64>,
    Json(body): Json<UpdateItemRequest>,
) -> ApiResponse<Item> { ... }
```

Handlers can also return plain types like `&'static str`, `String`, or `axum::response::Response` — though `ApiResponse<T>` is recommended for consistency.

## Summary

| Method | `App` | `OxideRouter` | Description |
|---|---|---|---|
| `.route(method, path, handler)` | Yes | Yes | Generic registration |
| `.get(path, handler)` | Yes | Yes | `GET` shorthand |
| `.post(path, handler)` | Yes | Yes | `POST` shorthand |
| `.put(path, handler)` | Yes | Yes | `PUT` shorthand |
| `.delete(path, handler)` | Yes | Yes | `DELETE` shorthand |
| `.patch(path, handler)` | Yes | Yes | `PATCH` shorthand |
| `.nest(prefix, router)` | Yes | Yes | Mount sub-router under prefix |
| `.routes(router)` | Yes | — | Flat merge (App only) |
| `.merge(router)` | — | Yes | Flat merge (OxideRouter only) |
