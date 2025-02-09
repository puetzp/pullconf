use super::user::Name as Username;
use crate::{Ensure, ResourceMetadata};
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::{fmt, ops::Deref, str::FromStr};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Parameters {
    pub ensure: Ensure,
    pub name: Name,
    pub system: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Relationships {
    pub requires: Vec<ResourceMetadata>,
    pub triggers: Vec<ResourceMetadata>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Name(String);

impl FromStr for Name {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            anyhow::bail!("groupname cannot be an empty string")
        }

        if s.chars().count() > 32 {
            anyhow::bail!("groupname cannot exceed 32 characters")
        }

        if s.chars()
            .next()
            .is_some_and(|c| !(c.is_ascii_alphabetic() || c == '_'))
        {
            anyhow::bail!("groupname must start with an alphabetic character or an underscore")
        }

        if let Some(ref c) = s
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_'))
        {
            anyhow::bail!("groupname contains invalid character `{}`", c)
        }

        Ok(Self(s.to_owned()))
    }
}

crate::impl_string_newtype!(Name);

impl PartialEq<Username> for Name {
    fn eq(&self, other: &Username) -> bool {
        &self.0 == other.deref()
    }
}

impl From<&Username> for Name {
    fn from(name: &Username) -> Self {
        Name(name.to_string())
    }
}
