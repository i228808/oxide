use async_trait::async_trait;
use mongodb::{Client, Database};
use oxide_framework_core::{App, FrameworkError, ReadinessCheck};

#[derive(Clone, Debug)]
pub struct MongoConfig {
    pub uri: String,
    pub database: String,
    pub strict: bool,
}

impl MongoConfig {
    pub fn new(uri: impl Into<String>, database: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            database: database.into(),
            strict: false,
        }
    }

    pub fn strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }
}

#[derive(Clone)]
pub struct MongoHandle {
    pub client: Client,
    pub db: Database,
}

impl MongoHandle {
    pub async fn connect(config: &MongoConfig) -> Result<Self, FrameworkError> {
        let client = Client::with_uri_str(&config.uri)
            .await
            .map_err(|e| FrameworkError::Internal(format!("mongodb connect failed: {e}")))?;
        let db = client.database(&config.database);
        Ok(Self { client, db })
    }

    pub async fn ping(&self) -> Result<(), FrameworkError> {
        self.db
            .run_command(mongodb::bson::doc! { "ping": 1 })
            .await
            .map(|_| ())
            .map_err(|e| FrameworkError::ReadinessFailed {
                check: "mongodb",
                message: e.to_string(),
            })
    }
}

#[derive(Clone)]
struct MongoReady(MongoHandle);

#[async_trait]
impl ReadinessCheck for MongoReady {
    fn name(&self) -> &'static str {
        "mongodb"
    }

    async fn check(&self) -> Result<(), FrameworkError> {
        self.0.ping().await
    }
}

pub trait AppMongoExt {
    fn mongodb(self, config: MongoConfig) -> Self;
}

impl AppMongoExt for App {
    fn mongodb(self, config: MongoConfig) -> Self {
        let strict = config.strict;
        let rt = tokio::runtime::Runtime::new().expect("failed to create runtime for mongodb init");
        let handle = rt
            .block_on(MongoHandle::connect(&config))
            .expect("failed to initialize mongodb client");

        let app = self.state(handle.clone());
        if strict {
            app.readiness_check(MongoReady(handle))
        } else {
            app
        }
    }
}
