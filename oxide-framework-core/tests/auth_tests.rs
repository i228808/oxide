//! JWT auth, session cookie, and role guards.

use oxide_framework_core::{
    encode_token, ApiResponse, App, AuthClaims, AuthConfig, Authenticated, OptionalAuth, RequireRole,
    RoleName,
};
use serde::Serialize;

const SECRET: &[u8] = b"test-secret-key-for-auth-tests-only";

#[derive(Serialize)]
struct Msg {
    text: String,
}

fn app_with_auth() -> App {
    App::new()
        .disable_request_logging()
        .auth(AuthConfig::new(SECRET))
}

fn token_for(sub: &str, roles: &[&str]) -> String {
    let roles: Vec<String> = roles.iter().map(|s| (*s).to_string()).collect();
    let claims = AuthClaims::new(sub, roles, 3600);
    encode_token(&claims, SECRET).unwrap()
}

#[tokio::test]
async fn bearer_token_sets_claims() {
    let server = app_with_auth()
        .get("/who", |OptionalAuth(opt): OptionalAuth| async move {
            let c = opt.expect("claims");
            ApiResponse::ok(Msg { text: c.sub.clone() })
        })
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let res = client
        .get(server.url("/who"))
        .header("Authorization", format!("Bearer {}", token_for("alice", &["user"])))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let v: serde_json::Value = res.json().await.unwrap();
    assert_eq!(v["data"]["text"], "alice");
}

#[tokio::test]
async fn missing_token_is_anonymous() {
    let server = app_with_auth()
        .get("/opt", |OptionalAuth(opt): OptionalAuth| async move {
            ApiResponse::ok(Msg {
                text: opt.map(|c| c.sub).unwrap_or_else(|| "anon".into()),
            })
        })
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let res = client.get(server.url("/opt")).send().await.unwrap();
    assert_eq!(res.status(), 200);
    let v: serde_json::Value = res.json().await.unwrap();
    assert_eq!(v["data"]["text"], "anon");
}

#[tokio::test]
async fn invalid_bearer_returns_401() {
    let server = app_with_auth()
        .get("/", || async { ApiResponse::ok(()) })
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let res = client
        .get(server.url("/"))
        .header("Authorization", "Bearer not-a-valid-jwt")
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn session_cookie_auth() {
    let server = App::new()
        .disable_request_logging()
        .auth(
            AuthConfig::new(SECRET).with_session_cookie("oxide_session"),
        )
        .get("/who", |OptionalAuth(opt): OptionalAuth| async move {
            ApiResponse::ok(Msg {
                text: opt.map(|c| c.sub).unwrap_or_default(),
            })
        })
        .into_test_server()
        .await;

    let tok = token_for("bob", &[]);
    let client = reqwest::Client::new();
    let res = client
        .get(server.url("/who"))
        .header("Cookie", format!("oxide_session={tok}"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let v: serde_json::Value = res.json().await.unwrap();
    assert_eq!(v["data"]["text"], "bob");
}

struct AdminRole;
impl RoleName for AdminRole {
    const ROLE: &'static str = "admin";
}

#[tokio::test]
async fn require_role_allows() {
    let server = app_with_auth()
        .get(
            "/admin",
            |_r: RequireRole<AdminRole>, Authenticated(c): Authenticated| async move {
                ApiResponse::ok(Msg { text: c.sub })
            },
        )
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let res = client
        .get(server.url("/admin"))
        .header(
            "Authorization",
            format!("Bearer {}", token_for("root", &["admin"])),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn require_role_forbids() {
    let server = app_with_auth()
        .get(
            "/admin",
            |_r: RequireRole<AdminRole>| async move { ApiResponse::ok(()) },
        )
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let res = client
        .get(server.url("/admin"))
        .header(
            "Authorization",
            format!("Bearer {}", token_for("user", &["user"])),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn authenticated_extractor_requires_login() {
    let server = app_with_auth()
        .get("/me", |Authenticated(c): Authenticated| async move {
            ApiResponse::ok(Msg { text: c.sub })
        })
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let res = client.get(server.url("/me")).send().await.unwrap();
    assert_eq!(res.status(), 401);
}

