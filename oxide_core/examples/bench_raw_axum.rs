//! Bare Axum server for load testing comparison.
//!
//! Start:  `cargo run -p oxide_core --release --example bench_raw_axum`
//! Test:   `wrk -t4 -c100 -d30s http://127.0.0.1:3001/json`

use axum::extract::Path;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use std::net::SocketAddr;

async fn json_handler() -> impl IntoResponse {
    axum::Json(serde_json::json!({"status":200,"data":{"text":"hello"}}))
}

async fn path_handler(Path(id): Path<u64>) -> impl IntoResponse {
    axum::Json(serde_json::json!({"status":200,"data":{"id":id,"name":format!("user-{id}")}}))
}

async fn post_handler(axum::Json(body): axum::Json<serde_json::Value>) -> impl IntoResponse {
    (
        axum::http::StatusCode::CREATED,
        axum::Json(serde_json::json!({"status":201,"data":body})),
    )
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/json", get(json_handler))
        .route("/users/{id}", get(path_handler))
        .route("/create", post(post_handler));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    println!("Raw Axum server listening on http://{addr}");
    println!("Endpoints: GET /json, GET /users/{{id}}, POST /create");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
