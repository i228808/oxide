use async_trait::async_trait;
use oxide_framework_core::{
    ApiResponse, App, FrameworkError, ReadinessCheck, RequestId, Validated,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug)]
struct AlwaysFail;

#[async_trait]
impl ReadinessCheck for AlwaysFail {
    fn name(&self) -> &'static str {
        "always_fail"
    }

    async fn check(&self) -> Result<(), FrameworkError> {
        Err(FrameworkError::Internal("boom".into()))
    }
}

#[tokio::test]
async fn default_health_routes_are_available() {
    let server = App::new()
        .disable_request_logging()
        .into_test_server()
        .await;

    let live = reqwest::get(server.url("/health/live")).await.unwrap();
    assert_eq!(live.status(), 200);

    let ready = reqwest::get(server.url("/health/ready")).await.unwrap();
    assert_eq!(ready.status(), 200);
}

#[tokio::test]
async fn readiness_returns_503_on_failed_check() {
    let server = App::new()
        .disable_request_logging()
        .readiness_check(AlwaysFail)
        .into_test_server()
        .await;

    let ready = reqwest::get(server.url("/health/ready")).await.unwrap();
    assert_eq!(ready.status(), 503);
    let body: serde_json::Value = ready.json().await.unwrap();
    assert_eq!(body["code"], "readiness_failed");
    assert!(body["failures"].is_array());
}

#[tokio::test]
async fn default_health_routes_can_be_disabled() {
    let server = App::new()
        .disable_request_logging()
        .disable_default_health_routes()
        .into_test_server()
        .await;

    let live = reqwest::get(server.url("/health/live")).await.unwrap();
    assert_eq!(live.status(), 404);
}

#[tokio::test]
async fn request_id_is_generated_and_echoed() {
    async fn handler(RequestId(id): RequestId) -> ApiResponse<String> {
        ApiResponse::ok(id)
    }

    let server = App::new()
        .disable_request_logging()
        .get("/id", handler)
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/id")).await.unwrap();
    assert_eq!(res.status(), 200);
    let header = res
        .headers()
        .get("x-request-id")
        .expect("x-request-id missing")
        .to_str()
        .unwrap()
        .to_string();
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["data"], header);
}

#[tokio::test]
async fn request_id_respects_incoming_header() {
    async fn handler(RequestId(id): RequestId) -> ApiResponse<String> {
        ApiResponse::ok(id)
    }

    let server = App::new()
        .disable_request_logging()
        .get("/id", handler)
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let res = client
        .get(server.url("/id"))
        .header("x-request-id", "custom-req-id")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let header = res
        .headers()
        .get("x-request-id")
        .expect("x-request-id missing")
        .to_str()
        .unwrap();
    assert_eq!(header, "custom-req-id");
}

#[derive(Debug, Deserialize, Serialize, Validate)]
struct CreateUser {
    #[validate(length(min = 3))]
    name: String,
}

async fn create_user(Validated(payload): Validated<CreateUser>) -> ApiResponse<String> {
    ApiResponse::ok(payload.name)
}

#[tokio::test]
async fn validated_accepts_valid_payload() {
    let server = App::new()
        .disable_request_logging()
        .post("/users", create_user)
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let res = client
        .post(server.url("/users"))
        .json(&serde_json::json!({"name": "alice"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn validated_rejects_invalid_payload_with_typed_error() {
    let server = App::new()
        .disable_request_logging()
        .post("/users", create_user)
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let res = client
        .post(server.url("/users"))
        .json(&serde_json::json!({"name": "ab"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["code"], "validation_error");
    assert!(body.get("details").is_some());
}
