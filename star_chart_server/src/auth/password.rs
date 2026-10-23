use argon2::{
    password_hash::{phc::PasswordHash, Error, PasswordHasher, PasswordVerifier},
    Argon2,
};

pub fn hash_password(password: &str) -> Result<String, Error> {
    Ok(Argon2::default().hash_password(password.as_bytes())?.to_string())
}

pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    match PasswordHash::new(stored_hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}
