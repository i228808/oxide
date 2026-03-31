//! Extractors: [`Authenticated`], [`OptionalAuth`], [`RequireRole`].

use std::marker::PhantomData;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};

use super::claims::AuthClaims;
use crate::response::ApiResponse;

/// Requires a valid JWT (middleware must run — use [`crate::App::auth`]).
pub struct Authenticated(pub AuthClaims);

impl<S: Send + Sync> FromRequestParts<S> for Authenticated {
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthClaims>()
            .cloned()
            .map(Authenticated)
            .ok_or(AuthRejection::Unauthorized)
    }
}

/// Present when the client sent a valid JWT; [`None`] for anonymous requests.
pub struct OptionalAuth(pub Option<AuthClaims>);

impl<S: Send + Sync> FromRequestParts<S> for OptionalAuth {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(OptionalAuth(parts.extensions.get::<AuthClaims>().cloned()))
    }
}

/// Map a zero-sized type to a role name (see [`RequireRole`]).
///
/// ```rust,ignore
/// struct Admin;
/// impl RoleName for Admin {
///     const ROLE: &'static str = "admin";
/// }
///
/// async fn admin_only(_: RequireRole<Admin>) -> ApiResponse<()> {
///     ApiResponse::ok(())
/// }
/// ```
pub trait RoleName: Send + Sync + 'static {
    const ROLE: &'static str;
}

/// Role guard: `RequireRole<YourRoleMarker>` where `YourRoleMarker: RoleName`.
#[derive(Debug, Clone, Copy)]
pub struct RequireRole<R: RoleName>(PhantomData<R>);

impl<S: Send + Sync, R: RoleName> FromRequestParts<S> for RequireRole<R> {
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = parts
            .extensions
            .get::<AuthClaims>()
            .ok_or(AuthRejection::Unauthorized)?;
        if claims.has_role(R::ROLE) {
            Ok(RequireRole(PhantomData))
        } else {
            Err(AuthRejection::Forbidden)
        }
    }
}

/// Rejection for auth extractors.
#[derive(Debug)]
pub enum AuthRejection {
    Unauthorized,
    Forbidden,
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        match self {
            AuthRejection::Unauthorized => {
                ApiResponse::<serde_json::Value>::unauthorized("authentication required").into_response()
            }
            AuthRejection::Forbidden => {
                ApiResponse::<serde_json::Value>::forbidden("insufficient permissions").into_response()
            }
        }
    }
}

