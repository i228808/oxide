use oxide_core::{ApiResponse, App, Config, Data, Json, OxideRouter, Path};
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
// Router modules
// ---------------------------------------------------------------------------

fn user_routes() -> OxideRouter {
    OxideRouter::new()
        .get("/", list_users)
        .get("/{id}", get_user)
        .post("/", create_user)
        .delete("/{id}", delete_user)
}

// ---------------------------------------------------------------------------
// Application entry point
// ---------------------------------------------------------------------------

fn main() {
    App::new()
        .config("../app.yaml")
        .state(Counter::new())
        // -- Scalability --
        .rate_limit(100, 60)       // 100 requests per 60 seconds per IP
        .cors_permissive()         // allow any origin (development)
        .request_timeout(5)        // 5-second timeout
        // -- Routes --
        .get("/", index)
        .get("/health", health)
        .get("/stats", stats)
        .get("/slow", slow)
        .nest("/api/users", user_routes())
        .run();
}
