use super::group::Name as Groupname;
use super::user::Name as Username;
use crate::{Ensure, ResourceMetadata, SafePathBuf};
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::{fmt, ops::Deref, str::FromStr};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Parameters {
    pub path: SafePathBuf,
    pub ensure: Ensure,
    pub mode: Mode,
    pub owner: Username,
    pub group: Option<Groupname>,
    pub content: Option<String>,
    pub source: Option<SafePathBuf>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Relationships {
    pub requires: Vec<ResourceMetadata>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct Mode(String);

crate::impl_string_newtype!(Mode);

impl FromStr for Mode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !(3..=4).contains(&s.len()) || s.chars().any(|c| !('0'..='7').contains(&c)) {
            anyhow::bail!("value is not a valid file mode");
        }

        Ok(Self(s.to_string()))
    }
}

impl Default for Mode {
    fn default() -> Self {
        Self("644".to_string())
    }
}
