//! Encode JWTs (tests, login handlers).

use jsonwebtoken::{Algorithm, EncodingKey, Header};

use super::claims::AuthClaims;

/// Encode [`AuthClaims`] as an HS256 JWT string.
pub fn encode_token(claims: &AuthClaims, secret: &[u8]) -> Result<String, jsonwebtoken::errors::Error> {
    let key = EncodingKey::from_secret(secret);
    let mut header = Header::new(Algorithm::HS256);
    header.typ = Some("JWT".into());
    jsonwebtoken::encode(&header, claims, &key)
}
