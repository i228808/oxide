use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// Standardized success envelope returned by all handlers that use [`ApiResponse`].
///
/// ```json
/// { "status": 200, "data": { ... } }
/// ```
#[derive(Debug, Serialize)]
pub struct SuccessBody<T: Serialize> {
    pub status: u16,
    pub data: T,
}

/// Standardized error envelope.
///
/// ```json
/// { "status": 404, "error": "not found" }
/// ```
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub status: u16,
    pub error: String,
}

/// A unified response type that handlers can return.
///
/// Converts into a properly-typed axum `Response` with the correct status code
/// and a JSON body using either [`SuccessBody`] or [`ErrorBody`].
///
/// # Usage
///
/// ```rust,ignore
/// async fn get_user() -> ApiResponse<User> {
///     let user = User { name: "Alice".into() };
///     ApiResponse::ok(user)
/// }
///
/// async fn not_found() -> ApiResponse<()> {
///     ApiResponse::error(StatusCode::NOT_FOUND, "resource not found")
/// }
/// ```
pub enum ApiResponse<T: Serialize> {
    Success(StatusCode, T),
    Error(StatusCode, String),
}

impl<T: Serialize> ApiResponse<T> {
    /// 200 OK with a data payload.
    pub fn ok(data: T) -> Self {
        Self::Success(StatusCode::OK, data)
    }

    /// 201 Created with a data payload.
    pub fn created(data: T) -> Self {
        Self::Success(StatusCode::CREATED, data)
    }

    /// Arbitrary success status with a data payload.
    pub fn success(status: StatusCode, data: T) -> Self {
        Self::Success(status, data)
    }

    /// Error response with a status code and message.
    pub fn error(status: StatusCode, message: impl Into<String>) -> Self {
        Self::Error(status, message.into())
    }

    /// 400 Bad Request.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::error(StatusCode::BAD_REQUEST, message)
    }

    /// 404 Not Found.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::error(StatusCode::NOT_FOUND, message)
    }

    /// 401 Unauthorized.
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::error(StatusCode::UNAUTHORIZED, message)
    }

    /// 403 Forbidden.
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::error(StatusCode::FORBIDDEN, message)
    }

    /// 500 Internal Server Error.
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::error(StatusCode::INTERNAL_SERVER_ERROR, message)
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        match self {
            ApiResponse::Success(status, data) => {
                let body = SuccessBody {
                    status: status.as_u16(),
                    data,
                };
                (status, Json(body)).into_response()
            }
            ApiResponse::Error(status, message) => {
                let body = ErrorBody {
                    status: status.as_u16(),
                    error: message,
                };
                (status, Json(body)).into_response()
            }
        }
    }
}
