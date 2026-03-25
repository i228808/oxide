//! Oxide framework server for load testing comparison.
//!
//! Start:  `cargo run -p oxide_core --release --example bench_oxide`
//! Test:   `wrk -t4 -c100 -d30s http://127.0.0.1:3002/json`
//!
//! Includes the full middleware stack: state injection, panic recovery,
//! rate limiting (1M/min — effectively unlimited for benchmarks), CORS,
//! and request timeout.

use oxide_core::{controller, ApiResponse, App};
use serde::Serialize;

#[derive(Serialize)]
struct Msg {
    text: String,
}

#[derive(Serialize)]
struct User {
    id: u64,
    name: String,
}

// -- Functional handlers --

async fn json_handler() -> ApiResponse<Msg> {
    ApiResponse::ok(Msg { text: "hello".into() })
}

async fn path_handler(oxide_core::Path(id): oxide_core::Path<u64>) -> ApiResponse<User> {
    ApiResponse::ok(User { id, name: format!("user-{id}") })
}

async fn post_handler(
    oxide_core::Json(body): oxide_core::Json<serde_json::Value>,
) -> ApiResponse<serde_json::Value> {
    ApiResponse::created(body)
}

// -- Controller-based handler --

#[derive(Default)]
struct ApiController;

#[controller("/api")]
impl ApiController {
    #[get("/json")]
    async fn ctrl_json(&self) -> ApiResponse<Msg> {
        ApiResponse::ok(Msg { text: "hello".into() })
    }

    #[get("/users/{id}")]
    async fn ctrl_user(&self, oxide_core::Path(id): oxide_core::Path<u64>) -> ApiResponse<User> {
        ApiResponse::ok(User { id, name: format!("user-{id}") })
    }
}

fn main() {
    println!("Oxide server listening on http://127.0.0.1:3002");
    println!("Endpoints:");
    println!("  GET  /json          (functional)");
    println!("  GET  /users/{{id}}    (functional)");
    println!("  POST /create        (functional)");
    println!("  GET  /api/json      (controller)");
    println!("  GET  /api/users/{{id}} (controller)");

    App::new()
        .rate_limit(1_000_000, 60)
        .cors_permissive()
        .request_timeout(30)
        .get("/json", json_handler)
        .get("/users/{id}", path_handler)
        .post("/create", post_handler)
        .controller::<ApiController>()
        .run();
}
