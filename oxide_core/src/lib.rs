mod app;
mod config;
mod controller;
mod extract;
mod logging;
pub mod middleware;
mod rate_limit;
mod response;
mod router;
mod state;

pub use app::{App, TestServer};
pub use config::AppConfig;
pub use controller::Controller;
pub use extract::{Config, Data, Inject};
pub use response::ApiResponse;
pub use router::{Method, OxideRouter};
pub use state::AppState;

pub use axum::extract::Path;
pub use axum::http::StatusCode;
pub use axum::Json;

// Re-export proc macro so users only need `use oxide_core::controller;`
pub use oxide_macros::controller;
