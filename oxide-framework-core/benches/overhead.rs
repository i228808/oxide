//! Micro-benchmarks: raw Axum vs Oxide framework overhead.
//!
//! Run:  `cargo bench -p oxide_core`
//!
//! Each benchmark starts real HTTP servers on random ports and measures
//! round-trip latency through reqwest.  This captures the true cost of
//! the Oxide middleware stack vs bare Axum.

use axum::body::Body;
use axum::extract::Path;
use axum::http::Request;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use oxide_core::{controller, ApiResponse, App};
use serde::Serialize;
use std::net::SocketAddr;
use std::time::Duration;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct Msg {
    text: String,
}

#[derive(Serialize)]
struct User {
    id: u64,
    name: String,
}

// ---------------------------------------------------------------------------
// Raw Axum handlers (return identical JSON shape to Oxide's ApiResponse)
// ---------------------------------------------------------------------------

async fn axum_json() -> impl IntoResponse {
    axum::Json(serde_json::json!({"status":200,"data":{"text":"hello"}}))
}

async fn axum_path(Path(id): Path<u64>) -> impl IntoResponse {
    axum::Json(serde_json::json!({"status":200,"data":{"id":id,"name":format!("user-{id}")}}))
}

async fn axum_post(axum::Json(body): axum::Json<serde_json::Value>) -> impl IntoResponse {
    (
        axum::http::StatusCode::CREATED,
        axum::Json(serde_json::json!({"status":201,"data":body})),
    )
}

// ---------------------------------------------------------------------------
// Oxide handlers
// ---------------------------------------------------------------------------

async fn oxide_json() -> ApiResponse<Msg> {
    ApiResponse::ok(Msg { text: "hello".into() })
}

async fn oxide_path(oxide_core::Path(id): oxide_core::Path<u64>) -> ApiResponse<User> {
    ApiResponse::ok(User { id, name: format!("user-{id}") })
}

async fn oxide_post(
    oxide_core::Json(body): oxide_core::Json<serde_json::Value>,
) -> ApiResponse<serde_json::Value> {
    ApiResponse::created(body)
}

// ---------------------------------------------------------------------------
// Oxide controller
// ---------------------------------------------------------------------------

#[derive(Default)]
struct BenchController;

#[controller("/api")]
impl BenchController {
    #[get("/json")]
    async fn json_handler(&self) -> ApiResponse<Msg> {
        ApiResponse::ok(Msg { text: "hello".into() })
    }

    #[get("/users/{id}")]
    async fn user_handler(&self, oxide_core::Path(id): oxide_core::Path<u64>) -> ApiResponse<User> {
        ApiResponse::ok(User { id, name: format!("user-{id}") })
    }
}

// ---------------------------------------------------------------------------
// Server builders
// ---------------------------------------------------------------------------

fn raw_axum_router() -> Router {
    Router::new()
        .route("/json", get(axum_json))
        .route("/users/{id}", get(axum_path))
        .route("/create", post(axum_post))
}

async fn start_raw_axum() -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, raw_axum_router()).await.ok();
    });
    addr
}

// ---------------------------------------------------------------------------
// 1. In-process oneshot: pure routing overhead (no network)
// ---------------------------------------------------------------------------

fn bench_oneshot(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("oneshot_no_network");

    let raw = raw_axum_router();

    group.bench_function("raw_axum/GET_json", |b| {
        b.to_async(&rt).iter(|| {
            let r = raw.clone();
            async move {
                let req = Request::get("/json").body(Body::empty()).unwrap();
                black_box(r.oneshot(req).await.unwrap());
            }
        });
    });

    group.bench_function("raw_axum/GET_path_param", |b| {
        b.to_async(&rt).iter(|| {
            let r = raw.clone();
            async move {
                let req = Request::get("/users/42").body(Body::empty()).unwrap();
                black_box(r.oneshot(req).await.unwrap());
            }
        });
    });

    group.bench_function("raw_axum/POST_json", |b| {
        b.to_async(&rt).iter(|| {
            let r = raw.clone();
            async move {
                let req = Request::post("/create")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"bench"}"#))
                    .unwrap();
                black_box(r.oneshot(req).await.unwrap());
            }
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// 2. Real HTTP round-trip: raw Axum vs Oxide variants
// ---------------------------------------------------------------------------

fn bench_roundtrip(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("http_roundtrip");
    group.sample_size(500);
    group.measurement_time(Duration::from_secs(10));

    // Start servers
    let raw_addr = rt.block_on(start_raw_axum());

    let oxide_min = rt.block_on(
        App::new()
            .disable_request_logging()
            .get("/json", oxide_json)
            .get("/users/{id}", oxide_path)
            .post("/create", oxide_post)
            .into_test_server(),
    );

    let oxide_full = rt.block_on(
        App::new()
            .disable_request_logging()
            .rate_limit(1_000_000, 60)
            .cors_permissive()
            .request_timeout(30)
            .get("/json", oxide_json)
            .get("/users/{id}", oxide_path)
            .post("/create", oxide_post)
            .into_test_server(),
    );

    let oxide_ctrl = rt.block_on(
        App::new()
            .disable_request_logging()
            .rate_limit(1_000_000, 60)
            .cors_permissive()
            .request_timeout(30)
            .controller::<BenchController>()
            .into_test_server(),
    );

    let client = reqwest::Client::new();

    // -- GET /json --------------------------------------------------------

    group.bench_function("GET_json/raw_axum", |b| {
        let url = format!("http://{}/json", raw_addr);
        b.to_async(&rt).iter(|| {
            let c = client.clone();
            let u = url.clone();
            async move { black_box(c.get(&u).send().await.unwrap().status()) }
        });
    });

    group.bench_function("GET_json/oxide_minimal", |b| {
        let url = oxide_min.url("/json");
        b.to_async(&rt).iter(|| {
            let c = client.clone();
            let u = url.clone();
            async move { black_box(c.get(&u).send().await.unwrap().status()) }
        });
    });

    group.bench_function("GET_json/oxide_full_stack", |b| {
        let url = oxide_full.url("/json");
        b.to_async(&rt).iter(|| {
            let c = client.clone();
            let u = url.clone();
            async move { black_box(c.get(&u).send().await.unwrap().status()) }
        });
    });

    group.bench_function("GET_json/oxide_controller", |b| {
        let url = oxide_ctrl.url("/api/json");
        b.to_async(&rt).iter(|| {
            let c = client.clone();
            let u = url.clone();
            async move { black_box(c.get(&u).send().await.unwrap().status()) }
        });
    });

    // -- GET /users/42 ----------------------------------------------------

    group.bench_function("GET_param/raw_axum", |b| {
        let url = format!("http://{}/users/42", raw_addr);
        b.to_async(&rt).iter(|| {
            let c = client.clone();
            let u = url.clone();
            async move { black_box(c.get(&u).send().await.unwrap().status()) }
        });
    });

    group.bench_function("GET_param/oxide_full_stack", |b| {
        let url = oxide_full.url("/users/42");
        b.to_async(&rt).iter(|| {
            let c = client.clone();
            let u = url.clone();
            async move { black_box(c.get(&u).send().await.unwrap().status()) }
        });
    });

    // -- POST /create -----------------------------------------------------

    group.bench_function("POST_json/raw_axum", |b| {
        let url = format!("http://{}/create", raw_addr);
        b.to_async(&rt).iter(|| {
            let c = client.clone();
            let u = url.clone();
            async move {
                black_box(
                    c.post(&u)
                        .json(&serde_json::json!({"name":"bench"}))
                        .send()
                        .await
                        .unwrap()
                        .status(),
                )
            }
        });
    });

    group.bench_function("POST_json/oxide_full_stack", |b| {
        let url = oxide_full.url("/create");
        b.to_async(&rt).iter(|| {
            let c = client.clone();
            let u = url.clone();
            async move {
                black_box(
                    c.post(&u)
                        .json(&serde_json::json!({"name":"bench"}))
                        .send()
                        .await
                        .unwrap()
                        .status(),
                )
            }
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// 3. Concurrent throughput at various levels
// ---------------------------------------------------------------------------

fn bench_concurrent(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_throughput");
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(15));

    let raw_addr = rt.block_on(start_raw_axum());
    let oxide_server = rt.block_on(
        App::new()
            .disable_request_logging()
            .rate_limit(10_000_000, 60)
            .cors_permissive()
            .request_timeout(30)
            .get("/json", oxide_json)
            .into_test_server(),
    );

    let client = reqwest::Client::new();

    for n in [10, 50, 100] {
        group.bench_with_input(BenchmarkId::new("raw_axum", n), &n, |b, &n| {
            let url = format!("http://{}/json", raw_addr);
            b.to_async(&rt).iter(|| {
                let c = client.clone();
                let u = url.clone();
                async move {
                    let futs: Vec<_> = (0..n)
                        .map(|_| {
                            let c = c.clone();
                            let u = u.clone();
                            tokio::spawn(async move { c.get(&u).send().await.unwrap().status() })
                        })
                        .collect();
                    for f in futs {
                        black_box(f.await.unwrap());
                    }
                }
            });
        });

        group.bench_with_input(BenchmarkId::new("oxide_full", n), &n, |b, &n| {
            let url = oxide_server.url("/json");
            b.to_async(&rt).iter(|| {
                let c = client.clone();
                let u = url.clone();
                async move {
                    let futs: Vec<_> = (0..n)
                        .map(|_| {
                            let c = c.clone();
                            let u = u.clone();
                            tokio::spawn(async move { c.get(&u).send().await.unwrap().status() })
                        })
                        .collect();
                    for f in futs {
                        black_box(f.await.unwrap());
                    }
                }
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_oneshot, bench_roundtrip, bench_concurrent);
criterion_main!(benches);
