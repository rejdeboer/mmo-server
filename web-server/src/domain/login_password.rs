use argon2::{Argon2, PasswordHash, PasswordVerifier};

use super::MAX_PASSWORD_LENGTH;

#[derive(Debug)]
pub struct LoginPassword(String);

impl LoginPassword {
    pub fn parse(s: String) -> Result<LoginPassword, String> {
        let is_too_long = s.chars().count() > MAX_PASSWORD_LENGTH;

        if is_too_long {
            Err(format!("password {} is too long", s))
        } else {
            Ok(Self(s))
        }
    }

    pub fn verify(&self, hash: &PasswordHash<'_>) -> argon2::password_hash::Result<()> {
        let argon = Argon2::default();
        argon.verify_password(self.as_ref().as_bytes(), hash)
    }
}

impl AsRef<str> for LoginPassword {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::{LoginPassword, MAX_PASSWORD_LENGTH};
    use claim::{assert_err, assert_ok};

    #[test]
    fn a_32_char_long_pw_is_valid() {
        let name = "a".repeat(32);
        assert_ok!(LoginPassword::parse(name));
    }

    #[test]
    fn a_pw_longer_than_max_chars_is_rejected() {
        let name = "a".repeat(MAX_PASSWORD_LENGTH + 1);
        assert_err!(LoginPassword::parse(name));
    }
}
