# Controllers

Controllers group related route handlers into a struct. The `#[controller]` macro generates all the boilerplate — you write methods, the framework wires them into the router.

## Defining a Controller

```rust
use oxide_framework_core::{controller, ApiResponse, AppState, Json, Path};

struct UserController {
    db: DbPool,
}

#[controller("/api/users")]
impl UserController {
    // Constructor: called once at startup with access to AppState
    fn new(state: &AppState) -> Self {
        Self {
            db: state.get::<DbPool>().expect("DbPool not registered").as_ref().clone(),
        }
    }

    #[get("/")]
    async fn list(&self) -> ApiResponse<Vec<User>> {
        ApiResponse::ok(self.db.find_all().await)
    }

    #[get("/{id}")]
    async fn get_one(&self, Path(id): Path<u64>) -> ApiResponse<User> {
        ApiResponse::ok(self.db.find(id).await)
    }

    #[post("/")]
    async fn create(&self, Json(body): Json<CreateUser>) -> ApiResponse<User> {
        ApiResponse::created(self.db.insert(body).await)
    }

    #[delete("/{id}")]
    async fn remove(&self, Path(id): Path<u64>) -> ApiResponse<()> {
        self.db.delete(id).await;
        ApiResponse::ok(())
    }
}
```

## Registering Controllers

```rust
fn main() {
    App::new()
        .state(DbPool::connect("postgres://..."))
        .controller::<UserController>()
        .controller::<HealthController>()
        .run();
}
```

## How It Works

The `#[controller("/prefix")]` macro generates an `impl Controller for YourStruct`:

1. **`from_state`** — calls your `fn new(state: &AppState) -> Self` constructor. If no `new` method exists, falls back to `Default::default()`.
2. **`register`** — wraps each `#[get]`/`#[post]`/`#[put]`/`#[delete]`/`#[patch]` method in a closure that captures `Arc<Self>`, then registers it on an `OxideRouter`.
3. Routes are nested under the prefix (e.g. `/api/users`).

At startup, `App::controller::<C>()` resolves the controller by:
- Constructing it via `C::from_state(&app_state)` — panics immediately if a dependency is missing (fail-fast).
- Wrapping it in `Arc<Self>` for shared ownership across request handlers.
- Merging the controller's routes into the main router.

## Constructor Injection

The `new` method receives `&AppState`, giving you access to everything registered via `App::state()`:

```rust
fn new(state: &AppState) -> Self {
    Self {
        db: state.get::<DbPool>().expect("DbPool missing").as_ref().clone(),
        cache: state.get::<Cache>().expect("Cache missing").as_ref().clone(),
    }
}
```

`state.get::<T>()` returns `Option<Arc<T>>`. Using `.expect()` ensures the app crashes at startup if a dependency isn't registered — not silently at request time.

## Supported Route Attributes

| Attribute | HTTP Method |
|---|---|
| `#[get("/path")]` | GET |
| `#[post("/path")]` | POST |
| `#[put("/path")]` | PUT |
| `#[delete("/path")]` | DELETE |
| `#[patch("/path")]` | PATCH |
| `#[head("/path")]` | HEAD |
| `#[options("/path")]` | OPTIONS |

## Method Signatures

Route methods can take `&self` plus any axum extractors:

```rust
// No extractors
#[get("/")]
async fn index(&self) -> ApiResponse<String> { ... }

// Path parameter
#[get("/{id}")]
async fn get_one(&self, Path(id): Path<u64>) -> ApiResponse<User> { ... }

// JSON body
#[post("/")]
async fn create(&self, Json(body): Json<CreateUser>) -> ApiResponse<User> { ... }

// Multiple extractors
#[put("/{id}")]
async fn update(&self, Path(id): Path<u64>, Json(body): Json<UpdateUser>) -> ApiResponse<User> { ... }
```

Static methods (without `&self`) are also supported — they're registered as plain function handlers.

## Default Controllers (No Dependencies)

If your controller has no dependencies, derive `Default` and skip the `new` method:

```rust
#[derive(Default)]
struct HealthController;

#[controller("/health")]
impl HealthController {
    #[get("/")]
    async fn check(&self) -> ApiResponse<String> {
        ApiResponse::ok("healthy".into())
    }
}
```

## Middleware Applies to Controllers

All middleware configured on the `App` (rate limiting, CORS, timeout, logging, before/after hooks) automatically applies to controller routes. No extra configuration needed.

## Controller-Level Middleware

Controllers can define middleware that applies only to their own routes:

```rust
#[derive(Default)]
struct AdminController;

#[controller("/api/admin")]
impl AdminController {
    fn middleware(router: axum::Router) -> axum::Router {
        router.layer(axum::middleware::from_fn(require_admin_role))
    }

    #[get("/dashboard")]
    async fn dashboard(&self) -> ApiResponse<Dashboard> { /* ... */ }
}
```

The `middleware` method receives the controller's `Router` (already containing all routes) and returns it with additional layers applied. This middleware does NOT leak to other controllers.

