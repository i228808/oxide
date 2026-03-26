//! Tower layer: decode JWT from Bearer and/or session cookie, attach [`super::AuthClaims`].

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::http::header::{AUTHORIZATION, COOKIE};
use axum::http::{Request, StatusCode};
use axum::Json;
use axum::response::{IntoResponse, Response};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use tower::{Layer, Service};
use tracing::debug;

use super::claims::AuthClaims;
use super::config::AuthConfig;

/// Tower `Layer` that validates JWTs and inserts [`AuthClaims`] into request extensions.
#[derive(Clone)]
pub struct AuthLayer {
    config: Arc<AuthConfig>,
    key: DecodingKey,
    validation: Validation,
}

impl AuthLayer {
    pub fn new(config: AuthConfig) -> Self {
        let config = Arc::new(config);
        let key = DecodingKey::from_secret(&config.secret);
        let mut validation = Validation::new(Algorithm::HS256);
        if let Some(ref iss) = config.issuer {
            validation.set_issuer(&[iss.as_str()]);
        }
        if let Some(ref aud) = config.audience {
            validation.set_audience(&[aud.as_str()]);
        }
        Self {
            config,
            key,
            validation,
        }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,
            config: self.config.clone(),
            key: self.key.clone(),
            validation: self.validation.clone(),
        }
    }
}

/// Inner service that decodes JWT and forwards the request.
#[derive(Clone)]
pub struct AuthService<S> {
    inner: S,
    config: Arc<AuthConfig>,
    key: DecodingKey,
    validation: Validation,
}

impl<S> Service<Request<Body>> for AuthService<S>
where
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Error: Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let config = self.config.clone();
        let key = self.key.clone();
        let validation = self.validation.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let token_result = resolve_token(&config, req.headers());

            match token_result {
                TokenResolution::None => inner.call(req).await,
                TokenResolution::Some(token) => match decode::<AuthClaims>(&token, &key, &validation) {
                    Ok(data) => {
                        req.extensions_mut().insert(data.claims);
                        inner.call(req).await
                    }
                    Err(e) => {
                        debug!(error = %e, "jwt validation failed");
                        Ok(auth_error_response(
                            StatusCode::UNAUTHORIZED,
                            "invalid or expired token",
                        ))
                    }
                },
                TokenResolution::Malformed => Ok(auth_error_response(
                    StatusCode::UNAUTHORIZED,
                    "malformed authorization",
                )),
            }
        })
    }
}

enum TokenResolution {
    /// No Bearer / no session cookie — anonymous.
    None,
    /// Raw JWT string to validate.
    Some(String),
    /// Authorization header present but not usable.
    Malformed,
}

fn resolve_token(config: &AuthConfig, headers: &axum::http::HeaderMap) -> TokenResolution {
    if config.bearer_token {
        if let Some(auth) = headers.get(AUTHORIZATION) {
            match auth.to_str() {
                Ok(s) => {
                    let s = s.trim();
                    if let Some(rest) = s.strip_prefix("Bearer ").or_else(|| s.strip_prefix("bearer ")) {
                        let t = rest.trim();
                        if t.is_empty() {
                            return TokenResolution::Malformed;
                        }
                        return TokenResolution::Some(t.to_string());
                    }
                    // Authorization present but not Bearer — reject
                    return TokenResolution::Malformed;
                }
                Err(_) => return TokenResolution::Malformed,
            }
        }
    }

    if let Some(name) = config.session_cookie_name.as_deref() {
        if let Some(cookie_hdr) = headers.get(COOKIE).and_then(|v| v.to_str().ok()) {
            if let Some(tok) = cookie_token(cookie_hdr, name) {
                return TokenResolution::Some(tok);
            }
        }
    }

    TokenResolution::None
}

fn cookie_token(header: &str, name: &str) -> Option<String> {
    for c in cookie::Cookie::split_parse(header).flatten() {
        if c.name() == name {
            let v = c.value();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn auth_error_response(status: StatusCode, message: &str) -> Response {
    let body = serde_json::json!({
        "status": status.as_u16(),
        "error": message,
    });
    (status, Json(body)).into_response()
}
