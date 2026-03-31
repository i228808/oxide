use oxide_framework_core::{controller, ApiResponse, App, AppState, Config, Data, Json, OxideRouter, Path};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct User {
    id: u64,
    name: String,
}

#[derive(Serialize)]
struct Message {
    text: String,
}

#[derive(Serialize)]
struct Stats {
    app_name: String,
    total_users_created: u64,
}

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

struct Counter {
    value: AtomicU64,
}

impl Counter {
    fn new() -> Self {
        Self {
            value: AtomicU64::new(0),
        }
    }

    fn increment(&self) -> u64 {
        self.value.fetch_add(1, Ordering::Relaxed) + 1
    }

    fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn index(Config(cfg): Config) -> ApiResponse<Message> {
    ApiResponse::ok(Message {
        text: format!("Hello from {}!", cfg.app_name),
    })
}

async fn health() -> ApiResponse<Message> {
    ApiResponse::ok(Message {
        text: "healthy".into(),
    })
}

async fn stats(Config(cfg): Config, Data(counter): Data<Counter>) -> ApiResponse<Stats> {
    ApiResponse::ok(Stats {
        app_name: cfg.app_name.clone(),
        total_users_created: counter.get(),
    })
}

async fn list_users() -> ApiResponse<Vec<User>> {
    ApiResponse::ok(vec![
        User { id: 1, name: "Alice".into() },
        User { id: 2, name: "Bob".into() },
    ])
}

async fn get_user(Path(id): Path<u64>) -> ApiResponse<User> {
    if id == 0 {
        return ApiResponse::not_found("user not found");
    }
    ApiResponse::ok(User {
        id,
        name: format!("User#{id}"),
    })
}

async fn create_user(
    Data(counter): Data<Counter>,
    Json(payload): Json<serde_json::Value>,
) -> ApiResponse<User> {
    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed");

    let id = counter.increment();

    ApiResponse::created(User {
        id,
        name: name.to_string(),
    })
}

async fn delete_user(Path(id): Path<u64>) -> ApiResponse<Message> {
    ApiResponse::ok(Message {
        text: format!("user {id} deleted"),
    })
}

/// Simulate a slow endpoint (for testing request timeout).
async fn slow() -> ApiResponse<Message> {
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    ApiResponse::ok(Message {
        text: "this should never arrive if timeout < 10s".into(),
    })
}

// ---------------------------------------------------------------------------
// Router modules (manual style)
// ---------------------------------------------------------------------------

fn user_routes() -> OxideRouter {
    OxideRouter::new()
        .get("/", list_users)
        .get("/{id}", get_user)
        .post("/", create_user)
        .delete("/{id}", delete_user)
}

// ---------------------------------------------------------------------------
// Controller-based approach (Month 2)
// ---------------------------------------------------------------------------

struct ProductController {
    counter: std::sync::Arc<Counter>,
}

#[controller("/api/products")]
impl ProductController {
    fn new(state: &AppState) -> Self {
        Self {
            counter: state.get::<Counter>().expect("Counter not registered"),
        }
    }

    #[get("/")]
    async fn list(&self) -> ApiResponse<Vec<Message>> {
        ApiResponse::ok(vec![
            Message { text: "Widget".into() },
            Message { text: "Gadget".into() },
        ])
    }

    #[get("/{id}")]
    async fn get_one(&self, Path(id): Path<u64>) -> ApiResponse<Message> {
        ApiResponse::ok(Message {
            text: format!("Product #{id}"),
        })
    }

    #[post("/")]
    async fn create(&self, Json(payload): Json<serde_json::Value>) -> ApiResponse<Message> {
        let count = self.counter.increment();
        let name = payload
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unnamed");
        ApiResponse::created(Message {
            text: format!("created {name} (total: {count})"),
        })
    }
}

// ---------------------------------------------------------------------------
// Application entry point
// ---------------------------------------------------------------------------

fn main() {
    App::new()
        .config("../app.yaml")
        .state(Counter::new())
        // -- Scalability --
        .rate_limit(100, 60)
        .cors_permissive()
        .request_timeout(5)
        // -- Routes (manual style) --
        .get("/", index)
        .get("/health", health)
        .get("/stats", stats)
        .get("/slow", slow)
        .nest("/api/users", user_routes())
        // -- Controller (macro style) --
        .controller::<ProductController>()
        .run();
}

