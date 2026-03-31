/// Edge tests for lifecycle hooks, controller-level middleware,
/// middleware ordering, and the Month 2 edge-case checklist.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use oxide_framework_core::{controller, ApiResponse, App, AppState};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct Msg {
    text: String,
}

#[derive(Clone)]
struct HitCounter(Arc<AtomicU64>);

// ---------------------------------------------------------------------------
// Controllers
// ---------------------------------------------------------------------------

#[derive(Default)]
struct ApiV1Controller;

#[controller("/api/v1")]
impl ApiV1Controller {
    #[get("/ping")]
    async fn ping(&self) -> ApiResponse<Msg> {
        ApiResponse::ok(Msg { text: "pong-v1".into() })
    }
}

#[derive(Default)]
struct ApiV2Controller;

#[controller("/api/v2")]
impl ApiV2Controller {
    #[get("/ping")]
    async fn ping(&self) -> ApiResponse<Msg> {
        ApiResponse::ok(Msg { text: "pong-v2".into() })
    }
}

#[derive(Default)]
struct GuardedController;

#[controller("/api/guarded")]
impl GuardedController {
    fn middleware(router: axum::Router) -> axum::Router {
        router.layer(axum::middleware::from_fn(require_secret_header))
    }

    #[get("/secret")]
    async fn secret(&self) -> ApiResponse<Msg> {
        ApiResponse::ok(Msg { text: "top-secret".into() })
    }

    #[get("/also-secret")]
    async fn also_secret(&self) -> ApiResponse<Msg> {
        ApiResponse::ok(Msg { text: "classified".into() })
    }
}

async fn require_secret_header(req: Request, next: Next) -> Response {
    if req.headers().get("x-secret").map(|v| v == "open-sesame").unwrap_or(false) {
        next.run(req).await
    } else {
        let body = serde_json::json!({"status": 403, "error": "forbidden"});
        (
            axum::http::StatusCode::FORBIDDEN,
            axum::Json(body),
        )
            .into_response()
    }
}

#[derive(Default)]
struct PanicController;

#[controller("/api/panic")]
impl PanicController {
    #[get("/boom")]
    async fn boom(&self) -> ApiResponse<Msg> {
        panic!("controller panic test");
    }

    #[get("/safe")]
    async fn safe(&self) -> ApiResponse<Msg> {
        ApiResponse::ok(Msg { text: "survived".into() })
    }
}

struct CountingController {
    counter: HitCounter,
}

#[controller("/api/counted")]
impl CountingController {
    fn new(state: &AppState) -> Self {
        Self {
            counter: state.get::<HitCounter>().expect("HitCounter missing").as_ref().clone(),
        }
    }

    #[get("/hit")]
    async fn hit(&self) -> ApiResponse<Msg> {
        let n = self.counter.0.fetch_add(1, Ordering::Relaxed) + 1;
        ApiResponse::ok(Msg { text: format!("{n}") })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn add_powered_by(mut res: Response) -> Response {
    res.headers_mut().insert("x-powered-by", "Oxide".parse().unwrap());
    res
}

async fn counting_hook(req: Request, next: Next) -> Response {
    if let Some(counter) = req.extensions().get::<AppState>() {
        if let Some(c) = counter.get::<HitCounter>() {
            c.0.fetch_add(1, Ordering::Relaxed);
        }
    }
    next.run(req).await
}

use axum::response::IntoResponse;

// ---------------------------------------------------------------------------
// Tests: Global Hooks
// ---------------------------------------------------------------------------

#[tokio::test]
async fn after_hook_adds_header() {
    let server = App::new()
        .disable_request_logging()
        .after(add_powered_by)
        .get("/", || async { ApiResponse::ok(Msg { text: "hi".into() }) })
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/")).await.unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers().get("x-powered-by").unwrap().to_str().unwrap(),
        "Oxide"
    );
}

#[tokio::test]
async fn after_hook_applies_to_error_responses() {
    let server = App::new()
        .disable_request_logging()
        .after(add_powered_by)
        .get("/", || async { ApiResponse::ok(Msg { text: "ok".into() }) })
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/nope")).await.unwrap();
    assert_eq!(res.status(), 404);
    assert_eq!(
        res.headers().get("x-powered-by").unwrap().to_str().unwrap(),
        "Oxide"
    );
}

#[tokio::test]
async fn before_hook_can_short_circuit() {
    async fn block_everything(_req: Request, _next: Next) -> Response {
        (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({"error": "maintenance"})),
        )
            .into_response()
    }

    let server = App::new()
        .disable_request_logging()
        .before(block_everything)
        .get("/", || async { ApiResponse::ok(Msg { text: "should not reach".into() }) })
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/")).await.unwrap();
    assert_eq!(res.status(), 503);
}

#[tokio::test]
async fn before_hook_can_access_state() {
    let counter = HitCounter(Arc::new(AtomicU64::new(0)));
    let server = App::new()
        .disable_request_logging()
        .state(counter.clone())
        .before(counting_hook)
        .get("/", || async { ApiResponse::ok(Msg { text: "ok".into() }) })
        .into_test_server()
        .await;

    for _ in 0..5 {
        reqwest::get(server.url("/")).await.unwrap();
    }

    assert_eq!(counter.0.load(Ordering::Relaxed), 5);
}

// ---------------------------------------------------------------------------
// Tests: Controller-Level Middleware
// ---------------------------------------------------------------------------

#[tokio::test]
async fn controller_middleware_blocks_without_header() {
    let server = App::new()
        .disable_request_logging()
        .controller::<GuardedController>()
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/api/guarded/secret")).await.unwrap();
    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn controller_middleware_allows_with_header() {
    let client = reqwest::Client::new();
    let server = App::new()
        .disable_request_logging()
        .controller::<GuardedController>()
        .into_test_server()
        .await;

    let res = client
        .get(server.url("/api/guarded/secret"))
        .header("x-secret", "open-sesame")
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["data"]["text"], "top-secret");
}

#[tokio::test]
async fn controller_middleware_applies_to_all_routes() {
    let server = App::new()
        .disable_request_logging()
        .controller::<GuardedController>()
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/api/guarded/also-secret")).await.unwrap();
    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn controller_middleware_doesnt_leak_to_other_controllers() {
    let server = App::new()
        .disable_request_logging()
        .controller::<GuardedController>()
        .controller::<ApiV1Controller>()
        .into_test_server()
        .await;

    // Guarded controller blocks
    let res = reqwest::get(server.url("/api/guarded/secret")).await.unwrap();
    assert_eq!(res.status(), 403);

    // V1 controller is unaffected
    let res = reqwest::get(server.url("/api/v1/ping")).await.unwrap();
    assert_eq!(res.status(), 200);
}

// ---------------------------------------------------------------------------
// Tests: Nested / Multiple Controllers
// ---------------------------------------------------------------------------

#[tokio::test]
async fn multiple_versioned_controllers() {
    let server = App::new()
        .disable_request_logging()
        .controller::<ApiV1Controller>()
        .controller::<ApiV2Controller>()
        .into_test_server()
        .await;

    let v1: serde_json::Value = reqwest::get(server.url("/api/v1/ping"))
        .await.unwrap().json().await.unwrap();
    let v2: serde_json::Value = reqwest::get(server.url("/api/v2/ping"))
        .await.unwrap().json().await.unwrap();

    assert_eq!(v1["data"]["text"], "pong-v1");
    assert_eq!(v2["data"]["text"], "pong-v2");
}

#[tokio::test]
async fn controllers_and_manual_routes_coexist() {
    let server = App::new()
        .disable_request_logging()
        .get("/health", || async { ApiResponse::ok(Msg { text: "up".into() }) })
        .controller::<ApiV1Controller>()
        .into_test_server()
        .await;

    let health: serde_json::Value = reqwest::get(server.url("/health"))
        .await.unwrap().json().await.unwrap();
    let v1: serde_json::Value = reqwest::get(server.url("/api/v1/ping"))
        .await.unwrap().json().await.unwrap();

    assert_eq!(health["data"]["text"], "up");
    assert_eq!(v1["data"]["text"], "pong-v1");
}

// ---------------------------------------------------------------------------
// Tests: Panic Isolation (handler + hook)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn panic_in_controller_returns_500() {
    let server = App::new()
        .disable_request_logging()
        .controller::<PanicController>()
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/api/panic/boom")).await.unwrap();
    assert_eq!(res.status(), 500);
}

#[tokio::test]
async fn panic_doesnt_kill_other_routes() {
    let server = App::new()
        .disable_request_logging()
        .controller::<PanicController>()
        .into_test_server()
        .await;

    let _ = reqwest::get(server.url("/api/panic/boom")).await.unwrap();
    let res = reqwest::get(server.url("/api/panic/safe")).await.unwrap();
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["data"]["text"], "survived");
}

#[tokio::test]
async fn panic_in_before_hook_returns_500() {
    async fn panicking_hook(_req: Request, _next: Next) -> Response {
        panic!("hook panic");
    }

    let server = App::new()
        .disable_request_logging()
        .before(panicking_hook)
        .get("/", || async { ApiResponse::ok(Msg { text: "ok".into() }) })
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/")).await.unwrap();
    assert_eq!(res.status(), 500);
}

// ---------------------------------------------------------------------------
// Tests: Hooks + CORS / Rate Limit interaction
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cors_headers_present_with_hooks() {
    let server = App::new()
        .disable_request_logging()
        .cors_permissive()
        .after(add_powered_by)
        .controller::<ApiV1Controller>()
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let res = client
        .get(server.url("/api/v1/ping"))
        .header("Origin", "http://evil.com")
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    assert!(res.headers().contains_key("access-control-allow-origin"));
    assert_eq!(
        res.headers().get("x-powered-by").unwrap().to_str().unwrap(),
        "Oxide"
    );
}

#[tokio::test]
async fn rate_limit_applies_to_controller_routes() {
    let server = App::new()
        .disable_request_logging()
        .rate_limit(2, 60)
        .controller::<ApiV1Controller>()
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let r1 = client.get(server.url("/api/v1/ping")).send().await.unwrap();
    let r2 = client.get(server.url("/api/v1/ping")).send().await.unwrap();
    let r3 = client.get(server.url("/api/v1/ping")).send().await.unwrap();

    assert_eq!(r1.status(), 200);
    assert_eq!(r2.status(), 200);
    assert_eq!(r3.status(), 429);
}

#[tokio::test]
async fn cors_headers_on_rate_limited_response() {
    let server = App::new()
        .disable_request_logging()
        .cors_permissive()
        .rate_limit(1, 60)
        .controller::<ApiV1Controller>()
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let _ = client.get(server.url("/api/v1/ping")).send().await.unwrap();
    let res = client
        .get(server.url("/api/v1/ping"))
        .header("Origin", "http://other.com")
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 429);
    assert!(res.headers().contains_key("access-control-allow-origin"));
}

#[tokio::test]
async fn preflight_options_through_controller_routes() {
    let server = App::new()
        .disable_request_logging()
        .cors_permissive()
        .controller::<ApiV1Controller>()
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let res = client
        .request(reqwest::Method::OPTIONS, server.url("/api/v1/ping"))
        .header("Origin", "http://example.com")
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .unwrap();

    assert!(res.status().is_success());
    assert!(res.headers().contains_key("access-control-allow-origin"));
    assert!(res.headers().contains_key("access-control-allow-methods"));
}

// ---------------------------------------------------------------------------
// Tests: Concurrency
// ---------------------------------------------------------------------------

#[tokio::test]
async fn concurrent_requests_to_multiple_controllers() {
    let server = App::new()
        .disable_request_logging()
        .controller::<ApiV1Controller>()
        .controller::<ApiV2Controller>()
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let mut handles = Vec::new();

    for i in 0..100 {
        let c = client.clone();
        let version = if i % 2 == 0 { "v1" } else { "v2" };
        let url = server.url(&format!("/api/{version}/ping"));
        handles.push(tokio::spawn(async move {
            let res = c.get(&url).send().await.unwrap();
            assert_eq!(res.status(), 200);
            let body: serde_json::Value = res.json().await.unwrap();
            let expected = format!("pong-{version}");
            assert_eq!(body["data"]["text"], expected);
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
}

#[tokio::test]
async fn concurrent_requests_with_stateful_controller() {
    let counter = HitCounter(Arc::new(AtomicU64::new(0)));
    let server = App::new()
        .disable_request_logging()
        .state(counter.clone())
        .controller::<CountingController>()
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let mut handles = Vec::new();

    for _ in 0..50 {
        let c = client.clone();
        let url = server.url("/api/counted/hit");
        handles.push(tokio::spawn(async move {
            c.get(&url).send().await.unwrap();
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    assert_eq!(counter.0.load(Ordering::Relaxed), 50);
}

// ---------------------------------------------------------------------------
// Tests: Middleware Ordering
// ---------------------------------------------------------------------------

#[tokio::test]
async fn hook_runs_after_state_injection() {
    async fn state_check_hook(req: Request, next: Next) -> Response {
        let has_state = req.extensions().get::<AppState>().is_some();
        let mut resp = next.run(req).await;
        resp.headers_mut().insert(
            "x-state-available",
            if has_state { "yes" } else { "no" }.parse().unwrap(),
        );
        resp
    }

    let server = App::new()
        .disable_request_logging()
        .before(state_check_hook)
        .get("/", || async { ApiResponse::ok(Msg { text: "ok".into() }) })
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/")).await.unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.headers().get("x-state-available").unwrap().to_str().unwrap(),
        "yes"
    );
}

#[tokio::test]
async fn timeout_applies_to_hooks() {
    async fn slow_hook(req: Request, next: Next) -> Response {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        next.run(req).await
    }

    let server = App::new()
        .disable_request_logging()
        .request_timeout(1)
        .before(slow_hook)
        .get("/", || async { ApiResponse::ok(Msg { text: "ok".into() }) })
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/")).await.unwrap();
    assert_eq!(res.status(), 408);
}


