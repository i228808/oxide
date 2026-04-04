use oxide_framework_core::{App, ApiResponse, Data};
use oxide_framework_supabase::{AppSupabaseExt, SupabaseConfig};
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpListener;

fn start_mock_server(status_line: &str, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let status_line = status_line.to_string();

    std::thread::spawn(move || {
        for mut stream in listener.incoming().take(8).flatten() {
            let mut buf = [0_u8; 4096];
            let _ = stream.read(&mut buf);
            let response = format!(
                "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });

    format!("http://{}", addr)
}

#[tokio::test]
async fn strict_supabase_unhealthy_returns_503() {
    let base_url = start_mock_server("HTTP/1.1 500 Internal Server Error", r#"{"msg":"down"}"#);

    let server = App::new()
        .disable_request_logging()
        .supabase(SupabaseConfig::new(base_url, "key").strict(true))
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/health/ready")).await.unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["code"], "readiness_failed");
}

#[tokio::test]
async fn lazy_supabase_keeps_ready_200() {
    let base_url = start_mock_server("HTTP/1.1 500 Internal Server Error", r#"{"msg":"down"}"#);

    let server = App::new()
        .disable_request_logging()
        .supabase(SupabaseConfig::new(base_url, "key").strict(false))
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/health/ready")).await.unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::OK);
}

#[tokio::test]
async fn select_works_through_injected_client() {
    let body = r#"[{"id":1,"name":"alice"}]"#;
    let base_url = start_mock_server("HTTP/1.1 200 OK", body);

    async fn handler(
        Data(client): Data<oxide_framework_supabase::SupabaseClient>,
    ) -> ApiResponse<serde_json::Value> {
        let data = client
            .select("users", &[("select", "*")])
            .await
            .unwrap();
        ApiResponse::ok(data)
    }

    let server = App::new()
        .disable_request_logging()
        .supabase(SupabaseConfig::new(base_url, "key"))
        .get("/users", handler)
        .into_test_server()
        .await;

    let res = reqwest::get(server.url("/users")).await.unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["data"], json!([{"id": 1, "name": "alice"}]));
}
