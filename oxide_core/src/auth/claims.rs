//! JWT payload claims inserted into request extensions after successful auth.

use serde::{Deserialize, Serialize};

/// Standard claims decoded from a JWT (Bearer or session cookie).
///
/// Include `roles` as a JSON array of strings in the token payload, for example:
/// `{"sub":"user-1","roles":["admin","user"],"exp":...}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthClaims {
    /// Subject (typically user id).
    pub sub: String,
    /// Role names for [`crate::auth::RequireRole`] and [`AuthClaims::has_role`].
    #[serde(default)]
    pub roles: Vec<String>,
    /// Expiration time (Unix seconds). Required for validation.
    pub exp: u64,
    /// Issuer — include when validating with [`crate::auth::AuthConfig::with_issuer`].
    #[serde(default)]
    pub iss: Option<String>,
    /// Audience — include when validating with [`crate::auth::AuthConfig::with_audience`].
    #[serde(default)]
    pub aud: Option<String>,
}

impl AuthClaims {
    /// Build claims with `exp` set to now + `ttl_secs`.
    pub fn new(sub: impl Into<String>, roles: Vec<String>, ttl_secs: u64) -> Self {
        let exp = jsonwebtoken::get_current_timestamp().saturating_add(ttl_secs);
        Self {
            sub: sub.into(),
            roles,
            exp,
            iss: None,
            aud: None,
        }
    }

    /// Returns true if `role` is present in [`Self::roles`].
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Returns true if the principal has **any** of the given roles.
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|r| self.has_role(r))
    }

    /// Returns true if the principal has **all** of the given roles.
    pub fn has_all_roles(&self, roles: &[&str]) -> bool {
        roles.iter().all(|r| self.has_role(r))
    }
}
