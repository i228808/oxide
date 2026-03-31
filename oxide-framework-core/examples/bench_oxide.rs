use oxide_framework_core::{ApiResponse, App};
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
struct Msg {
    text: String,
}

#[derive(Serialize)]
struct User {
    id: u64,
    name: String,
}

async fn oxide_json() -> ApiResponse<Msg> {
    ApiResponse::ok(Msg { text: "hello".into() })
}

async fn oxide_path(oxide_framework_core::Path(id): oxide_framework_core::Path<u64>) -> ApiResponse<User> {
    ApiResponse::ok(User { id, name: format!("user-{id}") })
}

#[tokio::main]
async fn main() {
    // Disable logging for fair performance comparison
    App::new()
        .disable_request_logging()
        .rate_limit(10_000_000, 60)
        .cors_permissive()
        .request_timeout(30)
        .get("/json", oxide_json)
        .get("/users/{id}", oxide_path)
        .serve()
        .await;
}

