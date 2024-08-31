use super::super::user::Name as Username;
use crate::{Ensure, ResourceMetadata};
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::{fmt, ops::Deref, path::PathBuf, str::FromStr};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Parameters {
    pub ensure: Ensure,
    pub target: PathBuf,
    pub name: Name,
    pub schedule: String,
    pub user: Username,
    pub command: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Relationships {
    pub requires: Vec<ResourceMetadata>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Name(String);

impl FromStr for Name {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            anyhow::bail!("cron job name cannot be an empty string")
        }

        if let Some(ref c) = s
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_'))
        {
            anyhow::bail!("cron job name contains invalid character `{}`", c)
        }

        Ok(Self(s.to_owned()))
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = String::deserialize(deserializer)?;

        Name::from_str(&v).map_err(Error::custom)
    }
}

impl From<&Name> for Name {
    fn from(name: &Name) -> Self {
        name.clone()
    }
}

impl From<&Username> for Name {
    fn from(name: &Username) -> Self {
        Name(name.to_string())
    }
}

impl Deref for Name {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*self.0, f)
    }
}

impl PartialEq<Username> for Name {
    fn eq(&self, other: &Username) -> bool {
        &self.0 == other.deref()
    }
}

impl Name {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
