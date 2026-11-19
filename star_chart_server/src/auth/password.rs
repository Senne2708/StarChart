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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_password_verifies() {
        let hash = hash_password("correct horse").unwrap();
        assert!(verify_password("correct horse", &hash));
    }

    #[test]
    fn wrong_password_fails() {
        let hash = hash_password("correct horse").unwrap();
        assert!(!verify_password("wrong horse", &hash));
    }

    #[test]
    fn same_password_hashes_differently() {
        // Each hash gets a random salt
        assert_ne!(hash_password("same").unwrap(), hash_password("same").unwrap());
    }

    #[test]
    fn invalid_hash_fails_instead_of_panicking() {
        assert!(!verify_password("anything", "not a hash"));
    }
}
