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
