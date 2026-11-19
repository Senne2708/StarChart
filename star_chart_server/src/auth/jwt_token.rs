use std::time::{SystemTime, UNIX_EPOCH};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, decode, encode, Validation};
use serde::{Deserialize, Serialize};

#[derive(Serialize,Deserialize,Clone)]
pub struct Claims {
    pub sub: String,
    pub iat: usize,
    pub exp: usize
}

pub struct JwtKeys {
    encoding: EncodingKey,
    decoding: DecodingKey
}

impl JwtKeys {
    pub fn from_env() -> Self {
        match std::env::var("JWT_SECRET") {
            Ok(secret) => {
                assert!(secret.len() >= 32, "JWT_SECRET must be at least 32 characters");
                Self::from_secret(secret.as_bytes())
            }

            // Only compiled into debug builds
            #[cfg(debug_assertions)]
            Err(_) => {
                tracing::warn!("JWT_SECRET not set, using insecure debug key");
                Self::from_secret(b"debug-only-secret-never-use-in-production")
            }

            // Only compiled into release builds
            #[cfg(not(debug_assertions))]
            Err(_) => panic!("JWT_SECRET must be set in release builds"),
        }
    }

    pub fn from_secret(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}


pub fn create_jwt(keys: &JwtKeys, username: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as usize;

    let claims = Claims {
        sub: username.to_string(),
        iat: now,
        exp: now + 60*60
    };

    encode(&Header::default(), &claims, &keys.encoding)
}


pub fn verify_jwt(keys: &JwtKeys, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode(token, &keys.decoding, &Validation::default()).map(|data| data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys() -> JwtKeys {
        JwtKeys::from_secret(b"test-secret-that-is-at-least-32-chars")
    }

    #[test]
    fn token_round_trips() {
        let keys = keys();
        let token = create_jwt(&keys, "teacher1").unwrap();

        let claims = verify_jwt(&keys, &token).unwrap();
        assert_eq!(claims.sub, "teacher1");
        assert_eq!(claims.exp - claims.iat, 60 * 60);
    }

    #[test]
    fn rejects_token_signed_with_another_key() {
        let other = JwtKeys::from_secret(b"another-secret-that-is-32-chars-long");
        let token = create_jwt(&other, "teacher1").unwrap();

        assert!(verify_jwt(&keys(), &token).is_err());
    }

    #[test]
    fn rejects_expired_token() {
        let keys = keys();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as usize;

        // Expired by more than the 60 second leeway `Validation::default()` allows
        let claims = Claims {
            sub: "teacher1".to_string(),
            iat: now - 2 * 60 * 60,
            exp: now - 120,
        };
        let token = encode(&Header::default(), &claims, &keys.encoding).unwrap();

        assert!(verify_jwt(&keys, &token).is_err());
    }

    #[test]
    fn rejects_tampered_token() {
        let keys = keys();
        let real = create_jwt(&keys, "teacher1").unwrap();
        let other = create_jwt(&keys, "teacher2").unwrap();

        // The header and signature of one token, with the claims of another
        let parts: Vec<&str> = real.split('.').collect();
        let other_claims = other.split('.').nth(1).unwrap();
        let forged = format!("{}.{}.{}", parts[0], other_claims, parts[2]);

        assert!(verify_jwt(&keys, &forged).is_err());
    }
}
