use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::Value;

/// Framework-level typed error used by core extractors and middleware.
#[derive(Debug, Clone)]
pub enum FrameworkError {
    MissingState {
        type_name: &'static str,
    },
    Validation {
        message: String,
        fields: Option<Value>,
    },
    ReadinessFailed {
        check: &'static str,
        message: String,
    },
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    Timeout(String),
    Internal(String),
}

impl FrameworkError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            FrameworkError::MissingState { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            FrameworkError::Validation { .. } => StatusCode::BAD_REQUEST,
            FrameworkError::ReadinessFailed { .. } => StatusCode::SERVICE_UNAVAILABLE,
            FrameworkError::BadRequest(_) => StatusCode::BAD_REQUEST,
            FrameworkError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            FrameworkError::Forbidden(_) => StatusCode::FORBIDDEN,
            FrameworkError::Timeout(_) => StatusCode::REQUEST_TIMEOUT,
            FrameworkError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            FrameworkError::MissingState { .. } => "missing_state",
            FrameworkError::Validation { .. } => "validation_error",
            FrameworkError::ReadinessFailed { .. } => "readiness_failed",
            FrameworkError::BadRequest(_) => "bad_request",
            FrameworkError::Unauthorized(_) => "unauthorized",
            FrameworkError::Forbidden(_) => "forbidden",
            FrameworkError::Timeout(_) => "request_timeout",
            FrameworkError::Internal(_) => "internal_error",
        }
    }

    pub fn message(&self) -> String {
        match self {
            FrameworkError::MissingState { type_name } => {
                format!("internal error: missing state ({type_name})")
            }
            FrameworkError::Validation { message, .. } => message.clone(),
            FrameworkError::ReadinessFailed { check, message } => {
                format!("readiness check `{check}` failed: {message}")
            }
            FrameworkError::BadRequest(message)
            | FrameworkError::Unauthorized(message)
            | FrameworkError::Forbidden(message)
            | FrameworkError::Timeout(message)
            | FrameworkError::Internal(message) => message.clone(),
        }
    }
}

impl std::fmt::Display for FrameworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for FrameworkError {}

impl IntoResponse for FrameworkError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let mut body = serde_json::json!({
            "status": status.as_u16(),
            "error": self.message(),
            "code": self.code(),
        });

        if let FrameworkError::Validation {
            fields: Some(fields),
            ..
        } = self
        {
            body["details"] = fields;
        }

        (status, axum::Json(body)).into_response()
    }
}
