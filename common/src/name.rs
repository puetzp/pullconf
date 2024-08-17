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

impl<'de> Deserialize<'de> for Hostname {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = String::deserialize(deserializer)?;

        Hostname::from_str(&v).map_err(Error::custom)
    }
}

impl From<&Hostname> for Hostname {
    fn from(name: &Hostname) -> Self {
        name.clone()
    }
}

impl Deref for Hostname {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl fmt::Display for Hostname {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

impl Hostname {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
