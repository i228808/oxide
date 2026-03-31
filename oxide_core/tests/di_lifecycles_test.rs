use oxide_core::{App, ApiResponse, Data};
use reqwest::StatusCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// We need a way to differentiate Singleton and Request-Scoped DI

// 1. Singleton state
struct GlobalCounter(AtomicUsize);

// 2. Request-scoped state (created anew per request)
#[derive(Clone)]
struct RequestId(usize);

async fn handler(
    Data(global): Data<Arc<GlobalCounter>>,
    // Proposed extractor for request-scoped dependencies
    // Scoped<T> fails to extract if T was not injected into this specific request
    oxide_core::Scoped(req_id): oxide_core::Scoped<RequestId>,
) -> ApiResponse<String> {
    let current_global = global.0.fetch_add(1, Ordering::SeqCst);
    ApiResponse::ok(format!("global: {}, request: {}", current_global, req_id.0))
}

#[tokio::test]
async fn test_singleton_vs_scoped_lifecycles() {
    let global_counter = Arc::new(GlobalCounter(AtomicUsize::new(0)));
    let request_counter = Arc::new(AtomicUsize::new(100));

    let server = App::new()
        // Singleton injection (already exists as app.state)
        .state(global_counter)
        // Proposed API: Request-scoped factory
        // This closure runs on *every request* and injects the returned value
        // into the request's extension map, extractable via `Scoped<T>`.
        .scoped_state(move |_req| {
            let rc = request_counter.clone();
            async move { RequestId(rc.fetch_add(1, Ordering::SeqCst)) }
        })
        .get("/scopes", handler)
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    
    // First request
    let res1 = client.get(server.url("/scopes")).send().await.unwrap();
    assert_eq!(res1.status(), StatusCode::OK);
    let body1: serde_json::Value = res1.json().await.unwrap();
    assert_eq!(body1["data"], "global: 0, request: 100");

    // Second request
    let res2 = client.get(server.url("/scopes")).send().await.unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
    let body2: serde_json::Value = res2.json().await.unwrap();
    assert_eq!(body2["data"], "global: 1, request: 101");
}

#[tokio::test]
async fn test_missing_scoped_dependency_fails_gracefully() {
    // If a handler demands `Scoped<T>` but it was never provided,
    // the framework should return a 500 cleanly, not panic and crash the server.

    async fn bad_handler(_missing: oxide_core::Scoped<String>) -> ApiResponse<()> {
        ApiResponse::ok(())
    }

    let server = App::new()
        .get("/bad", bad_handler)
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let res = client.get(server.url("/bad")).send().await.unwrap();
    
    // Framework should catch the missing extraction and return 500
    assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
