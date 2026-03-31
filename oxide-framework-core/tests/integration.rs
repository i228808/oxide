use oxide_core::{App, ApiResponse, Config, Data, Json, OxideRouter, Path};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

// ============================================================================
// Helpers
// ============================================================================

async fn hello() -> &'static str {
    "hello"
}

async fn echo_body(body: String) -> String {
    body
}

#[derive(Serialize, Deserialize, Debug)]
struct Msg {
    text: String,
}

async fn json_ok() -> ApiResponse<Msg> {
    ApiResponse::ok(Msg {
        text: "hi".into(),
    })
}

async fn greet(Path(name): Path<String>) -> String {
    format!("hello {name}")
}

async fn slow_handler() -> &'static str {
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    "done"
}

async fn fast_handler() -> &'static str {
    "fast"
}

async fn crash_handler() -> &'static str {
    panic!("handler exploded")
}

// ============================================================================
// 1. Server Boot
// ============================================================================

#[tokio::test]
async fn test_server_starts_and_responds() {
    let server = App::new()
        .disable_request_logging()
        .get("/", hello)
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/")).await.unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "hello");
}

// ============================================================================
// 2. Routing
// ============================================================================

#[tokio::test]
async fn test_get_route() {
    let server = App::new()
        .disable_request_logging()
        .get("/ping", || async { "pong" })
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/ping")).await.unwrap();
    assert_eq!(res.text().await.unwrap(), "pong");
}

#[tokio::test]
async fn test_post_route() {
    let server = App::new()
        .disable_request_logging()
        .post("/echo", echo_body)
        .into_test_server()
        .await;

    let client = Client::new();
    let res = client
        .post(server.url("/echo"))
        .body("payload")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "payload");
}

#[tokio::test]
async fn test_put_delete_patch() {
    let server = App::new()
        .disable_request_logging()
        .put("/r", || async { "put" })
        .delete("/r", || async { "delete" })
        .patch("/r", || async { "patch" })
        .into_test_server()
        .await;

    let client = Client::new();

    let res = client.put(server.url("/r")).send().await.unwrap();
    assert_eq!(res.text().await.unwrap(), "put");

    let res = client.delete(server.url("/r")).send().await.unwrap();
    assert_eq!(res.text().await.unwrap(), "delete");

    let res = client.patch(server.url("/r")).send().await.unwrap();
    assert_eq!(res.text().await.unwrap(), "patch");
}

#[tokio::test]
async fn test_path_params() {
    let server = App::new()
        .disable_request_logging()
        .get("/users/{name}", greet)
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/users/alice")).await.unwrap();
    assert_eq!(res.text().await.unwrap(), "hello alice");
}

#[tokio::test]
async fn test_nested_routes() {
    let api = OxideRouter::new()
        .get("/health", || async { "ok" })
        .get("/version", || async { "1.0" });

    let server = App::new()
        .disable_request_logging()
        .nest("/api", api)
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/api/health")).await.unwrap();
    assert_eq!(res.text().await.unwrap(), "ok");

    let res = reqwest::get(server.url("/api/version")).await.unwrap();
    assert_eq!(res.text().await.unwrap(), "1.0");
}

#[tokio::test]
async fn test_404_unknown_route() {
    let server = App::new()
        .disable_request_logging()
        .get("/", hello)
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/nonexistent")).await.unwrap();
    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn test_method_not_allowed() {
    let server = App::new()
        .disable_request_logging()
        .get("/only-get", hello)
        .into_test_server()
        .await;

    let client = Client::new();
    let res = client.post(server.url("/only-get")).send().await.unwrap();
    assert_eq!(res.status(), 405);
}

#[tokio::test]
async fn test_api_response_json_envelope() {
    let server = App::new()
        .disable_request_logging()
        .get("/json", json_ok)
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/json")).await.unwrap();
    assert_eq!(res.status(), 200);

    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["status"], 200);
    assert_eq!(body["data"]["text"], "hi");
}

// ============================================================================
// 3. CORS Validation
// ============================================================================

#[tokio::test]
async fn test_cors_preflight_returns_ok() {
    let server = App::new()
        .disable_request_logging()
        .cors_permissive()
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();
    let res = client
        .request(reqwest::Method::OPTIONS, server.url("/"))
        .header("Origin", "http://example.com")
        .header("Access-Control-Request-Method", "POST")
        .send()
        .await
        .unwrap();

    let status = res.status().as_u16();
    assert!(status == 200 || status == 204, "preflight status was {status}");
    assert!(res.headers().contains_key("access-control-allow-origin"));
    assert!(res.headers().contains_key("access-control-allow-methods"));
}

#[tokio::test]
async fn test_cors_headers_on_normal_response() {
    let server = App::new()
        .disable_request_logging()
        .cors_permissive()
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();
    let res = client
        .get(server.url("/"))
        .header("Origin", "http://example.com")
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    let acao = res
        .headers()
        .get("access-control-allow-origin")
        .expect("missing CORS header on normal response");
    assert_eq!(acao.to_str().unwrap(), "*");
}

#[tokio::test]
async fn test_cors_headers_on_429_response() {
    let server = App::new()
        .disable_request_logging()
        .cors_permissive()
        .rate_limit(1, 60)
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();

    // Exhaust the limit
    let _ = client.get(server.url("/")).send().await;

    // This should be 429 AND still have CORS headers
    let res = client
        .get(server.url("/"))
        .header("Origin", "http://example.com")
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 429);
    assert!(
        res.headers().contains_key("access-control-allow-origin"),
        "CORS headers missing on 429 response — layer order is wrong"
    );
}

#[tokio::test]
async fn test_cors_headers_on_408_response() {
    let server = App::new()
        .disable_request_logging()
        .cors_permissive()
        .request_timeout(1)
        .get("/slow", slow_handler)
        .into_test_server()
        .await;

    let client = Client::new();
    let res = client
        .get(server.url("/slow"))
        .header("Origin", "http://example.com")
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 408);
    assert!(
        res.headers().contains_key("access-control-allow-origin"),
        "CORS headers missing on 408 response — layer order is wrong"
    );
}

#[tokio::test]
async fn test_cors_restricted_origin_allowed() {
    let server = App::new()
        .disable_request_logging()
        .cors_origins(["http://allowed.com"])
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();
    let res = client
        .get(server.url("/"))
        .header("Origin", "http://allowed.com")
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    let acao = res
        .headers()
        .get("access-control-allow-origin")
        .expect("allowed origin should get CORS header");
    assert_eq!(acao.to_str().unwrap(), "http://allowed.com");
}

#[tokio::test]
async fn test_cors_restricted_origin_blocked() {
    let server = App::new()
        .disable_request_logging()
        .cors_origins(["http://allowed.com"])
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();
    let res = client
        .get(server.url("/"))
        .header("Origin", "http://evil.com")
        .send()
        .await
        .unwrap();

    // Request succeeds but no CORS header — browser will block it
    assert_eq!(res.status(), 200);
    assert!(
        !res.headers().contains_key("access-control-allow-origin"),
        "disallowed origin should NOT get CORS header"
    );
}

#[tokio::test]
async fn test_no_cors_when_not_configured() {
    let server = App::new()
        .disable_request_logging()
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();
    let res = client
        .get(server.url("/"))
        .header("Origin", "http://example.com")
        .send()
        .await
        .unwrap();

    assert!(!res.headers().contains_key("access-control-allow-origin"));
}

// ============================================================================
// 4. Rate Limiting
// ============================================================================

#[tokio::test]
async fn test_rate_limit_allows_within_limit() {
    let server = App::new()
        .disable_request_logging()
        .rate_limit(10, 60)
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();
    for _ in 0..10 {
        let res = client.get(server.url("/")).send().await.unwrap();
        assert_eq!(res.status(), 200, "request within limit should succeed");
    }
}

#[tokio::test]
async fn test_rate_limit_blocks_over_limit() {
    let server = App::new()
        .disable_request_logging()
        .rate_limit(5, 60)
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();

    for _ in 0..5 {
        let res = client.get(server.url("/")).send().await.unwrap();
        assert_eq!(res.status(), 200);
    }

    // 6th request should be blocked
    let res = client.get(server.url("/")).send().await.unwrap();
    assert_eq!(res.status(), 429);

    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["status"], 429);
    assert_eq!(body["error"], "rate limit exceeded");
}

#[tokio::test]
async fn test_rate_limit_returns_retry_after() {
    let server = App::new()
        .disable_request_logging()
        .rate_limit(1, 120)
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();
    let _ = client.get(server.url("/")).send().await;

    let res = client.get(server.url("/")).send().await.unwrap();
    assert_eq!(res.status(), 429);

    let retry_after = res
        .headers()
        .get("retry-after")
        .expect("429 should include Retry-After header");
    assert_eq!(retry_after.to_str().unwrap(), "120");
}

#[tokio::test]
async fn test_rate_limit_per_ip_isolation() {
    let server = App::new()
        .disable_request_logging()
        .rate_limit(2, 60)
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();

    // Exhaust limit for IP "10.0.0.1"
    for _ in 0..2 {
        let _ = client
            .get(server.url("/"))
            .header("X-Forwarded-For", "10.0.0.1")
            .send()
            .await;
    }

    let res = client
        .get(server.url("/"))
        .header("X-Forwarded-For", "10.0.0.1")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 429, "IP 10.0.0.1 should be rate-limited");

    // Different IP should still work
    let res = client
        .get(server.url("/"))
        .header("X-Forwarded-For", "10.0.0.2")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "IP 10.0.0.2 should NOT be rate-limited");
}

#[tokio::test]
async fn test_rate_limit_resets_after_window() {
    let server = App::new()
        .disable_request_logging()
        .rate_limit(2, 2) // 2 requests per 2 seconds
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();

    // Exhaust limit
    for _ in 0..2 {
        let _ = client.get(server.url("/")).send().await;
    }
    let res = client.get(server.url("/")).send().await.unwrap();
    assert_eq!(res.status(), 429);

    // Wait for window to reset
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let res = client.get(server.url("/")).send().await.unwrap();
    assert_eq!(res.status(), 200, "should succeed after window reset");
}

#[tokio::test]
async fn test_rate_limit_concurrent_requests() {
    let server = App::new()
        .disable_request_logging()
        .rate_limit(50, 60)
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();
    let url = server.url("/");

    let mut handles = Vec::new();
    for _ in 0..100 {
        let c = client.clone();
        let u = url.clone();
        handles.push(tokio::spawn(async move {
            c.get(&u).send().await.unwrap().status()
        }));
    }

    let mut ok_count = 0u64;
    let mut limited_count = 0u64;

    for h in handles {
        let status = h.await.unwrap();
        if status == 200 {
            ok_count += 1;
        } else if status == 429 {
            limited_count += 1;
        } else {
            panic!("unexpected status: {status}");
        }
    }

    assert_eq!(ok_count, 50, "exactly 50 requests should succeed");
    assert_eq!(limited_count, 50, "exactly 50 requests should be rate-limited");
    assert_eq!(ok_count + limited_count, 100, "no requests lost");
}

// ============================================================================
// 5. Timeout
// ============================================================================

#[tokio::test]
async fn test_timeout_returns_408() {
    let server = App::new()
        .disable_request_logging()
        .request_timeout(1)
        .get("/slow", slow_handler)
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/slow")).await.unwrap();
    assert_eq!(res.status(), 408);

    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["status"], 408);
    assert_eq!(body["error"], "request timeout");
}

#[tokio::test]
async fn test_fast_handler_not_timed_out() {
    let server = App::new()
        .disable_request_logging()
        .request_timeout(5)
        .get("/fast", fast_handler)
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/fast")).await.unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "fast");
}

// ============================================================================
// 6. State & Config Extractors
// ============================================================================

struct Counter {
    value: AtomicU64,
}

async fn read_counter(Data(counter): Data<Counter>) -> String {
    counter.value.load(Ordering::Relaxed).to_string()
}

async fn increment_counter(Data(counter): Data<Counter>) -> String {
    counter.value.fetch_add(1, Ordering::Relaxed);
    "incremented".to_string()
}

async fn read_config(Config(cfg): Config) -> String {
    cfg.app_name.clone()
}

#[tokio::test]
async fn test_config_extractor() {
    let server = App::new()
        .disable_request_logging()
        .get("/name", read_config)
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/name")).await.unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "oxide-app");
}

#[tokio::test]
async fn test_data_extractor() {
    let counter = Counter {
        value: AtomicU64::new(42),
    };

    let server = App::new()
        .disable_request_logging()
        .state(counter)
        .get("/count", read_counter)
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/count")).await.unwrap();
    assert_eq!(res.text().await.unwrap(), "42");
}

#[tokio::test]
async fn test_shared_state_mutation() {
    let counter = Counter {
        value: AtomicU64::new(0),
    };

    let server = App::new()
        .disable_request_logging()
        .state(counter)
        .post("/inc", increment_counter)
        .get("/count", read_counter)
        .into_test_server()
        .await;

    let client = Client::new();

    for _ in 0..5 {
        client.post(server.url("/inc")).send().await.unwrap();
    }

    let res = client.get(server.url("/count")).send().await.unwrap();
    assert_eq!(res.text().await.unwrap(), "5");
}

// ============================================================================
// 7. Error Handling & Abuse
// ============================================================================

#[derive(Deserialize)]
struct Payload {
    #[allow(dead_code)]
    name: String,
}

async fn expect_json(Json(_payload): Json<Payload>) -> &'static str {
    "ok"
}

#[tokio::test]
async fn test_malformed_json_returns_client_error() {
    let server = App::new()
        .disable_request_logging()
        .post("/data", expect_json)
        .into_test_server()
        .await;

    let client = Client::new();
    let res = client
        .post(server.url("/data"))
        .header("content-type", "application/json")
        .body("{invalid json!!")
        .send()
        .await
        .unwrap();

    assert!(
        res.status().is_client_error(),
        "malformed JSON should return 4xx, got {}",
        res.status()
    );
}

#[tokio::test]
async fn test_missing_content_type_for_json() {
    let server = App::new()
        .disable_request_logging()
        .post("/data", expect_json)
        .into_test_server()
        .await;

    let client = Client::new();
    let res = client
        .post(server.url("/data"))
        .body(r#"{"name":"test"}"#)
        .send()
        .await
        .unwrap();

    assert!(
        res.status().is_client_error(),
        "missing content-type should return 4xx, got {}",
        res.status()
    );
}

#[tokio::test]
async fn test_empty_body_for_json_route() {
    let server = App::new()
        .disable_request_logging()
        .post("/data", expect_json)
        .into_test_server()
        .await;

    let client = Client::new();
    let res = client
        .post(server.url("/data"))
        .header("content-type", "application/json")
        .send()
        .await
        .unwrap();

    assert!(res.status().is_client_error());
}

// ============================================================================
// 8. Burst / Abuse Testing
// ============================================================================

#[tokio::test]
async fn test_burst_1000_requests_no_crash() {
    let server = App::new()
        .disable_request_logging()
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();
    let url = server.url("/");

    let mut handles = Vec::new();
    for _ in 0..1000 {
        let c = client.clone();
        let u = url.clone();
        handles.push(tokio::spawn(
            async move { c.get(&u).send().await.unwrap().status() },
        ));
    }

    let mut success = 0u64;
    for h in handles {
        let status = h.await.unwrap();
        assert_eq!(status, 200);
        success += 1;
    }
    assert_eq!(success, 1000);
}

#[tokio::test]
async fn test_burst_with_rate_limit_holds() {
    let server = App::new()
        .disable_request_logging()
        .rate_limit(100, 60)
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();
    let url = server.url("/");

    let mut handles = Vec::new();
    for _ in 0..500 {
        let c = client.clone();
        let u = url.clone();
        handles.push(tokio::spawn(
            async move { c.get(&u).send().await.unwrap().status() },
        ));
    }

    let mut ok = 0u64;
    let mut limited = 0u64;
    for h in handles {
        match h.await.unwrap().as_u16() {
            200 => ok += 1,
            429 => limited += 1,
            other => panic!("unexpected: {other}"),
        }
    }

    assert_eq!(ok, 100, "exactly 100 should pass");
    assert_eq!(limited, 400, "exactly 400 should be rate-limited");
}

// ============================================================================
// 9. Middleware Interaction (stacking)
// ============================================================================

#[tokio::test]
async fn test_cors_plus_rate_limit_plus_timeout_all_active() {
    let server = App::new()
        .disable_request_logging()
        .cors_permissive()
        .rate_limit(5, 60)
        .request_timeout(2)
        .get("/", hello)
        .get("/slow", slow_handler)
        .into_test_server()
        .await;

    let client = Client::new();

    // Normal request: 200 + CORS
    let res = client
        .get(server.url("/"))
        .header("Origin", "http://example.com")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert!(res.headers().contains_key("access-control-allow-origin"));

    // Timeout: 408 + CORS
    let res = client
        .get(server.url("/slow"))
        .header("Origin", "http://example.com")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 408);
    assert!(res.headers().contains_key("access-control-allow-origin"));

    // Exhaust rate limit (4 more, we used 2 already)
    for _ in 0..4 {
        let _ = client.get(server.url("/")).send().await;
    }

    // Rate limited: 429 + CORS
    let res = client
        .get(server.url("/"))
        .header("Origin", "http://example.com")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 429);
    assert!(res.headers().contains_key("access-control-allow-origin"));
}

#[tokio::test]
async fn test_rate_limit_triggers_before_handler() {
    let counter = Arc::new(AtomicU64::new(0));
    let c = counter.clone();

    let server = App::new()
        .disable_request_logging()
        .rate_limit(3, 60)
        .get("/", move || {
            let c = c.clone();
            async move {
                c.fetch_add(1, Ordering::Relaxed);
                "ok"
            }
        })
        .into_test_server()
        .await;

    let client = Client::new();
    for _ in 0..10 {
        let _ = client.get(server.url("/")).send().await;
    }

    assert_eq!(
        counter.load(Ordering::Relaxed),
        3,
        "handler should only execute 3 times — rate limiter should block the rest"
    );
}

// ============================================================================
// 10. Config Override (env vars)
// ============================================================================

#[tokio::test]
async fn test_env_overrides_config() {
    // SAFETY: single-threaded access in this test; no other test reads these vars.
    unsafe {
        std::env::set_var("OXIDE_PORT", "9999");
        std::env::set_var("OXIDE_APP_NAME", "test-app");
    }

    let config = oxide_core::AppConfig::load(None);

    assert_eq!(config.port, 9999);
    assert_eq!(config.app_name, "test-app");

    unsafe {
        std::env::remove_var("OXIDE_PORT");
        std::env::remove_var("OXIDE_APP_NAME");
    }
}

// ============================================================================
// 11. Preflight Not Rate-Limited
// ============================================================================

#[tokio::test]
async fn test_preflight_not_counted_against_rate_limit() {
    let server = App::new()
        .disable_request_logging()
        .cors_permissive()
        .rate_limit(3, 60)
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();

    // Send 5 preflight requests — should NOT consume rate limit
    for _ in 0..5 {
        let res = client
            .request(reqwest::Method::OPTIONS, server.url("/"))
            .header("Origin", "http://example.com")
            .header("Access-Control-Request-Method", "GET")
            .send()
            .await
            .unwrap();
        let s = res.status().as_u16();
        assert!(s == 200 || s == 204);
    }

    // All 3 actual requests should still succeed
    for i in 0..3 {
        let res = client.get(server.url("/")).send().await.unwrap();
        assert_eq!(
            res.status(),
            200,
            "request {i} should succeed — preflights shouldn't count"
        );
    }

    // 4th actual request should be rate-limited
    let res = client.get(server.url("/")).send().await.unwrap();
    assert_eq!(res.status(), 429);
}

// ============================================================================
// KILLER TEST 1: Panic Recovery
// ============================================================================

#[tokio::test]
async fn test_panic_recovery_returns_500() {
    let server = App::new()
        .disable_request_logging()
        .get("/crash", crash_handler)
        .get("/ok", || async { "alive" })
        .into_test_server()
        .await;

    let client = Client::new();

    // Panic should become a JSON 500, NOT crash the server
    let res = client.get(server.url("/crash")).send().await.unwrap();
    assert_eq!(res.status(), 500, "panic should produce 500");
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["status"], 500);
    assert_eq!(body["error"], "internal server error");

    // Server must still be alive after the panic
    let res = client.get(server.url("/ok")).send().await.unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "alive");
}

#[tokio::test]
async fn test_repeated_panics_dont_kill_server() {
    let server = App::new()
        .disable_request_logging()
        .get("/crash", crash_handler)
        .get("/ok", || async { "alive" })
        .into_test_server()
        .await;

    let client = Client::new();

    // Hit the panic endpoint 20 times
    for _ in 0..20 {
        let res = client.get(server.url("/crash")).send().await.unwrap();
        assert_eq!(res.status(), 500);
    }

    // Server still alive
    let res = client.get(server.url("/ok")).send().await.unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "alive");
}

#[tokio::test]
async fn test_panic_gets_cors_headers() {
    let server = App::new()
        .disable_request_logging()
        .cors_permissive()
        .get("/crash", crash_handler)
        .into_test_server()
        .await;

    let client = Client::new();
    let res = client
        .get(server.url("/crash"))
        .header("Origin", "http://example.com")
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 500);
    assert!(
        res.headers().contains_key("access-control-allow-origin"),
        "CORS headers must be present even on panic 500"
    );
}

// ============================================================================
// KILLER TEST 2: IP Spoofing Bypass
// ============================================================================

#[tokio::test]
async fn test_ip_spoofing_with_header_rotation() {
    let server = App::new()
        .disable_request_logging()
        .rate_limit(2, 60)
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();

    // Exhaust limit for fake IP
    for _ in 0..2 {
        let _ = client
            .get(server.url("/"))
            .header("X-Forwarded-For", "1.2.3.4")
            .send()
            .await;
    }
    let res = client
        .get(server.url("/"))
        .header("X-Forwarded-For", "1.2.3.4")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 429, "original IP should be blocked");

    // Rotating to a new fake IP bypasses — this is expected per-IP behavior
    let res = client
        .get(server.url("/"))
        .header("X-Forwarded-For", "5.6.7.8")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "new IP bypasses per-IP rate limit");
}

#[tokio::test]
async fn test_no_header_uses_real_tcp_ip() {
    let server = App::new()
        .disable_request_logging()
        .rate_limit(3, 60)
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();

    // Requests without X-Forwarded-For should use ConnectInfo (127.0.0.1)
    for _ in 0..3 {
        let res = client.get(server.url("/")).send().await.unwrap();
        assert_eq!(res.status(), 200);
    }

    // 4th request from same real IP should be blocked
    let res = client.get(server.url("/")).send().await.unwrap();
    assert_eq!(
        res.status(),
        429,
        "without proxy headers, real TCP IP is used for rate limiting"
    );
}

// ============================================================================
// KILLER TEST 3: Rate Limiter Memory Growth
// ============================================================================

#[tokio::test]
async fn test_rate_limiter_eviction_under_many_unique_ips() {
    let server = App::new()
        .disable_request_logging()
        .rate_limit(1000, 1) // 1-second window so entries expire fast
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();

    // Flood with 500 unique IPs
    for i in 0u32..500 {
        let ip = format!("10.{}.{}.{}", (i >> 16) & 0xFF, (i >> 8) & 0xFF, i & 0xFF);
        let _ = client
            .get(server.url("/"))
            .header("X-Forwarded-For", &ip)
            .send()
            .await;
    }

    // Wait for entries to expire
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Send another batch to trigger periodic eviction
    for i in 500u32..700 {
        let ip = format!("10.{}.{}.{}", (i >> 16) & 0xFF, (i >> 8) & 0xFF, i & 0xFF);
        let res = client
            .get(server.url("/"))
            .header("X-Forwarded-For", &ip)
            .send()
            .await
            .unwrap();
        assert_eq!(
            res.status(),
            200,
            "new IPs after eviction window should succeed"
        );
    }

    // Server still responsive
    let res = reqwest::get(server.url("/")).await.unwrap();
    assert_eq!(res.status(), 200);
}

// ============================================================================
// KILLER TEST 4: Sustained Load (10 seconds, 50 concurrent clients)
// ============================================================================

#[tokio::test]
async fn test_sustained_load_10s_no_crashes() {
    let server = App::new()
        .disable_request_logging()
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();
    let url = server.url("/");
    let duration = std::time::Duration::from_secs(10);
    let start = std::time::Instant::now();

    let total = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));

    let mut handles = Vec::new();
    for _ in 0..50 {
        let c = client.clone();
        let u = url.clone();
        let t = total.clone();
        let e = errors.clone();

        handles.push(tokio::spawn(async move {
            while start.elapsed() < duration {
                match c.get(&u).send().await {
                    Ok(res) => {
                        t.fetch_add(1, Ordering::Relaxed);
                        if !res.status().is_success() {
                            e.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(_) => {
                        e.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let total_reqs = total.load(Ordering::Relaxed);
    let total_errors = errors.load(Ordering::Relaxed);

    assert!(
        total_reqs > 1000,
        "should have handled many requests, got {total_reqs}"
    );
    assert_eq!(
        total_errors, 0,
        "no errors expected under sustained load (got {total_errors}/{total_reqs})"
    );

    // Server still responsive after sustained load
    let res = reqwest::get(server.url("/")).await.unwrap();
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn test_sustained_load_with_rate_limit() {
    let server = App::new()
        .disable_request_logging()
        .rate_limit(10000, 60)
        .get("/", hello)
        .into_test_server()
        .await;

    let client = Client::new();
    let url = server.url("/");
    let duration = std::time::Duration::from_secs(5);
    let start = std::time::Instant::now();

    let ok_count = Arc::new(AtomicU64::new(0));
    let limited_count = Arc::new(AtomicU64::new(0));
    let conn_errors = Arc::new(AtomicU64::new(0));

    let mut handles = Vec::new();
    for _ in 0..20 {
        let c = client.clone();
        let u = url.clone();
        let ok = ok_count.clone();
        let lim = limited_count.clone();
        let err = conn_errors.clone();

        handles.push(tokio::spawn(async move {
            while start.elapsed() < duration {
                match c.get(&u).send().await {
                    Ok(res) => match res.status().as_u16() {
                        200 => {
                            ok.fetch_add(1, Ordering::Relaxed);
                        }
                        429 => {
                            lim.fetch_add(1, Ordering::Relaxed);
                        }
                        other => panic!("unexpected status: {other}"),
                    },
                    Err(_) => {
                        err.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let ok = ok_count.load(Ordering::Relaxed);
    let lim = limited_count.load(Ordering::Relaxed);
    let errs = conn_errors.load(Ordering::Relaxed);

    assert_eq!(errs, 0, "no connection errors expected");
    assert!(ok > 0, "some requests should have succeeded");
    assert_eq!(
        ok, 10000,
        "exactly 10000 requests should pass the rate limiter"
    );
    assert!(lim > 0, "some requests should have been rate-limited");

    // Server still alive
    let res = reqwest::get(server.url("/")).await.unwrap();
    assert!(res.status() == 200 || res.status() == 429);
}

// ============================================================================
// KILLER TEST 5: Middleware Failure Chain
// ============================================================================

#[tokio::test]
async fn test_full_middleware_failure_chain() {
    let server = App::new()
        .disable_request_logging()
        .cors_permissive()
        .rate_limit(3, 60)
        .request_timeout(2)
        .get("/crash", crash_handler)
        .get("/slow", slow_handler)
        .get("/ok", hello)
        .into_test_server()
        .await;

    let client = Client::new();

    // 1. Panic: 500 + CORS headers (uses 1/3 rate limit)
    let res = client
        .get(server.url("/crash"))
        .header("Origin", "http://example.com")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 500, "panic should be 500");
    assert!(
        res.headers().contains_key("access-control-allow-origin"),
        "CORS on panic response"
    );

    // 2. Timeout: 408 + CORS headers (uses 2/3 rate limit)
    let res = client
        .get(server.url("/slow"))
        .header("Origin", "http://example.com")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 408, "slow handler should timeout");
    assert!(
        res.headers().contains_key("access-control-allow-origin"),
        "CORS on timeout response"
    );

    // 3. Normal: 200 + CORS (uses 3/3 rate limit)
    let res = client
        .get(server.url("/ok"))
        .header("Origin", "http://example.com")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "normal request should succeed");
    assert!(
        res.headers().contains_key("access-control-allow-origin"),
        "CORS on success response"
    );

    // 4. Rate limited: 429 + CORS (all 3 spent)
    let res = client
        .get(server.url("/ok"))
        .header("Origin", "http://example.com")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 429, "should be rate limited");
    assert!(
        res.headers().contains_key("access-control-allow-origin"),
        "CORS on rate-limited response"
    );

    // 5. Server still alive — use different IP to bypass rate limit
    let res = client
        .get(server.url("/ok"))
        .header("X-Forwarded-For", "bypass-ip")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "server must survive the full failure chain");
}

#[tokio::test]
async fn test_concurrent_panics_and_normal_requests() {
    let server = App::new()
        .disable_request_logging()
        .get("/crash", crash_handler)
        .get("/ok", || async { "alive" })
        .into_test_server()
        .await;

    let client = Client::new();
    let mut handles = Vec::new();

    // 50 panic requests + 50 normal requests — all concurrent
    for i in 0..100u32 {
        let c = client.clone();
        let path = if i % 2 == 0 { "/crash" } else { "/ok" };
        let url = server.url(path);
        handles.push(tokio::spawn(async move {
            let res = c.get(&url).send().await.unwrap();
            (path, res.status().as_u16())
        }));
    }

    let mut ok_200 = 0u32;
    let mut crash_500 = 0u32;

    for h in handles {
        let (path, status) = h.await.unwrap();
        match (path, status) {
            ("/ok", 200) => ok_200 += 1,
            ("/crash", 500) => crash_500 += 1,
            (p, s) => panic!("unexpected: {p} returned {s}"),
        }
    }

    assert_eq!(crash_500, 50, "all panic requests should return 500");
    assert_eq!(ok_200, 50, "all normal requests should return 200");
}
