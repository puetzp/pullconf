use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

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

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Username(String);

impl FromStr for Username {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            anyhow::bail!("username cannot be an empty string")
        }

        if s.chars().count() > 32 {
            anyhow::bail!("username cannot exceed 32 characters")
        }

        if s.chars()
            .next()
            .is_some_and(|c| !(c.is_ascii_alphabetic() || c == '_'))
        {
            anyhow::bail!("username must start with an alphabetic character or an underscore")
        }

        if let Some(ref c) = s
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_'))
        {
            anyhow::bail!("username contains invalid character `{}`", c)
        }

        Ok(Self(s.to_owned()))
    }
}

impl<'de> Deserialize<'de> for Username {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = String::deserialize(deserializer)?;

        Username::from_str(&v).map_err(Error::custom)
    }
}

impl From<&Username> for Username {
    fn from(name: &Username) -> Self {
        name.clone()
    }
}

impl Deref for Username {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

impl PartialEq<Groupname> for Username {
    fn eq(&self, other: &Groupname) -> bool {
        self.0 == *other.0
    }
}

impl Username {
    pub fn root() -> Self {
        Self(String::from("root"))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Groupname(String);

impl FromStr for Groupname {
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

impl<'de> Deserialize<'de> for Groupname {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = String::deserialize(deserializer)?;

        Groupname::from_str(&v).map_err(Error::custom)
    }
}

impl From<&Groupname> for Groupname {
    fn from(name: &Groupname) -> Self {
        name.clone()
    }
}

impl From<&Username> for Groupname {
    fn from(name: &Username) -> Self {
        Groupname(name.to_string())
    }
}

impl Deref for Groupname {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl fmt::Display for Groupname {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

impl PartialEq<Username> for Groupname {
    fn eq(&self, other: &Username) -> bool {
        self.0 == *other.0
    }
}

impl Groupname {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
