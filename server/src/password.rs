use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::{SaltString, rand_core::OsRng},
};
use thiserror::Error;

const MIN_PASSWORD_BYTES: usize = 14;
const MAX_PASSWORD_BYTES: usize = 128;
const ARGON2_MEMORY_KIB: u32 = 19 * 1024;
const ARGON2_ITERATIONS: u32 = 2;
const ARGON2_PARALLELISM: u32 = 1;

#[derive(Debug, Error)]
pub enum PasswordError {
    #[error("password must contain between 14 and 128 bytes")]
    InvalidLength,
    #[error("password hashing failed")]
    Hash,
}

fn argon2() -> Result<Argon2<'static>, PasswordError> {
    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        None,
    )
    .map_err(|_| PasswordError::Hash)?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    if !(MIN_PASSWORD_BYTES..=MAX_PASSWORD_BYTES).contains(&password.len()) {
        return Err(PasswordError::InvalidLength);
    }
    let salt = SaltString::generate(&mut OsRng);
    argon2()?
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| PasswordError::Hash)
}

#[must_use]
pub fn verify_password(password: &str, encoded: &str) -> bool {
    if password.len() > MAX_PASSWORD_BYTES {
        return false;
    }
    let Ok(hash) = PasswordHash::new(encoded) else {
        return false;
    };
    argon2()
        .and_then(|hasher| {
            hasher
                .verify_password(password.as_bytes(), &hash)
                .map_err(|_| PasswordError::Hash)
        })
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argon2id_round_trip_and_wrong_password() {
        let encoded = hash_password("a long test password").expect("hash");
        assert!(encoded.starts_with("$argon2id$v=19$"));
        assert!(verify_password("a long test password", &encoded));
        assert!(!verify_password("not the password", &encoded));
    }

    #[test]
    fn password_bounds_are_enforced() {
        assert!(matches!(
            hash_password("too short"),
            Err(PasswordError::InvalidLength)
        ));
        assert!(matches!(
            hash_password(&"x".repeat(129)),
            Err(PasswordError::InvalidLength)
        ));
    }
}
