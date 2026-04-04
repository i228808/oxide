use oxide_framework_core::{App, ApiResponse, Data};
use oxide_framework_db::{AppDbExt, ConnectMode, Sqlite};
use reqwest::StatusCode;
use std::sync::Arc;
use tokio::task::JoinSet;

// A basic handler that uses the injected database pool
// We will test if the DI system correctly provides the connection
async fn get_user(Data(db): Data<oxide_framework_db::DbPool<Sqlite>>) -> ApiResponse<String> {
    // Attempt a basic query to prove the connection works
    let result: (String,) = sqlx::query_as("SELECT 'Hello Oxide'")
        .fetch_one(&*db)
        .await
        .unwrap();
    ApiResponse::ok(result.0)
}

#[tokio::test]
async fn test_database_strict_mode_still_works_with_sqlite() {
    let server = App::new()
        .database_with_mode::<Sqlite>(
            "sqlite::memory:",
            ConnectMode::Strict,
            |opts: sqlx::pool::PoolOptions<Sqlite>| opts.max_connections(3),
        )
        .get("/db-test", get_user)
        .into_test_server()
        .await;

    let client = reqwest::Client::new();
    let res = client.get(server.url("/db-test")).send().await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["data"], "Hello Oxide");
}

#[tokio::test]
async fn test_database_injection_and_query() {
    // 1. Arrange: Spin up server with in-memory SQLite
    let server = App::new()
        // Here is the expected API design:
        // Automatically creates a pool and registers it in the DI container
        .database::<Sqlite>("sqlite::memory:", |opts: sqlx::pool::PoolOptions<Sqlite>| opts.max_connections(5))
        .get("/db-test", get_user)
        .into_test_server()
        .await;

    // 2. Act: Make a request
    let client = reqwest::Client::new();
    let res = client.get(server.url("/db-test")).send().await.unwrap();

    // 3. Assert
    assert_eq!(res.status(), StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["data"], "Hello Oxide");
}

#[tokio::test]
async fn test_db_concurrency_and_pool_limits() {
    // Edge/Concurrency test: Fire 100 concurrent requests against a pool of size 2.
    // Proves deterministic behavior under load.
    
    let server = App::new()
        .database::<Sqlite>("sqlite::memory:", |opts: sqlx::pool::PoolOptions<Sqlite>| opts.max_connections(2))
        .get("/db-test", get_user)
        .into_test_server()
        .await;

    let server = Arc::new(server);
    let client = reqwest::Client::new();
    let mut tasks = JoinSet::new();

    for _ in 0..100 {
        let server_clone = server.clone();
        let client_clone = client.clone();
        tasks.spawn(async move {
            let res = client_clone.get(server_clone.url("/db-test")).send().await.unwrap();
            assert_eq!(res.status(), StatusCode::OK);
        });
    }

    // Wait for all requests to finish successfully without Panics
    while let Some(res) = tasks.join_next().await {
        res.unwrap();
    }
}

