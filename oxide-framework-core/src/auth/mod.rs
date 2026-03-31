//! JWT authentication and role guards.
//!
//! 1. Configure [`AuthConfig`] and register with [`crate::App::auth`].
//! 2. Use [`Authenticated`], [`OptionalAuth`], or [`RequireRole`] in handlers.
//!
//! See [`crate::auth::token::encode_token`] to mint JWTs in login handlers or tests.

mod claims;
mod config;
mod extract;
mod layer;
pub mod token;

pub use claims::AuthClaims;
pub use config::AuthConfig;
pub use extract::{AuthRejection, Authenticated, OptionalAuth, RequireRole, RoleName};
pub use layer::AuthLayer;
pub use token::encode_token;

