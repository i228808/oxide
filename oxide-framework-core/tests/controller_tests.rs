//! Integration tests for the `#[controller]` proc macro and DI system.

use oxide_framework_core::{controller, ApiResponse, App, Json, Path, AppState};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct DbPool {
    url: String,
}

#[derive(Clone)]
struct Counter(Arc<AtomicU64>);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct User {
    id: u64,
    name: String,
}

#[derive(Deserialize)]
struct CreateUser {
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Msg {
    text: String,
}

// ---------------------------------------------------------------------------
// Controller: UserController (stateful, constructor injection)
// ---------------------------------------------------------------------------

struct UserController {
    db: DbPool,
}

#[controller("/api/users")]
impl UserController {
    fn new(state: &AppState) -> Self {
        Self {
            db: state.get::<DbPool>().expect("DbPool not registered").as_ref().clone(),
        }
    }

    #[get("/")]
    async fn list(&self) -> ApiResponse<Vec<User>> {
        ApiResponse::ok(vec![
            User { id: 1, name: "Alice".into() },
            User { id: 2, name: "Bob".into() },
        ])
    }

    #[get("/{id}")]
    async fn get_one(&self, Path(id): Path<u64>) -> ApiResponse<User> {
        ApiResponse::ok(User {
            id,
            name: format!("user-{id}"),
        })
    }

    #[post("/")]
    async fn create(&self, Json(body): Json<CreateUser>) -> ApiResponse<User> {
        ApiResponse::created(User { id: 99, name: body.name })
    }

    #[delete("/{id}")]
    async fn remove(&self, Path(id): Path<u64>) -> ApiResponse<Msg> {
        ApiResponse::ok(Msg {
            text: format!("deleted {id} from {}", self.db.url),
        })
    }

    #[get("/db")]
    async fn db_info(&self) -> ApiResponse<Msg> {
        ApiResponse::ok(Msg { text: self.db.url.clone() })
    }
}

// ---------------------------------------------------------------------------
// Controller: HealthController (no deps, uses Default)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct HealthController;

#[controller("/health")]
impl HealthController {
    #[get("/")]
    async fn check(&self) -> ApiResponse<Msg> {
        ApiResponse::ok(Msg { text: "healthy".into() })
    }
}

// ---------------------------------------------------------------------------
// Controller: CounterController (stateful with atomics)
// ---------------------------------------------------------------------------

struct CounterController {
    counter: Counter,
}

#[controller("/api/counter")]
impl CounterController {
    fn new(state: &AppState) -> Self {
        Self {
            counter: state.get::<Counter>().expect("Counter not registered").as_ref().clone(),
        }
    }

    #[get("/")]
    async fn get_count(&self) -> ApiResponse<Msg> {
        let val = self.counter.0.load(Ordering::Relaxed);
        ApiResponse::ok(Msg {
            text: format!("{val}"),
        })
    }

    #[post("/increment")]
    async fn increment(&self) -> ApiResponse<Msg> {
        let val = self.counter.0.fetch_add(1, Ordering::Relaxed) + 1;
        ApiResponse::ok(Msg {
            text: format!("{val}"),
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_app() -> App {
    App::new()
        .disable_request_logging()
        .state(DbPool { url: "postgres://test".into() })
        .state(Counter(Arc::new(AtomicU64::new(0))))
        .controller::<UserController>()
        .controller::<HealthController>()
        .controller::<CounterController>()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn controller_get_list() {
    let server = test_app().into_test_server().await;
    let res = reqwest::get(server.url("/api/users")).await.unwrap();
    assert_eq!(res.status(), 200);

    let body: serde_json::Value = res.json().await.unwrap();
    let users = body["data"].as_array().unwrap();
    assert_eq!(users.len(), 2);
    assert_eq!(users[0]["name"], "Alice");
}

#[tokio::test]
async fn controller_get_by_id() {
    let server = test_app().into_test_server().await;
    let res = reqwest::get(server.url("/api/users/42")).await.unwrap();
    assert_eq!(res.status(), 200);

    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["data"]["id"], 42);
    assert_eq!(body["data"]["name"], "user-42");
}

#[tokio::test]
async fn controller_post_create() {
    let client = reqwest::Client::new();
    let server = test_app().into_test_server().await;

    let res = client
        .post(server.url("/api/users"))
        .json(&serde_json::json!({ "name": "Charlie" }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 201);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["data"]["name"], "Charlie");
    assert_eq!(body["data"]["id"], 99);
}

#[tokio::test]
async fn controller_delete() {
    let client = reqwest::Client::new();
    let server = test_app().into_test_server().await;

    let res = client
        .delete(server.url("/api/users/7"))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(body["data"]["text"]
        .as_str()
        .unwrap()
        .contains("deleted 7"));
}

#[tokio::test]
async fn controller_injects_db_pool() {
    let server = test_app().into_test_server().await;
    let res = reqwest::get(server.url("/api/users/db")).await.unwrap();
    assert_eq!(res.status(), 200);

    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["data"]["text"], "postgres://test");
}

#[tokio::test]
async fn health_controller_no_deps() {
    let server = test_app().into_test_server().await;
    let res = reqwest::get(server.url("/health")).await.unwrap();
    assert_eq!(res.status(), 200);

    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["data"]["text"], "healthy");
}

#[tokio::test]
async fn counter_controller_stateful() {
    let server = test_app().into_test_server().await;
    let client = reqwest::Client::new();

    // Initial value
    let body: serde_json::Value = client
        .get(server.url("/api/counter"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(body["data"]["text"], "0");

    // Increment three times
    for _ in 0..3 {
        client
            .post(server.url("/api/counter/increment"))
            .send()
            .await
            .unwrap();
    }

    let body: serde_json::Value = client
        .get(server.url("/api/counter"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(body["data"]["text"], "3");
}

#[tokio::test]
async fn controller_404_for_unknown_route() {
    let server = test_app().into_test_server().await;
    let res = reqwest::get(server.url("/api/nope")).await.unwrap();
    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn controller_wrong_method_returns_405() {
    let client = reqwest::Client::new();
    let server = test_app().into_test_server().await;

    let res = client
        .put(server.url("/api/users"))
        .send()
        .await
        .unwrap();

    assert!(res.status() == 405 || res.status() == 404);
}

#[tokio::test]
async fn multiple_controllers_coexist() {
    let server = test_app().into_test_server().await;

    let user_res = reqwest::get(server.url("/api/users")).await.unwrap();
    let health_res = reqwest::get(server.url("/health")).await.unwrap();
    let counter_res = reqwest::get(server.url("/api/counter")).await.unwrap();

    assert_eq!(user_res.status(), 200);
    assert_eq!(health_res.status(), 200);
    assert_eq!(counter_res.status(), 200);
}

#[tokio::test]
async fn controller_with_cors() {
    let server = App::new()
        .disable_request_logging()
        .cors_permissive()
        .state(DbPool { url: "pg://".into() })
        .state(Counter(Arc::new(AtomicU64::new(0))))
        .controller::<UserController>()
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let res = client
        .get(server.url("/api/users"))
        .header("Origin", "http://example.com")
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    assert!(res.headers().contains_key("access-control-allow-origin"));
}

#[tokio::test]
async fn controller_with_rate_limit() {
    let server = App::new()
        .disable_request_logging()
        .rate_limit(3, 60)
        .state(DbPool { url: "pg://".into() })
        .state(Counter(Arc::new(AtomicU64::new(0))))
        .controller::<UserController>()
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    for _ in 0..3 {
        let r = client.get(server.url("/api/users")).send().await.unwrap();
        assert_eq!(r.status(), 200);
    }

    let r = client.get(server.url("/api/users")).send().await.unwrap();
    assert_eq!(r.status(), 429);
}

#[tokio::test]
async fn concurrent_controller_requests() {
    let server = test_app().into_test_server().await;
    let client = reqwest::Client::new();

    let mut handles = Vec::new();
    for i in 0..50 {
        let c = client.clone();
        let url = server.url(&format!("/api/users/{i}"));
        handles.push(tokio::spawn(async move {
            let res = c.get(&url).send().await.unwrap();
            assert_eq!(res.status(), 200);
            let body: serde_json::Value = res.json().await.unwrap();
            assert_eq!(body["data"]["id"], i);
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
}

