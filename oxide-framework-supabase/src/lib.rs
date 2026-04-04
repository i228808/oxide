use async_trait::async_trait;
use oxide_framework_core::{App, FrameworkError, ReadinessCheck};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct SupabaseConfig {
    pub base_url: String,
    pub api_key: String,
    pub schema: String,
    pub timeout_secs: u64,
    pub strict: bool,
}

impl SupabaseConfig {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            schema: "public".to_string(),
            timeout_secs: 10,
            strict: false,
        }
    }

    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = schema.into();
        self
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    pub fn strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }
}

#[derive(Clone)]
pub struct SupabaseClient {
    cfg: SupabaseConfig,
    http: reqwest::Client,
}

impl SupabaseClient {
    pub fn new(cfg: SupabaseConfig) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            "apikey",
            HeaderValue::from_str(&cfg.api_key).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", cfg.api_key))
                .unwrap_or_else(|_| HeaderValue::from_static("Bearer")),
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(cfg.timeout_secs))
            .build()
            .expect("failed to build reqwest client for supabase");

        Self { cfg, http }
    }

    pub async fn health_check(&self) -> Result<(), FrameworkError> {
        let url = format!("{}/rest/v1/", self.cfg.base_url);
        let res = self
            .http
            .get(url)
            .header("accept-profile", &self.cfg.schema)
            .send()
            .await
            .map_err(|e| FrameworkError::ReadinessFailed {
                check: "supabase",
                message: e.to_string(),
            })?;

        if res.status().is_success() {
            Ok(())
        } else {
            Err(FrameworkError::ReadinessFailed {
                check: "supabase",
                message: format!("unexpected status {}", res.status()),
            })
        }
    }

    pub async fn select(&self, table: &str, query: &[(&str, &str)]) -> Result<Value, FrameworkError> {
        let url = format!("{}/rest/v1/{}", self.cfg.base_url, table);
        let res = self
            .http
            .get(url)
            .header("accept-profile", &self.cfg.schema)
            .query(query)
            .send()
            .await
            .map_err(|e| FrameworkError::Internal(e.to_string()))?;

        res.json::<Value>()
            .await
            .map_err(|e| FrameworkError::Internal(e.to_string()))
    }

    pub async fn insert<T: Serialize>(&self, table: &str, payload: &T) -> Result<Value, FrameworkError> {
        let url = format!("{}/rest/v1/{}", self.cfg.base_url, table);
        let res = self
            .http
            .post(url)
            .header("content-profile", &self.cfg.schema)
            .header("prefer", "return=representation")
            .json(payload)
            .send()
            .await
            .map_err(|e| FrameworkError::Internal(e.to_string()))?;

        res.json::<Value>()
            .await
            .map_err(|e| FrameworkError::Internal(e.to_string()))
    }

    pub async fn rpc<T: Serialize>(&self, function: &str, payload: &T) -> Result<Value, FrameworkError> {
        let url = format!("{}/rest/v1/rpc/{}", self.cfg.base_url, function);
        let res = self
            .http
            .post(url)
            .header("content-profile", &self.cfg.schema)
            .json(payload)
            .send()
            .await
            .map_err(|e| FrameworkError::Internal(e.to_string()))?;

        res.json::<Value>()
            .await
            .map_err(|e| FrameworkError::Internal(e.to_string()))
    }
}

#[derive(Clone)]
struct SupabaseReady(SupabaseClient);

#[async_trait]
impl ReadinessCheck for SupabaseReady {
    fn name(&self) -> &'static str {
        "supabase"
    }

    async fn check(&self) -> Result<(), FrameworkError> {
        self.0.health_check().await
    }
}

pub trait AppSupabaseExt {
    fn supabase(self, config: SupabaseConfig) -> Self;
}

impl AppSupabaseExt for App {
    fn supabase(self, config: SupabaseConfig) -> Self {
        let strict = config.strict;
        let client = SupabaseClient::new(config);
        let app = self.state(client.clone());
        if strict {
            app.readiness_check(SupabaseReady(client))
        } else {
            app
        }
    }
}
