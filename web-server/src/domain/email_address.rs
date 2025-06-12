use validator::ValidateEmail;

#[derive(Debug, Clone)]
pub struct EmailAddress(String);

impl EmailAddress {
    pub fn parse(s: String) -> Result<EmailAddress, String> {
        if ValidateEmail::validate_email(&s) {
            Ok(Self(s))
        } else {
            Err(format!("{} is not a valid email address", s))
        }
    }
}

impl AsRef<str> for EmailAddress {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EmailAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::EmailAddress;
    use claim::assert_err;
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let email = SafeEmail().fake_with_rng(g);
            Self(email)
        }
    }

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(EmailAddress::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "rick.deboer.com".to_string();
        assert_err!(EmailAddress::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@live.nl".to_string();
        assert_err!(EmailAddress::parse(email));
    }

    #[quickcheck_macros::quickcheck]
    fn valid_emails_are_parsed_successfully(valid_email: ValidEmailFixture) -> bool {
        EmailAddress::parse(valid_email.0).is_ok()
    }
}
