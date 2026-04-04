use axum::extract::{FromRequest, FromRequestParts};
use axum::http::request::Parts;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use validator::Validate;

use crate::config::AppConfig;
use crate::error::FrameworkError;
use crate::state::AppState;

/// Extractor for the application configuration.
///
/// # Example
///
/// ```rust,ignore
/// use oxide_framework_core::{ApiResponse, Config};
///
/// async fn handler(Config(cfg): Config) -> ApiResponse<String> {
///     ApiResponse::ok(format!("Welcome to {}", cfg.app_name))
/// }
/// ```
pub struct Config(pub Arc<AppConfig>);

impl<S: Send + Sync> FromRequestParts<S> for Config {
    type Rejection = FrameworkError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AppState>()
            .map(|s| Config(s.config.clone()))
            .ok_or(FrameworkError::MissingState {
                type_name: "AppConfig",
            })
    }
}

/// Extractor for user-provided state registered via [`App::state()`](crate::App::state).
///
/// Returns `Arc<T>` so the data can be shared cheaply across handlers.
///
/// # Example
///
/// ```rust,ignore
/// use oxide_framework_core::{ApiResponse, Data};
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
    type Rejection = FrameworkError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let app_state = parts
            .extensions
            .get::<AppState>()
            .ok_or(FrameworkError::MissingState {
                type_name: "AppState",
            })?;

        app_state
            .get::<T>()
            .map(Data)
            .ok_or(FrameworkError::MissingState {
                type_name: std::any::type_name::<T>(),
            })
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
    type Rejection = FrameworkError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let app_state = parts
            .extensions
            .get::<AppState>()
            .ok_or(FrameworkError::MissingState {
                type_name: "AppState",
            })?;

        app_state
            .get::<T>()
            .map(Inject)
            .ok_or(FrameworkError::MissingState {
                type_name: std::any::type_name::<T>(),
            })
    }
}

/// Extractor for request-scoped dependencies.
///
/// If a dependency `T` was injected into the current request (e.g. via `App::scoped_state`),
/// this extractor will retrieve it. Otherwise, it fails with a 500 Internal Server Error.
pub struct Scoped<T>(pub T);

impl<S, T> axum::extract::FromRequestParts<S> for Scoped<T>
where
    S: Send + Sync,
    T: Clone + Send + Sync + 'static,
{
    type Rejection = FrameworkError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<T>()
            .cloned()
            .map(Scoped)
            .ok_or_else(|| FrameworkError::MissingState {
                type_name: std::any::type_name::<T>(),
            })
    }
}

/// Correlation/request id value extracted from request extensions.
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

impl<S: Send + Sync> FromRequestParts<S> for RequestId {
    type Rejection = FrameworkError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<RequestId>()
            .cloned()
            .ok_or(FrameworkError::MissingState {
                type_name: "RequestId",
            })
    }
}

/// JSON body extractor with `validator` integration.
///
/// Parses the JSON body into `T` and runs `T::validate()`.
pub struct Validated<T>(pub T);

impl<S, T> FromRequest<S> for Validated<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    type Rejection = FrameworkError;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::Json(value) = axum::Json::<T>::from_request(req, state)
            .await
            .map_err(|e| FrameworkError::BadRequest(format!("invalid json body: {e}")))?;

        value.validate().map_err(|e| {
            let details = serde_json::to_value(&e).ok();
            FrameworkError::Validation {
                message: "validation failed".to_string(),
                fields: details,
            }
        })?;

        Ok(Validated(value))
    }
}


