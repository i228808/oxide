mod app;
pub mod auth;
mod config;
mod controller;
mod error;
mod extract;
mod logging;
pub mod middleware;
mod rate_limit;
mod response;
mod router;
mod state;

pub use app::{App, ReadinessCheck, TestServer};
pub use config::AppConfig;
pub use controller::Controller;
pub use auth::{
    encode_token, AuthClaims, AuthConfig, AuthLayer, AuthRejection, Authenticated, OptionalAuth,
    RequireRole, RoleName,
};
pub use error::FrameworkError;
pub use extract::{Config, Data, Inject, RequestId, Scoped, Validated};
pub use response::ApiResponse;
pub use router::{Method, OxideRouter};
pub use state::AppState;

pub use axum::extract::Path;
pub use axum::http::StatusCode;
pub use axum::Json;

// Re-export proc macro so users only need `use oxide_framework_core::controller;`
pub use oxide_framework_macros::controller;

