//! Configuration for JWT / cookie authentication.

/// How [`super::AuthLayer`] obtains and validates tokens.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// HMAC secret for HS256.
    pub secret: Vec<u8>,
    /// If set, `iss` must match.
    pub issuer: Option<String>,
    /// If set, `aud` must match (single audience string).
    pub audience: Option<String>,
    /// Accept `Authorization: Bearer <jwt>`.
    pub bearer_token: bool,
    /// If set, also read a JWT from this cookie name (session-style).
    pub session_cookie_name: Option<String>,
}

impl AuthConfig {
    /// HS256 with Bearer tokens only.
    pub fn new(secret: impl Into<Vec<u8>>) -> Self {
        Self {
            secret: secret.into(),
            issuer: None,
            audience: None,
            bearer_token: true,
            session_cookie_name: None,
        }
    }

    /// Also accept JWT stored in a browser cookie (common for session UX).
    pub fn with_session_cookie(mut self, cookie_name: impl Into<String>) -> Self {
        self.session_cookie_name = Some(cookie_name.into());
        self
    }

    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }

    /// Enable or disable the `Authorization: Bearer` scheme (cookie-only when `false`).
    pub fn enable_bearer(mut self, yes: bool) -> Self {
        self.bearer_token = yes;
        self
    }
}
