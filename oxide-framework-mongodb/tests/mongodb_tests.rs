use oxide_framework_core::{App, Data};
use oxide_framework_mongodb::{AppMongoExt, MongoConfig, MongoHandle};

#[tokio::test]
async fn lazy_mongodb_keeps_ready_200_when_unreachable() {
    let server = App::new()
        .disable_request_logging()
        .mongodb(MongoConfig::new("mongodb://127.0.0.1:1", "app").strict(false))
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/health/ready")).await.unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::OK);
}

#[tokio::test]
async fn strict_mongodb_reports_not_ready_when_unreachable() {
    let server = App::new()
        .disable_request_logging()
        .mongodb(MongoConfig::new("mongodb://127.0.0.1:1", "app").strict(true))
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/health/ready")).await.unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["code"], "readiness_failed");
}

#[tokio::test]
async fn mongo_handle_is_injected_even_when_lazy() {
    async fn has_handle(Data(_h): Data<MongoHandle>) -> &'static str {
        "ok"
    }

    let server = App::new()
        .disable_request_logging()
        .mongodb(MongoConfig::new("mongodb://127.0.0.1:1", "app").strict(false))
        .get("/h", has_handle)
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/h")).await.unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::OK);
}
