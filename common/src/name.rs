use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::{fmt, ops::Deref, str::FromStr};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Hostname(String);

impl FromStr for Hostname {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            anyhow::bail!("hostname cannot be an empty string")
        }

        if s.chars().count() > 253 {
            anyhow::bail!("hostname cannot exceed 253 characters")
        }

        if s.starts_with('_') {
            anyhow::bail!("hostname cannot start with a hyphen")
        }

        if let Some(ref c) = s
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '.'))
        {
            anyhow::bail!("hostname contains invalid character `{}`", c)
        }

        if let Some(ref element) = s
            .split('.')
            .find(|element| !(1..=63).contains(&element.chars().count()))
        {
            anyhow::bail!("dot-separated parts of a hostname must be between 1 and 63 characters long, found `{}`", element)
        }

        Ok(Self(s.to_owned()))
    }
}

crate::impl_string_newtype!(Hostname);
