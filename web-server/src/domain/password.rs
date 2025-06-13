use argon2::{
    password_hash::{rand_core::OsRng, PasswordHashString, PasswordHasher, SaltString},
    Argon2,
};

pub const MAX_PASSWORD_LENGTH: usize = 64;

#[derive(Debug)]
pub struct Password(String);

impl Password {
    // TODO: Verify if we use normal constraints
    pub fn parse(s: String) -> Result<Password, String> {
        let is_empty_or_whitespace = s.trim().is_empty();
        let is_too_long = s.chars().count() > MAX_PASSWORD_LENGTH;
        let is_too_short = s.chars().count() < 8;

        let forbidden_characters = ['(', ')', '"', '<', '>', '\\', '{', '}', ' '];
        let contains_forbidden_characters = s.chars().any(|g| forbidden_characters.contains(&g));

        if is_empty_or_whitespace || is_too_long || is_too_short || contains_forbidden_characters {
            Err(format!("{} is not a valid password", s))
        } else {
            Ok(Self(s))
        }
    }

    pub fn hash(self) -> Result<PasswordHashString, argon2::password_hash::Error> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let passhash = argon2
            .hash_password(self.as_ref().as_bytes(), &salt)?
            .serialize();
        Ok(passhash)
    }
}

impl AsRef<str> for Password {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::{Password, MAX_PASSWORD_LENGTH};
    use claim::{assert_err, assert_ok};

    #[test]
    fn a_32_char_long_pw_is_valid() {
        let name = "a".repeat(32);
        assert_ok!(Password::parse(name));
    }

    #[test]
    fn a_pw_longer_than_max_chars_is_rejected() {
        let name = "a".repeat(MAX_PASSWORD_LENGTH + 1);
        assert_err!(Password::parse(name));
    }

    #[test]
    fn whitespace_only_pws_are_rejected() {
        let name = " ".to_string();
        assert_err!(Password::parse(name));
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(Password::parse(name));
    }

    #[test]
    fn pws_containing_an_invalid_character_are_rejected() {
        for name in &['(', ')', '"', '<', '>', '\\', '{', '}', ' '] {
            let name = name.to_string();
            assert_err!(Password::parse(name));
        }
    }

    #[test]
    fn a_valid_pw_is_pared_successfully() {
        let name = "superSecret@".to_string();
        assert_ok!(Password::parse(name));
    }
}
