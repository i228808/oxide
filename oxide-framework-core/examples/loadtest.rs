//! Cross-platform load test runner.  No external tools required.
//!
//! Spins up a raw Axum server and an Oxide server, hammers both with
//! concurrent requests, and prints a comparison table.
//!
//! Run:  `cargo run -p oxide_core --release --example loadtest`
//!
//! Options via env vars:
//!   DURATION=10    — test duration in seconds (default 10)
//!   CONCURRENCY=50 — concurrent tasks (default 50)

use axum::extract::Path;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use oxide_core::{controller, ApiResponse, App};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct Msg { text: String }
#[derive(Serialize)]
struct User { id: u64, name: String }

// ---------------------------------------------------------------------------
// Raw Axum
// ---------------------------------------------------------------------------

async fn axum_json() -> impl IntoResponse {
    axum::Json(serde_json::json!({"status":200,"data":{"text":"hello"}}))
}

async fn axum_path(Path(id): Path<u64>) -> impl IntoResponse {
    axum::Json(serde_json::json!({"status":200,"data":{"id":id,"name":format!("user-{id}")}}))
}

async fn axum_post(axum::Json(b): axum::Json<serde_json::Value>) -> impl IntoResponse {
    (axum::http::StatusCode::CREATED, axum::Json(serde_json::json!({"status":201,"data":b})))
}

// ---------------------------------------------------------------------------
// Oxide
// ---------------------------------------------------------------------------

async fn oxide_json() -> ApiResponse<Msg> {
    ApiResponse::ok(Msg { text: "hello".into() })
}
async fn oxide_path(oxide_core::Path(id): oxide_core::Path<u64>) -> ApiResponse<User> {
    ApiResponse::ok(User { id, name: format!("user-{id}") })
}
async fn oxide_post(oxide_core::Json(b): oxide_core::Json<serde_json::Value>) -> ApiResponse<serde_json::Value> {
    ApiResponse::created(b)
}

#[derive(Default)]
struct BenchCtrl;

#[controller("/api")]
impl BenchCtrl {
    #[get("/json")]
    async fn json_h(&self) -> ApiResponse<Msg> {
        ApiResponse::ok(Msg { text: "hello".into() })
    }
}

// ---------------------------------------------------------------------------
// Load driver
// ---------------------------------------------------------------------------

struct Stats {
    total: u64,
    success: u64,
    errors: u64,
    duration: Duration,
    latencies: Vec<Duration>,
}

impl Stats {
    fn rps(&self) -> f64 {
        self.total as f64 / self.duration.as_secs_f64()
    }

    fn p50(&self) -> Duration {
        self.percentile(50)
    }
    fn p95(&self) -> Duration {
        self.percentile(95)
    }
    fn p99(&self) -> Duration {
        self.percentile(99)
    }

    fn percentile(&self, p: usize) -> Duration {
        if self.latencies.is_empty() {
            return Duration::ZERO;
        }
        let idx = (p * self.latencies.len() / 100).min(self.latencies.len() - 1);
        self.latencies[idx]
    }

    fn avg(&self) -> Duration {
        if self.latencies.is_empty() {
            return Duration::ZERO;
        }
        let sum: Duration = self.latencies.iter().sum();
        sum / self.latencies.len() as u32
    }
}

async fn run_load(url: &str, concurrency: usize, duration: Duration) -> Stats {
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(concurrency)
        .build()
        .unwrap();

    let total = Arc::new(AtomicU64::new(0));
    let success = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));
    let latencies: Arc<tokio::sync::Mutex<Vec<Duration>>> =
        Arc::new(tokio::sync::Mutex::new(Vec::new()));

    let deadline = Instant::now() + duration;

    let mut handles = Vec::new();
    for _ in 0..concurrency {
        let c = client.clone();
        let u = url.to_string();
        let t = total.clone();
        let s = success.clone();
        let e = errors.clone();
        let l = latencies.clone();

        handles.push(tokio::spawn(async move {
            while Instant::now() < deadline {
                let start = Instant::now();
                let result = c.get(&u).send().await;
                let elapsed = start.elapsed();

                t.fetch_add(1, Ordering::Relaxed);
                match result {
                    Ok(r) if r.status().is_success() => {
                        s.fetch_add(1, Ordering::Relaxed);
                    }
                    _ => {
                        e.fetch_add(1, Ordering::Relaxed);
                    }
                }
                l.lock().await.push(elapsed);
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let mut lats = Arc::try_unwrap(latencies).unwrap().into_inner();
    lats.sort();

    Stats {
        total: total.load(Ordering::Relaxed),
        success: success.load(Ordering::Relaxed),
        errors: errors.load(Ordering::Relaxed),
        duration,
        latencies: lats,
    }
}

fn print_stats(label: &str, s: &Stats) {
    println!(
        "  {:<28} {:>8} req  {:>10.0} req/s  avg {:>6.2}ms  p50 {:>6.2}ms  p95 {:>6.2}ms  p99 {:>6.2}ms  err {}",
        label,
        s.total,
        s.rps(),
        s.avg().as_secs_f64() * 1000.0,
        s.p50().as_secs_f64() * 1000.0,
        s.p95().as_secs_f64() * 1000.0,
        s.p99().as_secs_f64() * 1000.0,
        s.errors,
    );
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let duration_secs: u64 = std::env::var("DURATION")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let concurrency: usize = std::env::var("CONCURRENCY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50);
    let duration = Duration::from_secs(duration_secs);

    println!("=== Oxide Framework Load Test ===");
    println!("Duration: {}s  |  Concurrency: {}", duration_secs, concurrency);
    println!();

    // -- Start servers ----------------------------------------------------

    // Raw Axum
    let raw_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let raw_addr = raw_listener.local_addr().unwrap();
    tokio::spawn(async move {
        let app = Router::new()
            .route("/json", get(axum_json))
            .route("/users/{id}", get(axum_path))
            .route("/create", post(axum_post));
        axum::serve(raw_listener, app).await.ok();
    });

    // Oxide minimal (state injection only)
    let oxide_min = App::new()
        .disable_request_logging()
        .get("/json", oxide_json)
        .get("/users/{id}", oxide_path)
        .post("/create", oxide_post)
        .into_test_server()
        .await;

    // Oxide full middleware
    let oxide_full = App::new()
        .disable_request_logging()
        .rate_limit(10_000_000, 60)
        .cors_permissive()
        .request_timeout(30)
        .get("/json", oxide_json)
        .get("/users/{id}", oxide_path)
        .post("/create", oxide_post)
        .into_test_server()
        .await;

    // Oxide controller + full middleware
    let oxide_ctrl = App::new()
        .disable_request_logging()
        .rate_limit(10_000_000, 60)
        .cors_permissive()
        .request_timeout(30)
        .controller::<BenchCtrl>()
        .into_test_server()
        .await;

    // Warm up connection pools
    let client = reqwest::Client::new();
    for url in &[
        format!("http://{}/json", raw_addr),
        oxide_min.url("/json"),
        oxide_full.url("/json"),
        oxide_ctrl.url("/api/json"),
    ] {
        let _ = client.get(url).send().await;
    }

    // -- Run tests --------------------------------------------------------

    println!("--- GET /json ---");
    let raw  = run_load(&format!("http://{}/json", raw_addr), concurrency, duration).await;
    let omin = run_load(&oxide_min.url("/json"), concurrency, duration).await;
    let ofull= run_load(&oxide_full.url("/json"), concurrency, duration).await;
    let octrl= run_load(&oxide_ctrl.url("/api/json"), concurrency, duration).await;

    print_stats("Raw Axum", &raw);
    print_stats("Oxide (minimal)", &omin);
    print_stats("Oxide (full middleware)", &ofull);
    print_stats("Oxide (controller + full)", &octrl);

    let overhead_pct = ((ofull.avg().as_nanos() as f64 / raw.avg().as_nanos() as f64) - 1.0) * 100.0;
    println!();
    println!("  Framework overhead (full stack vs raw): {overhead_pct:+.1}% avg latency");

    println!();
    println!("--- GET /users/42 (path param) ---");
    let raw_p  = run_load(&format!("http://{}/users/42", raw_addr), concurrency, duration).await;
    let ofull_p = run_load(&oxide_full.url("/users/42"), concurrency, duration).await;

    print_stats("Raw Axum", &raw_p);
    print_stats("Oxide (full middleware)", &ofull_p);

    let overhead_p = ((ofull_p.avg().as_nanos() as f64 / raw_p.avg().as_nanos() as f64) - 1.0) * 100.0;
    println!("  Framework overhead: {overhead_p:+.1}% avg latency");

    println!();
    println!("=== Done ===");
}
