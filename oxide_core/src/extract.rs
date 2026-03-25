use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use std::sync::Arc;

use crate::config::AppConfig;
use crate::state::AppState;

/// Extractor for the application configuration.
///
/// # Example
///
/// ```rust,ignore
/// use oxide_core::{ApiResponse, Config};
///
/// async fn handler(Config(cfg): Config) -> ApiResponse<String> {
///     ApiResponse::ok(format!("Welcome to {}", cfg.app_name))
/// }
/// ```
pub struct Config(pub Arc<AppConfig>);

impl<S: Send + Sync> FromRequestParts<S> for Config {
    type Rejection = StateNotFound;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AppState>()
            .map(|s| Config(s.config.clone()))
            .ok_or(StateNotFound("AppConfig"))
    }
}

/// Extractor for user-provided state registered via [`App::state()`](crate::App::state).
///
/// Returns `Arc<T>` so the data can be shared cheaply across handlers.
///
/// # Example
///
/// ```rust,ignore
/// use oxide_core::{ApiResponse, Data};
/// use std::sync::Arc;
///
/// struct DbPool { /* ... */ }
///
/// async fn handler(Data(pool): Data<DbPool>) -> ApiResponse<String> {
///     ApiResponse::ok("connected".into())
/// }
/// ```
pub struct Data<T: Send + Sync + 'static>(pub Arc<T>);

impl<S: Send + Sync, T: Send + Sync + 'static> FromRequestParts<S> for Data<T> {
    type Rejection = StateNotFound;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let app_state = parts
            .extensions
            .get::<AppState>()
            .ok_or(StateNotFound("AppState"))?;

        app_state
            .get::<T>()
            .map(Data)
            .ok_or(StateNotFound(std::any::type_name::<T>()))
    }
}

/// Ergonomic alias for [`Data<T>`] — intended for use inside controllers.
///
/// Semantically identical to `Data`, but reads more naturally in constructor
/// injection code:
///
/// ```rust,ignore
/// fn new(state: &AppState) -> Self {
///     Self {
///         pool: state.get::<DbPool>().expect("DbPool missing").as_ref().clone(),
///     }
/// }
///
/// #[get("/")]
/// async fn index(&self, Inject(cache): Inject<Cache>) -> ApiResponse<String> {
///     // ...
/// }
/// ```
pub struct Inject<T: Send + Sync + 'static>(pub Arc<T>);

impl<S: Send + Sync, T: Send + Sync + 'static> FromRequestParts<S> for Inject<T> {
    type Rejection = StateNotFound;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let app_state = parts
            .extensions
            .get::<AppState>()
            .ok_or(StateNotFound("AppState"))?;

        app_state
            .get::<T>()
            .map(Inject)
            .ok_or(StateNotFound(std::any::type_name::<T>()))
    }
}

/// Rejection returned when requested state is missing.
#[derive(Debug)]
pub struct StateNotFound(pub &'static str);

impl std::fmt::Display for StateNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "state not found: {}", self.0)
    }
}

impl std::error::Error for StateNotFound {}

impl IntoResponse for StateNotFound {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("internal error: missing state ({})", self.0),
        )
            .into_response()
    }
}
