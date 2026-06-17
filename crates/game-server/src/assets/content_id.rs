use serde::{Deserialize, Deserializer};
use std::fmt;
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContentId(pub u64);

impl ContentId {
    pub fn from(s: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        Self(hasher.finish())
    }
}

impl fmt::Debug for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentId({:#x})", self.0)
    }
}

impl<'de> Deserialize<'de> for ContentId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ContentKeyVisitor;

        impl<'de> serde::de::Visitor<'de> for ContentKeyVisitor {
            type Value = ContentId;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representing a content key")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(ContentId::from(value))
            }
        }

        deserializer.deserialize_str(ContentKeyVisitor)
    }
}
