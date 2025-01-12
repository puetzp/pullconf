use super::super::user::Name as Username;
use crate::{Ensure, ResourceMetadata};
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::{fmt, ops::Deref, path::PathBuf, str::FromStr};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Environment {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Parameters {
    pub ensure: Ensure,
    pub target: PathBuf,
    pub name: Name,
    pub environment: Vec<Environment>,
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

crate::impl_string_newtype!(Name);

impl PartialEq<Username> for Name {
    fn eq(&self, other: &Username) -> bool {
        &self.0 == other.deref()
    }
}
