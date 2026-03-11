#[derive(Debug)]
pub struct CharacterName(String);

impl CharacterName {
    // TODO: Verify if we use normal constraints
    pub fn parse(s: String) -> Result<CharacterName, String> {
        let is_too_long = s.chars().count() > 32;
        let is_too_short = s.chars().count() < 3;

        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}', ' '];
        let contains_forbidden_characters = s.chars().any(|g| forbidden_characters.contains(&g));

        if is_too_long || is_too_short || contains_forbidden_characters {
            Err(format!("{s} is not a valid username"))
        } else {
            Ok(Self(s.to_lowercase()))
        }
    }
}

impl AsRef<str> for CharacterName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::CharacterName;

    #[test]
    fn a_32_char_long_name_is_valid() {
        let name = "a".repeat(32);
        assert!(CharacterName::parse(name).is_ok());
    }

    #[test]
    fn a_name_longer_than_32_chars_is_rejected() {
        let name = "a".repeat(33);
        assert!(CharacterName::parse(name).is_err());
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert!(CharacterName::parse(name).is_err());
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert!(CharacterName::parse(name).is_err());
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}', ' '] {
            let name = name.to_string();
            assert!(CharacterName::parse(name).is_err());
        }
    }

    #[test]
    fn a_valid_name_is_pared_successfully() {
        let name = "rejdeboer".to_string();
        assert!(CharacterName::parse(name).is_ok());
    }
}
